//! Native Windows media sessions. No Spotify Web API, network calls, or playback engine.
use std::{
    sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}, mpsc::{self, Sender, Receiver}},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows::{Foundation::TypedEventHandler, Media::Control::*, Storage::Streams::DataReader};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaSnapshot {
    pub session_id: String, pub selection_revision: u64,
    pub title: String, pub artist: String, pub album: String,
    pub source_app: String,
    pub playing: bool,
    pub position_seconds: f64, pub duration_seconds: f64,
    pub can_seek: bool, pub can_next: bool, pub can_previous: bool,pub can_toggle:bool,
    pub artwork: Vec<u8>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaSession {
    pub id: String, pub source_app: String, pub playing: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaSessions {
    pub sessions: Vec<MediaSession>, pub manual: Option<String>, pub selection_revision: u64,
}
#[derive(Clone, Debug)]
pub enum MediaEvent { Changed(MediaSnapshot), SessionsChanged(MediaSessions) }

// Auto follows the Windows current-session policy first, then falls back to an
// actually playing source. Windows frequently reports no current session (or a
// paused one) while Spotify/Chrome is audibly playing, which used to make Auto
// show "No music playing" even though a source was active.
// A paused manual session remains selected until the session actually disappears.
fn choose_session(manual: &mut Option<String>, sessions: &[MediaSession], current: Option<&str>) -> Option<String> {
    if manual.as_ref().is_some_and(|id| !sessions.iter().any(|s| &s.id == id)) { *manual = None; }
    if let Some(manual_id) = manual.clone() { return Some(manual_id); }
    if let Some(current_id) = current.filter(|id| sessions.iter().any(|s| s.id == *id)) { return Some(current_id.to_owned()); }
    sessions.iter().find(|s| s.playing).map(|s| s.id.clone())
}

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
    SelectSession(Option<String>, u64),
    Refresh { metadata: bool },
    Toggle(Sender<Result<(), String>>), Next(Sender<Result<(), String>>),
    Previous(Sender<Result<(), String>>), Seek(f64, Sender<Result<(), String>>), Stop,
}

pub struct MediaService {
    commands: Sender<Command>, selection_revision: AtomicU64, shared: Arc<Mutex<State>>, cancelled: Arc<AtomicBool>,
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
            Ok(Ok(())) => Ok(Self { commands, selection_revision: AtomicU64::new(0), shared, cancelled, worker: Some(worker), done }),
            result => {
                cancelled.store(true, Ordering::Release); let _ = commands.send(Command::Stop);
                if worker.is_finished() { let _ = worker.join(); }
                Err(match result { Ok(Err(e)) => e, _ => "Inisialisasi sesi media Windows melewati batas waktu.".into() })
            }
        }
    }
    #[allow(dead_code)] pub fn snapshot(&self) -> MediaSnapshot { self.shared.lock().map(|s| s.snapshot()).unwrap_or_default() }
    /// Cheap timeline getter for lyric/UI ticks; avoids cloning artwork on every frame.
    pub fn position_seconds(&self) -> f64 { self.shared.lock().map(|s| s.position()).unwrap_or_default() }
    pub fn toggle_playback(&self) -> Result<(), String> { self.command(Command::Toggle) }
    pub fn next(&self) -> Result<(), String> { self.command(Command::Next) }
    pub fn previous(&self) -> Result<(), String> { self.command(Command::Previous) }
    /// Queues selection without blocking the UI on a WinRT provider.
    pub fn select_session(&self, id: Option<String>) -> Result<u64, String> {
        let revision = self.selection_revision.fetch_add(1, Ordering::Relaxed) + 1;
        self.commands.send(Command::SelectSession(id, revision)).map_err(|_| "Worker media sedang tidak tersedia.".to_string())?;
        Ok(revision)
    }
    pub fn seek(&self, pos_seconds: f64) -> Result<(), String> {
        if !pos_seconds.is_finite() || pos_seconds < 0.0 { return Err("Posisi pemutaran tidak valid.".into()); }
        self.command(|reply| Command::Seek(pos_seconds, reply))
    }
    fn command(&self, make: impl FnOnce(Sender<Result<(), String>>) -> Command) -> Result<(), String> {
        if self.cancelled.load(Ordering::Acquire) { return Err("Layanan media sedang dihentikan.".into()); }
        let (tx, rx) = mpsc::channel();
        self.commands.send(make(tx)).map_err(|_| "Worker media sedang tidak tersedia.".to_string())?;
        rx.recv_timeout(Duration::from_millis(750)).map_err(|_| "Perintah media melewati batas waktu.".to_string())?
    }
    /// Stops the worker. Returns true when the worker was joined cleanly, false when it
    /// was still running after a bounded 2 s grace period and had to be detached (its
    /// final state update is dropped because `cancelled` is set). The worker loop wakes
    /// on Stop immediately, so a healthy worker exits well under the grace period; the
    /// bound exists only so a stalled WinRT provider cannot hang shutdown forever.
    /// Callers that need the distinction check the return value; Drop ignores it.
    pub fn stop(&mut self) -> bool {
        self.cancelled.store(true, Ordering::Release); let _ = self.commands.send(Command::Stop);
        let completed = self.done.recv_timeout(Duration::from_secs(2)).is_ok();
        let joined = match self.worker.take() {
            None => true,
            Some(worker) => {
                // done is sent only after media_worker has released WinRT and all
                // subscriptions. is_finished may lag that signal by a scheduler tick.
                if completed || worker.is_finished() { let _ = worker.join(); true } else { false }
            }
        };
        if let Ok(mut state) = self.shared.lock() { *state = State::default(); }
        joined
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
        let _ = events.send(Command::Refresh { metadata: true }); Ok(())
    }))?;
    let events = tx.clone();
    let playback = match session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { metadata: false }); Ok(())
    })) { Ok(token) => token, Err(error) => { let _ = session.RemoveMediaPropertiesChanged(metadata); return Err(error); } };
    let events = tx.clone();
    let timeline = match session.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { metadata: false }); Ok(())
    })) {
        Ok(token) => token,
        Err(error) => { let _ = session.RemoveMediaPropertiesChanged(metadata); let _ = session.RemovePlaybackInfoChanged(playback); return Err(error); }
    };
    Ok(SessionEvents { session, metadata, playback, timeline })
}

