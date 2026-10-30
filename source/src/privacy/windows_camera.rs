use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::privacy::model::{Activity, CapabilityKind, Evidence, PrivacyEvent};
use crate::privacy::probe::probe_consent_store_webcam;

/// Background worker monitoring webcam access via Windows ConsentStore registry metadata.
/// Read-only observation; NEVER opens camera devices or video streams.
pub struct CameraMonitorWorker;

impl CameraMonitorWorker {
    pub fn spawn(
        event_tx: Sender<PrivacyEvent>,
        stop_flag: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        thread::Builder::new()
            .name("privacy-camera-monitor".into())
            .spawn(move || {
                let mut last_active = false;
                let mut last_apps: Vec<String> = Vec::new();

                while !stop_flag.load(Ordering::Relaxed) {
                    if let Ok(probe) = probe_consent_store_webcam() {
                        let is_active = !probe.active_candidates.is_empty();
                        let apps = probe.active_candidates;

                        let changed = is_active != last_active || apps != last_apps;
                        if changed {
                            last_active = is_active;
                            last_apps = apps.clone();

                            if is_active {
                                let _ = event_tx.send(PrivacyEvent::CapabilityUpdate {
                                    kind: CapabilityKind::Camera,
                                    activity: Activity::Active,
                                    evidence: Some(Evidence::ConsentStoreExperimental),
                                    applications: apps,
                                });
                            } else {
                                let _ = event_tx.send(PrivacyEvent::CameraInactive);
                            }
                        }
                    }

                    // Poll every 400ms, checking stop flag every 10ms for prompt shutdown
                    for _ in 0..40 {
                        if stop_flag.load(Ordering::Relaxed) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            })
            .expect("failed to spawn privacy-camera-monitor thread")
    }
}
