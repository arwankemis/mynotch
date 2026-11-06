use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioKind {
    WiFi,
    Bluetooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioState {
    On,
    Off,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioAccess {
    Allowed,
    DeniedUser,
    DeniedSystem,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioAvailability {
    Ready,
    PendingTransition { target_on: bool, requested_at: Instant },
    PendingConsent,
    HardwareDisabled,
    Absent,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioDevice {
    pub id: String,
    pub name: String,
    pub kind: RadioKind,
    pub state: RadioState,
    pub access: RadioAccess,
    pub availability: RadioAvailability,
    pub status_text: String,
    pub sub_status_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKind {
    InternalWmi,
    ExternalMccs,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayDevice {
    pub id: String,
    pub name: String,
    pub kind: DisplayKind,
    pub current_percent: u32,
    pub target_percent: Option<u32>,
    pub supported_levels: Vec<u8>,
    pub min_percent: u32,
    pub max_percent: u32,
    pub status_text: String,
    pub sub_status_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickControlsSnapshot {
    pub radios: Vec<RadioDevice>,
    pub selected_wifi_idx: usize,
    pub selected_bt_idx: usize,
    pub displays: Vec<DisplayDevice>,
    pub selected_display_idx: usize,
}

/// Snap brightness percent to nearest supported level (e.g. WMI levels [0, 10, 25, 50, 75, 100]).
/// In case of an exact tie between two equidistant levels, tie-breaks higher (deterministic).
/// If `supported_levels` is empty, returns percent clamped to 0..=100.
pub fn snap_brightness(raw_percent: u32, supported_levels: &[u8]) -> u32 {
    let clamped = raw_percent.min(100);
    if supported_levels.is_empty() {
        return clamped;
    }

    let mut best_level = supported_levels[0] as u32;
    let mut best_diff = (best_level as i32 - clamped as i32).abs();

    for &lvl in &supported_levels[1..] {
        let candidate = lvl as u32;
        let diff = (candidate as i32 - clamped as i32).abs();
        if diff < best_diff || (diff == best_diff && candidate > best_level) {
            best_diff = diff;
            best_level = candidate;
        }
    }
    best_level
}
