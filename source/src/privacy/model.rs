use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const DEACTIVATION_GRACE_PERIOD_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    Active,
    Inactive,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Coverage {
    Supported,
    Partial,
    Unsupported,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Evidence {
    CoreAudio,
    ConsentStoreExperimental,
    Integration,
    Simulation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityState {
    pub activity: Activity,
    pub coverage: Coverage,
    pub evidence: Option<Evidence>,
    pub active_since: Option<u64>, // Monotonic or timestamp
    pub applications: Vec<String>,
}

impl CapabilityState {
    pub fn new(coverage: Coverage) -> Self {
        let activity = match coverage {
            Coverage::Supported | Coverage::Partial => Activity::Inactive,
            Coverage::Unsupported | Coverage::Unavailable => Activity::Unknown,
        };
        Self {
            activity,
            coverage,
            evidence: None,
            active_since: None,
            applications: Vec::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.activity == Activity::Active
    }

    pub fn is_unknown(&self) -> bool {
        self.activity == Activity::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacySnapshot {
    pub generation: u64,
    pub revision: u64,
    pub microphone: CapabilityState,
    pub camera: CapabilityState,
    pub screen_share: CapabilityState,
    pub screen_recording: CapabilityState,
}

impl PrivacySnapshot {
    pub fn new(generation: u64) -> Self {
        Self {
            generation,
            revision: 0,
            microphone: CapabilityState::new(Coverage::Supported),
            camera: CapabilityState::new(Coverage::Partial),
            screen_share: CapabilityState::new(Coverage::Unsupported),
            screen_recording: CapabilityState::new(Coverage::Unsupported),
        }
    }

    pub fn any_active(&self) -> bool {
        self.microphone.is_active()
            || self.camera.is_active()
            || self.screen_share.is_active()
            || self.screen_recording.is_active()
    }

    pub fn capability(&self, kind: CapabilityKind) -> &CapabilityState {
        match kind {
            CapabilityKind::Microphone => &self.microphone,
            CapabilityKind::Camera => &self.camera,
            CapabilityKind::ScreenShare => &self.screen_share,
            CapabilityKind::ScreenRecording => &self.screen_recording,
        }
    }

    pub fn capability_mut(&mut self, kind: CapabilityKind) -> &mut CapabilityState {
        match kind {
            CapabilityKind::Microphone => &mut self.microphone,
            CapabilityKind::Camera => &mut self.camera,
            CapabilityKind::ScreenShare => &mut self.screen_share,
            CapabilityKind::ScreenRecording => &mut self.screen_recording,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityKind {
    Microphone,
    Camera,
    ScreenShare,
    ScreenRecording,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyEvent {
    CapabilityUpdate {
        kind: CapabilityKind,
        activity: Activity,
        evidence: Option<Evidence>,
        applications: Vec<String>,
    },
    MicActive {
        app: String,
    },
    MicInactive,
    CameraActive {
        app: String,
    },
    CameraInactive,
    ScreenShareActive {
        app: String,
    },
    ScreenShareInactive,
    ScreenRecordingActive {
        app: String,
    },
    ScreenRecordingInactive,
    Tick,
}

#[derive(Debug, Clone, Copy)]
pub enum Timestamp {
    Millis(u64),
    Instant(Instant),
}

impl From<u64> for Timestamp {
    fn from(v: u64) -> Self {
        Timestamp::Millis(v)
    }
}

impl From<Instant> for Timestamp {
    fn from(v: Instant) -> Self {
        Timestamp::Instant(v)
    }
}

pub fn normalize_and_dedup_apps(apps: &[String]) -> Vec<String> {
    let mut deduped: Vec<String> = Vec::with_capacity(apps.len());
    for item in apps {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !deduped.iter().any(|existing| existing.eq_ignore_ascii_case(trimmed)) {
            deduped.push(trimmed.to_string());
        }
    }
    deduped
}

pub fn merge_apps(existing: &mut Vec<String>, new_apps: &[String]) -> bool {
    let mut changed = false;
    for app in new_apps {
        let trimmed = app.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !existing.iter().any(|e| e.eq_ignore_ascii_case(trimmed)) {
            existing.push(trimmed.to_string());
            changed = true;
        }
    }
    changed
}

#[derive(Debug, Clone)]
pub struct PrivacyReducer {
    snapshot: PrivacySnapshot,
    base_instant: Option<Instant>,
    mic_deactivation_deadline: Option<u64>,
    camera_deactivation_deadline: Option<u64>,
    share_deactivation_deadline: Option<u64>,
    record_deactivation_deadline: Option<u64>,
}

impl Default for PrivacyReducer {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivacyReducer {
    pub fn new() -> Self {
        Self::with_generation(1)
    }

    pub fn with_generation(generation: u64) -> Self {
        Self {
            snapshot: PrivacySnapshot::new(generation),
            base_instant: None,
            mic_deactivation_deadline: None,
            camera_deactivation_deadline: None,
            share_deactivation_deadline: None,
            record_deactivation_deadline: None,
        }
    }

    pub fn snapshot(&self) -> &PrivacySnapshot {
        &self.snapshot
    }

    pub fn set_generation(&mut self, generation: u64) {
        if self.snapshot.generation != generation {
            self.snapshot.generation = generation;
            self.snapshot.revision = 0;
            self.mic_deactivation_deadline = None;
            self.camera_deactivation_deadline = None;
            self.share_deactivation_deadline = None;
            self.record_deactivation_deadline = None;
        }
    }

    fn to_ms(&mut self, time: Timestamp) -> u64 {
        match time {
            Timestamp::Millis(ms) => ms,
            Timestamp::Instant(inst) => {
                let base = *self.base_instant.get_or_insert(inst);
                inst.duration_since(base).as_millis() as u64
            }
        }
    }

    fn deadline_mut(&mut self, kind: CapabilityKind) -> &mut Option<u64> {
        match kind {
            CapabilityKind::Microphone => &mut self.mic_deactivation_deadline,
            CapabilityKind::Camera => &mut self.camera_deactivation_deadline,
            CapabilityKind::ScreenShare => &mut self.share_deactivation_deadline,
            CapabilityKind::ScreenRecording => &mut self.record_deactivation_deadline,
        }
    }

    pub fn handle_event(&mut self, event: PrivacyEvent, time: impl Into<Timestamp>) -> bool {
        let now_ms = self.to_ms(time.into());
        let mut changed = false;

        match event {
            PrivacyEvent::MicActive { app } => {
                changed |= self.apply_active(
                    CapabilityKind::Microphone,
                    Evidence::CoreAudio,
                    &[app],
                    now_ms,
                );
            }
            PrivacyEvent::MicInactive => {
                changed |= self.apply_inactive(CapabilityKind::Microphone, now_ms);
            }
            PrivacyEvent::CameraActive { app } => {
                changed |= self.apply_active(
                    CapabilityKind::Camera,
                    Evidence::ConsentStoreExperimental,
                    &[app],
                    now_ms,
                );
            }
            PrivacyEvent::CameraInactive => {
                changed |= self.apply_inactive(CapabilityKind::Camera, now_ms);
            }
            PrivacyEvent::ScreenShareActive { app } => {
                changed |= self.apply_active(
                    CapabilityKind::ScreenShare,
                    Evidence::Integration,
                    &[app],
                    now_ms,
                );
            }
            PrivacyEvent::ScreenShareInactive => {
                changed |= self.apply_inactive(CapabilityKind::ScreenShare, now_ms);
            }
            PrivacyEvent::ScreenRecordingActive { app } => {
                changed |= self.apply_active(
                    CapabilityKind::ScreenRecording,
                    Evidence::Integration,
                    &[app],
                    now_ms,
                );
            }
            PrivacyEvent::ScreenRecordingInactive => {
                changed |= self.apply_inactive(CapabilityKind::ScreenRecording, now_ms);
            }
            PrivacyEvent::CapabilityUpdate {
                kind,
                activity,
                evidence,
                applications,
            } => match activity {
                Activity::Active => {
                    let ev = evidence.unwrap_or(match kind {
                        CapabilityKind::Microphone => Evidence::CoreAudio,
                        CapabilityKind::Camera => Evidence::ConsentStoreExperimental,
                        CapabilityKind::ScreenShare | CapabilityKind::ScreenRecording => {
                            Evidence::Integration
                        }
                    });
                    changed |= self.apply_active(kind, ev, &applications, now_ms);
                }
                Activity::Inactive => {
                    changed |= self.apply_inactive(kind, now_ms);
                }
                Activity::Unknown => {
                    changed |= self.apply_unknown(kind);
                }
            },
            PrivacyEvent::Tick => {}
        }

        // Check grace period expiry for all capabilities
        changed |= self.check_deadlines(now_ms);

        if changed {
            self.snapshot.revision = self.snapshot.revision.wrapping_add(1);
        }
        changed
    }

    fn apply_active(
        &mut self,
        kind: CapabilityKind,
        evidence: Evidence,
        apps: &[String],
        now_ms: u64,
    ) -> bool {
        let mut changed = false;
        *self.deadline_mut(kind) = None; // Cancel pending grace period deactivation immediately!

        let cap = self.snapshot.capability_mut(kind);
        if cap.activity != Activity::Active {
            cap.activity = Activity::Active;
            cap.active_since = Some(now_ms);
            changed = true;
        } else if cap.active_since.is_none() {
            cap.active_since = Some(now_ms);
            changed = true;
        }

        if cap.evidence != Some(evidence) {
            cap.evidence = Some(evidence);
            changed = true;
        }

        let deduped = normalize_and_dedup_apps(apps);
        if !deduped.is_empty() {
            let app_changed = merge_apps(&mut cap.applications, &deduped);
            if app_changed {
                changed = true;
            }
        }

        changed
    }

    fn apply_inactive(&mut self, kind: CapabilityKind, now_ms: u64) -> bool {
        let cap = self.snapshot.capability(kind);
        if cap.activity == Activity::Active {
            let deadline = self.deadline_mut(kind);
            if deadline.is_none() {
                // Enter 500ms grace period. State stays Active until deadline expires.
                *deadline = Some(now_ms.saturating_add(DEACTIVATION_GRACE_PERIOD_MS));
            }
            false
        } else if cap.activity == Activity::Unknown {
            // Transitioning from Unknown to Inactive when monitor confirms inactive.
            *self.deadline_mut(kind) = None;
            let cap_mut = self.snapshot.capability_mut(kind);
            cap_mut.activity = Activity::Inactive;
            cap_mut.active_since = None;
            cap_mut.applications.clear();
            cap_mut.evidence = None;
            true
        } else {
            false
        }
    }

    fn apply_unknown(&mut self, kind: CapabilityKind) -> bool {
        *self.deadline_mut(kind) = None;
        let cap = self.snapshot.capability_mut(kind);
        if cap.activity != Activity::Unknown {
            cap.activity = Activity::Unknown;
            cap.active_since = None;
            cap.applications.clear();
            cap.evidence = None;
            true
        } else {
            false
        }
    }

    fn check_deadlines(&mut self, now_ms: u64) -> bool {
        let kinds = [
            CapabilityKind::Microphone,
            CapabilityKind::Camera,
            CapabilityKind::ScreenShare,
            CapabilityKind::ScreenRecording,
        ];
        let mut changed = false;

        for kind in kinds {
            let mut expired = false;
            if let Some(deadline) = *self.deadline_mut(kind) {
                if now_ms >= deadline {
                    expired = true;
                }
            }

            if expired {
                *self.deadline_mut(kind) = None;
                let cap = self.snapshot.capability_mut(kind);
                if cap.activity != Activity::Inactive {
                    cap.activity = Activity::Inactive;
                    cap.active_since = None;
                    cap.applications.clear();
                    cap.evidence = None;
                    changed = true;
                }
            }
        }

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_snapshot_honest_defaults() {
        let reducer = PrivacyReducer::new();
        let snap = reducer.snapshot();
        assert_eq!(snap.generation, 1);
        assert_eq!(snap.revision, 0);
        assert_eq!(snap.microphone.coverage, Coverage::Supported);
        assert_eq!(snap.microphone.activity, Activity::Inactive);
        assert_eq!(snap.camera.coverage, Coverage::Partial);
        assert_eq!(snap.camera.activity, Activity::Inactive);
        assert_eq!(snap.screen_share.coverage, Coverage::Unsupported);
        assert_eq!(snap.screen_share.activity, Activity::Unknown);
        assert_eq!(snap.screen_recording.coverage, Coverage::Unsupported);
        assert_eq!(snap.screen_recording.activity, Activity::Unknown);
    }

    #[test]
    fn test_immediate_onset_on_activity() {
        let mut reducer = PrivacyReducer::new();
        let changed = reducer.handle_event(
            PrivacyEvent::MicActive {
                app: "chrome.exe".into(),
            },
            1000u64,
        );
        assert!(changed);
        let snap = reducer.snapshot();
        assert_eq!(snap.microphone.activity, Activity::Active);
        assert_eq!(snap.microphone.active_since, Some(1000));
        assert_eq!(snap.microphone.applications, vec!["chrome.exe"]);
        assert_eq!(snap.revision, 1);
    }

    #[test]
    fn test_500ms_grace_period_and_expiration() {
        let mut reducer = PrivacyReducer::new();
        reducer.handle_event(
            PrivacyEvent::MicActive {
                app: "zoom.exe".into(),
            },
            1000u64,
        );
        assert_eq!(reducer.snapshot().microphone.activity, Activity::Active);

        // At 1200ms, inactive signal arrives
        reducer.handle_event(PrivacyEvent::MicInactive, 1200u64);
        // During grace period (1200ms to 1700ms), state MUST stay Active!
        assert_eq!(
            reducer.snapshot().microphone.activity,
            Activity::Active,
            "State must remain Active during grace period"
        );
        assert_eq!(
            reducer.snapshot().microphone.applications,
            vec!["zoom.exe"]
        );

        // At 1699ms, still within grace period
        reducer.handle_event(PrivacyEvent::Tick, 1699u64);
        assert_eq!(reducer.snapshot().microphone.activity, Activity::Active);

        // At 1700ms, grace period expires
        reducer.handle_event(PrivacyEvent::Tick, 1700u64);
        assert_eq!(
            reducer.snapshot().microphone.activity,
            Activity::Inactive,
            "State must transition to Inactive after 500ms grace period"
        );
        assert!(reducer.snapshot().microphone.applications.is_empty());
        assert_eq!(reducer.snapshot().microphone.active_since, None);
    }

    #[test]
    fn test_grace_cancellation_on_immediate_resume() {
        let mut reducer = PrivacyReducer::new();
        reducer.handle_event(
            PrivacyEvent::MicActive {
                app: "teams.exe".into(),
            },
            1000u64,
        );

        // Inactive at 1500ms (deadline = 2000ms)
        reducer.handle_event(PrivacyEvent::MicInactive, 1500u64);
        assert_eq!(reducer.snapshot().microphone.activity, Activity::Active);

        // Resume at 1700ms (before deadline)
        reducer.handle_event(
            PrivacyEvent::MicActive {
                app: "teams.exe".into(),
            },
            1700u64,
        );
        assert_eq!(reducer.snapshot().microphone.activity, Activity::Active);

        // Now advance past original deadline (e.g. 2100ms)
        reducer.handle_event(PrivacyEvent::Tick, 2100u64);
        // Should STILL be Active because grace period was cancelled!
        assert_eq!(
            reducer.snapshot().microphone.activity,
            Activity::Active,
            "Grace period should have been cancelled by immediate resume"
        );
        assert_eq!(reducer.snapshot().microphone.active_since, Some(1000));
    }

    #[test]
    fn test_unknown_status_propagation() {
        let mut reducer = PrivacyReducer::new();
        // Camera starts Inactive
        assert_eq!(reducer.snapshot().camera.activity, Activity::Inactive);

        // Update to Unknown (e.g. registry error or unmonitored)
        reducer.handle_event(
            PrivacyEvent::CapabilityUpdate {
                kind: CapabilityKind::Camera,
                activity: Activity::Unknown,
                evidence: None,
                applications: vec![],
            },
            1000u64,
        );
        assert_eq!(
            reducer.snapshot().camera.activity,
            Activity::Unknown,
            "Unknown status must propagate immediately without fake Inactive"
        );
    }

    #[test]
    fn test_multi_app_merging_and_deduplication() {
        let mut reducer = PrivacyReducer::new();
        reducer.handle_event(
            PrivacyEvent::MicActive {
                app: "chrome.exe".into(),
            },
            1000u64,
        );
        reducer.handle_event(
            PrivacyEvent::MicActive {
                app: "discord.exe".into(),
            },
            1100u64,
        );
        // Duplicate app with different case and whitespace
        reducer.handle_event(
            PrivacyEvent::MicActive {
                app: "  CHROME.EXE  ".into(),
            },
            1200u64,
        );

        let apps = &reducer.snapshot().microphone.applications;
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0], "chrome.exe");
        assert_eq!(apps[1], "discord.exe");
    }
}
