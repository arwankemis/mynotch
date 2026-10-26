//! Native Windows media sessions. No Spotify Web API, network calls, or playback engine.
use std::{
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}, mpsc::{self, Sender, Receiver}},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows::{Foundation::TypedEventHandler, Media::Control::*, Storage::Streams::DataReader};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaSnapshot {
    pub title: String, pub artist: String, pub album: String,
    pub source_app: String,
    pub playing: bool,
    pub position_seconds: f64, pub duration_seconds: f64,
    pub can_seek: bool, pub can_next: bool, pub can_previous: bool,pub can_toggle:bool,
    pub artwork: Vec<u8>,
}
#[derive(Clone, Debug)]
pub enum MediaEvent { Changed(MediaSnapshot) }

struct State { value: MediaSnapshot, stamp: Instant, rate: f64 }
impl Default for State {
    fn default() -> Self { Self { value: MediaSnapshot::default(), stamp: Instant::now(), rate: 1.0 } }
}
impl State {
    fn position(&self) -> f64 {
        let mut position = self.value.position_seconds;
        if self.value.playing { position += self.stamp.elapsed().as_secs_f64() * self.rate; }
        clamp_position(position, self.value.duration_seconds)
    }
    fn snapshot(&self) -> MediaSnapshot {
        let mut value = self.value.clone();
        value.position_seconds = self.position();
        value
    }
}
fn clamp_position(value: f64, duration: f64) -> f64 {
    if !value.is_finite() { return 0.0; }
    if duration > 0.0 { value.clamp(0.0, duration) } else { value.max(0.0) }
}

enum Command {
    Refresh { choose: bool, metadata: bool },
    Toggle(Sender<Result<(), String>>), Next(Sender<Result<(), String>>),
    Previous(Sender<Result<(), String>>), Seek(f64, Sender<Result<(), String>>), Stop,
}

pub struct MediaService {
    commands: Sender<Command>, shared: Arc<Mutex<State>>, cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>, done: Receiver<()>,
}
impl MediaService {
    /// Acquires the Windows session manager on a dedicated MTA worker. Does not start playback.
    pub fn start(tx: Sender<MediaEvent>) -> Result<Self, String> {
        let (commands, receiver) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (done_tx, done) = mpsc::channel();
        let shared = Arc::new(Mutex::new(State::default()));
        let cancelled = Arc::new(AtomicBool::new(false));
        let state = shared.clone(); let cancel = cancelled.clone(); let events = commands.clone();
        let worker = thread::Builder::new().name("notch-media".into()).spawn(move || {
            let result = media_worker(receiver, events, tx, state, cancel, ready_tx.clone());
            if let Err(error) = result { let _ = ready_tx.send(Err(error)); }
            let _ = done_tx.send(());
        }).map_err(|e| e.to_string())?;
        match ready_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(())) => Ok(Self { commands, shared, cancelled, worker: Some(worker), done }),
            result => {
                cancelled.store(true, Ordering::Release); let _ = commands.send(Command::Stop);
                if worker.is_finished() { let _ = worker.join(); }
                Err(match result { Ok(Err(e)) => e, _ => "Windows media initialization timed out".into() })
            }
        }
    }
    pub fn snapshot(&self) -> MediaSnapshot { self.shared.lock().map(|s| s.snapshot()).unwrap_or_default() }
    /// Cheap timeline getter for lyric/UI ticks; avoids cloning artwork on every frame.
    pub fn position_seconds(&self) -> f64 { self.shared.lock().map(|s| s.position()).unwrap_or_default() }
    pub fn toggle_playback(&self) -> Result<(), String> { self.command(Command::Toggle) }
    pub fn next(&self) -> Result<(), String> { self.command(Command::Next) }
    pub fn previous(&self) -> Result<(), String> { self.command(Command::Previous) }
    pub fn seek(&self, pos_seconds: f64) -> Result<(), String> {
        if !pos_seconds.is_finite() || pos_seconds < 0.0 { return Err("Invalid playback position".into()); }
        self.command(|reply| Command::Seek(pos_seconds, reply))
    }
    fn command(&self, make: impl FnOnce(Sender<Result<(), String>>) -> Command) -> Result<(), String> {
        if self.cancelled.load(Ordering::Acquire) { return Err("Media is stopped".into()); }
        let (tx, rx) = mpsc::channel();
        self.commands.send(make(tx)).map_err(|_| "Media worker unavailable".to_string())?;
        rx.recv_timeout(Duration::from_millis(750)).map_err(|_| "Media command result unknown (timeout); do not retry automatically".to_string())?
    }
    pub fn stop(&mut self) {
        self.cancelled.store(true, Ordering::Release); let _ = self.commands.send(Command::Stop);
        // A stalled WinRT provider must not hang the UI indefinitely. Late state updates are cancelled.
        let _ = self.done.recv_timeout(Duration::from_millis(200));
        if let Some(worker) = self.worker.take() { if worker.is_finished() { let _ = worker.join(); } }
        if let Ok(mut state) = self.shared.lock() { *state = State::default(); }
    }
}
impl Drop for MediaService { fn drop(&mut self) { self.stop(); } }

