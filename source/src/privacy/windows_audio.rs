use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::privacy::model::{Activity, CapabilityKind, Evidence, PrivacyEvent};
use crate::privacy::probe::probe_audio_endpoints;

/// Background worker monitoring active Windows CoreAudio capture sessions.
/// NEVER opens an audio capture stream or records buffers.
pub struct AudioMonitorWorker;

impl AudioMonitorWorker {
    pub fn spawn(
        event_tx: Sender<PrivacyEvent>,
        stop_flag: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        thread::Builder::new()
            .name("privacy-audio-monitor".into())
            .spawn(move || {
                let mut last_active = false;
                let mut last_apps: Vec<String> = Vec::new();
                let mut was_unknown = false;

                while !stop_flag.load(Ordering::Relaxed) {
                    match probe_audio_endpoints() {
                        Ok(probe) => {
                            let is_active = probe.active_sessions_count > 0;
                            let apps = probe.active_applications;

                            let changed = is_active != last_active || apps != last_apps || was_unknown;
                            if changed {
                                was_unknown = false;
                                last_active = is_active;
                                last_apps = apps.clone();

                                if is_active {
                                    let _ = event_tx.send(PrivacyEvent::CapabilityUpdate {
                                        kind: CapabilityKind::Microphone,
                                        activity: Activity::Active,
                                        evidence: Some(Evidence::CoreAudio),
                                        applications: apps,
                                    });
                                } else {
                                    let _ = event_tx.send(PrivacyEvent::MicInactive);
                                }
                            }
                        }
                        Err(_) => {
                            if !was_unknown {
                                was_unknown = true;
                                last_active = false;
                                last_apps.clear();
                                let _ = event_tx.send(PrivacyEvent::CapabilityUpdate {
                                    kind: CapabilityKind::Microphone,
                                    activity: Activity::Unknown,
                                    evidence: None,
                                    applications: Vec::new(),
                                });
                            }
                        }
                    }

                    // Poll every 250ms, checking stop flag every 10ms for prompt shutdown
                    for _ in 0..25 {
                        if stop_flag.load(Ordering::Relaxed) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            })
            .expect("failed to spawn privacy-audio-monitor thread")
    }
}