struct AvailableSession {
    id: String, session: GlobalSystemMediaTransportControlsSession, _events: Option<SessionEvents>,
}
fn enumerate_sessions(manager: &GlobalSystemMediaTransportControlsSessionManager,
    known: &mut Vec<AvailableSession>, next_id: &mut u64, commands: &Sender<Command>) -> windows::core::Result<Vec<MediaSession>> {
    // Only reconcile after the complete enumeration succeeds. A provider failure
    // must not be mistaken for disappearance of a manually selected session.
    let view = manager.GetSessions()?;
    let mut live = Vec::new();
    for index in 0..view.Size()? { live.push(view.GetAt(index)?); }
    known.retain(|entry| live.contains(&entry.session));
    for session in live {
        if !known.iter().any(|entry| entry.session == session) {
            *next_id += 1;
            known.push(AvailableSession { id: next_id.to_string(), _events: watch_session(session.clone(), commands).ok(), session });
        }
    }
    for entry in known.iter_mut() {
        if entry._events.is_none() { entry._events = watch_session(entry.session.clone(), commands).ok(); }
    }
    Ok(known.iter().map(|entry| MediaSession {
        // Worker-lifetime identity: two sessions from the same app stay distinct.
        id: entry.id.clone(), source_app: entry.session.SourceAppUserModelId().unwrap_or_default().to_string(),
        playing: entry.session.GetPlaybackInfo().and_then(|p| p.PlaybackStatus()).is_ok_and(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing),
    }).collect())
}