#[link(name = "api-ms-win-core-winrt-l1-1-0.dll", kind = "raw-dylib", modifiers = "+verbatim")]
unsafe extern "system" { fn RoInitialize(kind: u32) -> i32; fn RoUninitialize(); }
struct Runtime;
impl Runtime {
    fn new() -> Result<Self, String> {
        let hr = unsafe { RoInitialize(1) };
        if hr < 0 { Err(format!("RoInitialize failed: {hr:#x}")) } else { Ok(Self) }
    }
}
impl Drop for Runtime { fn drop(&mut self) { unsafe { RoUninitialize(); } } }

struct ManagerEvents { manager: GlobalSystemMediaTransportControlsSessionManager, current: i64, sessions: i64 }
impl Drop for ManagerEvents {
    fn drop(&mut self) { let _ = self.manager.RemoveCurrentSessionChanged(self.current); let _ = self.manager.RemoveSessionsChanged(self.sessions); }
}
struct SessionEvents { session: GlobalSystemMediaTransportControlsSession, metadata: i64, playback: i64, timeline: i64 }
impl Drop for SessionEvents {
    fn drop(&mut self) {
        let _ = self.session.RemoveMediaPropertiesChanged(self.metadata);
        let _ = self.session.RemovePlaybackInfoChanged(self.playback);
        let _ = self.session.RemoveTimelinePropertiesChanged(self.timeline);
    }
}
fn watch_session(session: GlobalSystemMediaTransportControlsSession, tx: &Sender<Command>) -> windows::core::Result<SessionEvents> {
    let events = tx.clone();
    let metadata = session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { choose: false, metadata: true }); Ok(())
    }))?;
    let events = tx.clone();
    let playback = match session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { choose: false, metadata: false }); Ok(())
    })) { Ok(token) => token, Err(error) => { let _ = session.RemoveMediaPropertiesChanged(metadata); return Err(error); } };
    let events = tx.clone();
    let timeline = match session.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { choose: false, metadata: false }); Ok(())
    })) {
        Ok(token) => token,
        Err(error) => { let _ = session.RemoveMediaPropertiesChanged(metadata); let _ = session.RemovePlaybackInfoChanged(playback); return Err(error); }
    };
    Ok(SessionEvents { session, metadata, playback, timeline })
}

