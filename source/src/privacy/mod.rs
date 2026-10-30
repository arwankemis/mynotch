use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub mod model;
pub mod mailbox;
pub mod probe;
pub mod windows_audio;
pub mod windows_camera;

#[cfg(test)]
mod tests;

pub use model::{
    Activity, CapabilityKind, CapabilityState, Coverage, Evidence, PrivacyEvent, PrivacyReducer,
    PrivacySnapshot, DEACTIVATION_GRACE_PERIOD_MS,
};
pub use mailbox::{mailbox, MailboxError, MailboxReceiver, MailboxSender};
pub use windows_audio::AudioMonitorWorker;
pub use windows_camera::CameraMonitorWorker;

/// Coordinator handle managing background audio and camera monitor workers.
/// Aggregates events into `PrivacyReducer` and posts atomic snapshots into `MailboxSender`.
/// Shuts down all workers cleanly when dropped or stopped.
pub struct PrivacyMonitorHandle {
    stop_flag: Arc<AtomicBool>,
    audio_thread: Option<JoinHandle<()>>,
    camera_thread: Option<JoinHandle<()>>,
    coordinator_thread: Option<JoinHandle<()>>,
}

impl PrivacyMonitorHandle {
    pub fn start(mailbox_tx: MailboxSender) -> Self {
        Self::start_with_generation(mailbox_tx, 1)
    }

    pub fn start_with_generation(mailbox_tx: MailboxSender, generation: u64) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (event_tx, event_rx) = std::sync::mpsc::channel::<PrivacyEvent>();

        // 1. Audio monitor worker thread
        let audio_thread = AudioMonitorWorker::spawn(event_tx.clone(), Arc::clone(&stop_flag));

        // 2. Camera monitor worker thread
        let camera_thread = CameraMonitorWorker::spawn(event_tx.clone(), Arc::clone(&stop_flag));

        // 3. Coordinator / Reducer thread
        let coord_stop = Arc::clone(&stop_flag);
        let coordinator_thread = thread::Builder::new()
            .name("privacy-coordinator".into())
            .spawn(move || {
                let mut reducer = PrivacyReducer::with_generation(generation);
                // Seed initial snapshot
                let _ = mailbox_tx.post(reducer.snapshot().clone());

                while !coord_stop.load(Ordering::Relaxed) {
                    match event_rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(event) => {
                            let changed = reducer.handle_event(event, Instant::now());
                            if changed {
                                let _ = mailbox_tx.post(reducer.snapshot().clone());
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            // Check deactivation deadlines periodically
                            let changed = reducer.handle_event(PrivacyEvent::Tick, Instant::now());
                            if changed {
                                let _ = mailbox_tx.post(reducer.snapshot().clone());
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
            })
            .expect("failed to spawn privacy-coordinator thread");

        Self {
            stop_flag,
            audio_thread: Some(audio_thread),
            camera_thread: Some(camera_thread),
            coordinator_thread: Some(coordinator_thread),
        }
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(h) = self.audio_thread.take() {
            let _ = h.join();
        }
        if let Some(h) = self.camera_thread.take() {
            let _ = h.join();
        }
        if let Some(h) = self.coordinator_thread.take() {
            let _ = h.join();
        }
    }
}

impl Drop for PrivacyMonitorHandle {
    fn drop(&mut self) {
        self.stop();
    }
}