fn media_worker(rx: Receiver<Command>, commands: Sender<Command>, tx: Sender<MediaEvent>,
    shared: Arc<Mutex<State>>, cancelled: Arc<AtomicBool>, ready: Sender<Result<(), String>>) -> Result<(), String> {
    let _runtime = Runtime::new()?;
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().and_then(|op| op.join()).map_err(|e| e.to_string())?;
    let events = commands.clone();
    let current = manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { metadata: true }); Ok(())
    })).map_err(|e| e.to_string())?;
    let events = commands.clone();
    let sessions = match manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
        let _ = events.send(Command::Refresh { metadata: true }); Ok(())
    })) {
        Ok(token) => token,
        Err(error) => { let _ = manager.RemoveCurrentSessionChanged(current); return Err(error.to_string()); }
    };
    let _manager_events = ManagerEvents { manager: manager.clone(), current, sessions };
    let mut known: Vec<AvailableSession> = Vec::new(); let mut next_id = 0;
    let mut manual = None; let mut selected: Option<String> = None;
    let mut published = None; let mut selection_revision = 0; let mut metadata_pending = true;
    let mut last_scan = Instant::now();
    let _ = ready.send(Ok(()));
    let _ = commands.send(Command::Refresh { metadata: true });
    while !cancelled.load(Ordering::Acquire) {
        let mut command = match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(command) => command,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if last_scan.elapsed() < Duration::from_secs(1) { continue; }
                Command::Refresh { metadata: false }
            },
            Err(_) => break,
        };
        if let Command::SelectSession(id, revision) = command {
            manual = id; selection_revision = revision;
            command = Command::Refresh { metadata: true };
        }
        let target = known.iter().find(|entry| Some(&entry.id) == selected.as_ref()).map(|entry| &entry.session);
        let (reply, result) = match command {
            Command::Stop => break,
            Command::SelectSession(..) => unreachable!(),
            Command::Refresh { metadata } => {
                last_scan = Instant::now();
                let Ok(sessions) = enumerate_sessions(&manager, &mut known, &mut next_id, &commands) else { continue; };
                let current = manager.GetCurrentSession().ok();
                let current_id = known.iter().find(|entry| Some(&entry.session) == current.as_ref()).map(|entry| entry.id.as_str());
                let latest = choose_session(&mut manual, &sessions, current_id);
                let switched = selected != latest;
                if switched {
                    selected = latest;
                    if let Ok(mut state) = shared.lock() { *state = State::default(); }
                }
                metadata_pending |= metadata || switched;
                let list = MediaSessions { sessions, manual: manual.clone(), selection_revision };
                if published.as_ref() != Some(&list) && !cancelled.load(Ordering::Acquire) {
                    let _ = tx.send(MediaEvent::SessionsChanged(list.clone())); published = Some(list);
                }
                if let Some(entry) = known.iter().find(|entry| Some(&entry.id) == selected.as_ref()) {
                    metadata_pending |= entry._events.is_none();
                    if let Ok(mut state) = read_state(&entry.session, metadata_pending, &shared) {
                        metadata_pending = false;
                        state.value.session_id = entry.id.clone(); state.value.selection_revision = selection_revision;
                        if !cancelled.load(Ordering::Acquire) {
                            if let Ok(mut current) = shared.lock() { *current = state; let _ = tx.send(MediaEvent::Changed(current.snapshot())); }
                        }
                    } else if !cancelled.load(Ordering::Acquire) {
                        metadata_pending = true;
                        if let Ok(mut state) = shared.lock() { *state = State::default(); }
                        let _ = tx.send(MediaEvent::Changed(MediaSnapshot { selection_revision, ..Default::default() }));
                    }
                } else if !cancelled.load(Ordering::Acquire) { let _ = tx.send(MediaEvent::Changed(MediaSnapshot { selection_revision, ..Default::default() })); }
                continue;
            }
            Command::Toggle(reply) => (reply, with_session(target, |s| {let info=s.GetPlaybackInfo()?;let controls=info.Controls()?;
                if controls.IsPlayPauseToggleEnabled()?{s.TryTogglePlayPauseAsync()?.join()}
                else if info.PlaybackStatus()?==GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing&&controls.IsPauseEnabled()?{s.TryPauseAsync()?.join()}
                else if controls.IsPlayEnabled()?{s.TryPlayAsync()?.join()}else{Ok(false)}
            })),
            Command::Next(reply) => (reply, with_session(target, |s| {
                if !s.GetPlaybackInfo()?.Controls()?.IsNextEnabled()? { return Ok(false); } s.TrySkipNextAsync()?.join()
            })),
            Command::Previous(reply) => (reply, with_session(target, |s| {
                if !s.GetPlaybackInfo()?.Controls()?.IsPreviousEnabled()? { return Ok(false); } s.TrySkipPreviousAsync()?.join()
            })),
            Command::Seek(seconds, reply) => (reply, with_session(target, |s| {
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
        let _ = commands.send(Command::Refresh { metadata: false });
    }
    drop(known); // remove all session event tokens before uninitializing WinRT
    Ok(())
}
fn with_session(selected: Option<&GlobalSystemMediaTransportControlsSession>, action: impl FnOnce(&GlobalSystemMediaTransportControlsSession) -> windows::core::Result<bool>) -> Result<(), String> {
    let session = selected.ok_or_else(|| "Belum ada sesi pemutar media Windows yang aktif.".to_string())?;
    match action(session) { Ok(true) => Ok(()), Ok(false) => Err("Pemutar media tidak mendukung perintah ini.".into()), Err(e) => Err(e.to_string()) }
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
    fn sessions() -> Vec<MediaSession> { vec![
        MediaSession { id: "1".into(), source_app: "chrome.exe".into(), playing: true },
        MediaSession { id: "2".into(), source_app: "chrome.exe".into(), playing: false },
    ] }
    #[test] fn manual_session_survives_pause_and_windows_current_change() {
        let mut manual = Some("2".into());
        assert_eq!(choose_session(&mut manual, &sessions(), Some("1")), Some("2".into()));
        assert_eq!(manual, Some("2".into()));
    }
    #[test] fn disappearance_and_stale_selection_return_to_auto() {
        let mut manual = Some("closed".into());
        assert_eq!(choose_session(&mut manual, &sessions(), Some("1")), Some("1".into()));
        assert_eq!(manual, None);
        assert_eq!(choose_session(&mut manual, &[], None), None);
    }
    #[test] fn auto_uses_windows_current_then_falls_back_to_playing_session() {
        let mut manual = None;
        // Windows current session wins when it is still enumerated.
        assert_eq!(choose_session(&mut manual, &sessions(), Some("2")), Some("2".into()));
        // No current session reported: Auto must pick an audibly playing source
        // instead of reporting "No music playing".
        assert_eq!(choose_session(&mut manual, &sessions(), None), Some("1".into()));
        // Current session reported but not audibly playing: still honour it.
        assert_eq!(choose_session(&mut manual, &sessions(), Some("1")), Some("1".into()));
        // Nothing enumerated and no current session: nothing to play.
        assert_eq!(choose_session(&mut manual, &[], None), None);
    }
    #[test] fn duplicate_app_names_are_separate_session_choices() {
        let mut manual = Some("1".into());
        assert_eq!(choose_session(&mut manual, &sessions(), Some("2")), Some("1".into()));
        manual = Some("2".into());
        assert_eq!(choose_session(&mut manual, &sessions(), Some("1")), Some("2".into()));
    }
    #[test]
    #[ignore = "read-only native GSMTC probe; run explicitly on the Windows desktop"]
    fn native_sessions_and_selection_probe() {
        let (tx, rx) = mpsc::channel();
        let mut service = MediaService::start(tx).expect("Windows GSMTC manager");
        let list = loop {
            if let MediaEvent::SessionsChanged(list) = rx.recv_timeout(Duration::from_secs(5)).expect("initial session enumeration") { break list; }
        };
        println!("GSMTC sessions={} (Playing is provider status, not audibility)", list.sessions.len());
        for session in &list.sessions {
            println!("id={} app={} playing={}", session.id, session.source_app, session.playing);
            let revision = service.select_session(Some(session.id.clone())).unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())).expect("selected snapshot") {
                    MediaEvent::Changed(track) if track.session_id == session.id && track.selection_revision == revision => { assert_eq!(track.source_app, session.source_app); break; },
                    _ => {},
                }
            }
            println!("selected_snapshot_id={}", session.id);
        }
        let revision = service.select_session(Some("missing-session-for-probe".into())).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let MediaEvent::SessionsChanged(list) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())).expect("Auto selection") {
                if list.manual.is_none() && list.selection_revision == revision { println!("missing selection -> Auto revision={revision}"); break; }
            }
        }
        let revision = service.select_session(None).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let MediaEvent::SessionsChanged(list) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())).expect("explicit Auto") {
                if list.manual.is_none() && list.selection_revision == revision { println!("explicit Auto revision={revision}"); break; }
            }
        }
        assert!(service.stop(), "native worker must stop within its grace period");
    }
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