fn media_worker(rx: Receiver<Command>, commands: Sender<Command>, tx: Sender<MediaEvent>,
    shared: Arc<Mutex<State>>, cancelled: Arc<AtomicBool>, ready: Sender<Result<(), String>>) -> Result<(), String> {
    let _runtime = Runtime::new()?;
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().and_then(|op| op.join()).map_err(|e| e.to_string())?;
    let events = commands.clone();
    let current = manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { choose: true, metadata: true }); Ok(())
    })).map_err(|e| e.to_string())?;
    let events = commands.clone();
    let sessions = match manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { choose: true, metadata: true }); Ok(())
    })) {
        Ok(token) => token,
        Err(error) => { let _ = manager.RemoveCurrentSessionChanged(current); return Err(error.to_string()); }
    };
    let _manager_events = ManagerEvents { manager: manager.clone(), current, sessions };
    let mut selected: Option<SessionEvents> = None;
    let _ = ready.send(Ok(()));
    let _ = commands.send(Command::Refresh { choose: true, metadata: true });
    while !cancelled.load(Ordering::Acquire) {
        let command = match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(command) => command,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        let (reply, result) = match command {
            Command::Stop => break,
            Command::Refresh { choose, mut metadata } => {
                if choose {
                    let latest = manager.GetCurrentSession().ok();
                    if selected.as_ref().map(|s| &s.session) != latest.as_ref() {
                        selected = None;
                        if let Ok(mut state) = shared.lock() { *state = State::default(); }
                        if let Some(session) = latest { selected = watch_session(session, &commands).ok(); }
                        metadata = true;
                    }
                }
                if let Some(ref session) = selected {
                    if let Ok(state) = read_state(&session.session, metadata, &shared) {
                        if !cancelled.load(Ordering::Acquire) {
                            if let Ok(mut current) = shared.lock() { *current = state; let _ = tx.send(MediaEvent::Changed(current.snapshot())); }
                        }
                    }
                } else if !cancelled.load(Ordering::Acquire) { let _ = tx.send(MediaEvent::Changed(MediaSnapshot::default())); }
                continue;
            }
            Command::Toggle(reply) => (reply, with_session(&selected, |s| {let info=s.GetPlaybackInfo()?;let controls=info.Controls()?;
                if controls.IsPlayPauseToggleEnabled()?{s.TryTogglePlayPauseAsync()?.join()}
                else if info.PlaybackStatus()?==GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing&&controls.IsPauseEnabled()?{s.TryPauseAsync()?.join()}
                else if controls.IsPlayEnabled()?{s.TryPlayAsync()?.join()}else{Ok(false)}
            })),
            Command::Next(reply) => (reply, with_session(&selected, |s| {
                if !s.GetPlaybackInfo()?.Controls()?.IsNextEnabled()? { return Ok(false); } s.TrySkipNextAsync()?.join()
            })),
            Command::Previous(reply) => (reply, with_session(&selected, |s| {
                if !s.GetPlaybackInfo()?.Controls()?.IsPreviousEnabled()? { return Ok(false); } s.TrySkipPreviousAsync()?.join()
            })),
            Command::Seek(seconds, reply) => (reply, with_session(&selected, |s| {
                if !s.GetPlaybackInfo()?.Controls()?.IsPlaybackPositionEnabled()? { return Ok(false); }
                let timeline = s.GetTimelineProperties()?;
                let min = timeline.MinSeekTime()?.Duration;
                let provided_max = timeline.MaxSeekTime()?.Duration;
                let max = if provided_max > min { provided_max } else { timeline.EndTime()?.Duration.max(min) };
                let absolute = (timeline.StartTime()?.Duration as f64 + seconds * 10_000_000.0).clamp(min as f64, max as f64) as i64;
                s.TryChangePlaybackPositionAsync(absolute)?.join()
            })),
        };
        let _ = reply.send(result);
        let _ = commands.send(Command::Refresh { choose: false, metadata: false });
    }
    drop(selected); // remove session event tokens before uninitializing WinRT
    Ok(())
}
fn with_session(selected: &Option<SessionEvents>, action: impl FnOnce(&GlobalSystemMediaTransportControlsSession) -> windows::core::Result<bool>) -> Result<(), String> {
    let session = selected.as_ref().ok_or_else(|| "No Windows media session".to_string())?;
    match action(&session.session) { Ok(true) => Ok(()), Ok(false) => Err("Media player rejected/does not support this command".into()), Err(e) => Err(e.to_string()) }
}
fn read_state(session: &GlobalSystemMediaTransportControlsSession, metadata: bool, shared: &Mutex<State>) -> windows::core::Result<State> {
    let playback = session.GetPlaybackInfo()?; let timeline = session.GetTimelineProperties()?;
    let controls = playback.Controls()?;
    let mut value = shared.lock().map(|s| s.value.clone()).unwrap_or_default();
    value.source_app = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
    value.playing = playback.PlaybackStatus()? == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
    let rate = playback.PlaybackRate().and_then(|r| r.Value()).unwrap_or(1.0);
    let rate = if rate.is_finite() && (0.0..=16.0).contains(&rate) { rate } else { 1.0 };
    let start = timeline.StartTime()?.Duration;
    value.duration_seconds = (timeline.EndTime()?.Duration.saturating_sub(start) as f64 / 10_000_000.0).max(0.0);
    value.position_seconds = timeline.Position()?.Duration.saturating_sub(start) as f64 / 10_000_000.0;
    let now_ticks = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64() * 10_000_000.0 + 116_444_736_000_000_000.0;
    let age = (now_ticks - timeline.LastUpdatedTime()?.UniversalTime as f64) / 10_000_000.0;
    if value.playing && (0.0..86_400.0).contains(&age) { value.position_seconds += age * rate; }
    value.position_seconds = clamp_position(value.position_seconds, value.duration_seconds);
    value.can_seek = controls.IsPlaybackPositionEnabled()? && value.duration_seconds > 0.0;
    value.can_toggle=controls.IsPlayPauseToggleEnabled()?||if value.playing{controls.IsPauseEnabled()?}else{controls.IsPlayEnabled()?};
    value.can_next = controls.IsNextEnabled()?; value.can_previous = controls.IsPreviousEnabled()?;
    let stamp = Instant::now();
    if metadata {
        // Do not retain an old song/cover if the player withdraws metadata during a transition.
        value.title.clear(); value.artist.clear(); value.album.clear(); value.artwork.clear();
        if let Ok(properties) = session.TryGetMediaPropertiesAsync().and_then(|op| op.join()) {
            value.title = properties.Title().unwrap_or_default().to_string();
            value.artist = properties.Artist().unwrap_or_default().to_string();
            value.album = properties.AlbumTitle().unwrap_or_default().to_string();
            value.artwork = read_artwork(&properties).unwrap_or_default();
        }
    }
    Ok(State { value, stamp, rate })
}
fn read_artwork(properties: &GlobalSystemMediaTransportControlsSessionMediaProperties) -> windows::core::Result<Vec<u8>> {
    let stream = properties.Thumbnail()?.OpenReadAsync()?.join()?;
    let size = stream.Size()?;
    if size == 0 || size > 4 * 1024 * 1024 { let _ = stream.Close(); return Ok(Vec::new()); }
    let input = stream.GetInputStreamAt(0)?;
    let reader = DataReader::CreateDataReader(&input)?;
    let result = (|| {
        let loaded = reader.LoadAsync(size as u32)?.join()?;
        let mut bytes = vec![0; loaded as usize]; reader.ReadBytes(&mut bytes)?; Ok(bytes)
    })();
    let _ = reader.Close(); let _ = stream.Close(); result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn positions_are_finite_and_bounded() {
        assert_eq!(clamp_position(f64::NAN, 10.0), 0.0);
        assert_eq!(clamp_position(-1.0, 10.0), 0.0);
        assert_eq!(clamp_position(20.0, 10.0), 10.0);
        assert_eq!(clamp_position(20.0, 0.0), 20.0);
    }
    #[test] fn paused_snapshot_does_not_advance() {
        let mut state = State::default(); state.value.position_seconds = 4.0;
        state.stamp = Instant::now() - Duration::from_secs(2);
        assert_eq!(state.snapshot().position_seconds, 4.0);
        state.value.playing = true; state.value.duration_seconds = 5.0;
        assert_eq!(state.snapshot().position_seconds, 5.0);
    }
}
