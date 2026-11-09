use super::adapter::*;
use super::model::*;
use std::time::Instant;

pub const FOCUS_BACK: usize = 0;
pub const FOCUS_CHEVRON: usize = 1;
pub const FOCUS_SETTINGS: usize = 2;
pub const FOCUS_WIFI_TOGGLE: usize = 3;
pub const FOCUS_WIFI_SETTINGS: usize = 4;
pub const FOCUS_BT_TOGGLE: usize = 5;
pub const FOCUS_BT_SETTINGS: usize = 6;
pub const FOCUS_BRIGHTNESS_SLIDER: usize = 7;
pub const FOCUS_DISPLAY_SETTINGS: usize = 8;
pub const NUM_FOCUS_ELEMENTS: usize = 9;

pub struct QuickControlsController {
    radio_adapter: Box<dyn RadioAdapter>,
    brightness_adapter: Box<dyn BrightnessAdapter>,
    pub radios: Vec<RadioDevice>,
    pub selected_wifi_idx: usize,
    pub selected_bt_idx: usize,
    pub displays: Vec<DisplayDevice>,
    pub selected_display_idx: usize,
    pub focused_element: Option<usize>,
    pub feedback_toast: Option<(String, Instant)>,
}

impl QuickControlsController {
    pub fn new(radio_adapter: Box<dyn RadioAdapter>, brightness_adapter: Box<dyn BrightnessAdapter>) -> Self {
        let mut ctrl = Self {
            radio_adapter,
            brightness_adapter,
            radios: Vec::new(),
            selected_wifi_idx: 0,
            selected_bt_idx: 0,
            displays: Vec::new(),
            selected_display_idx: 0,
            focused_element: None,
            feedback_toast: None,
        };
        ctrl.refresh();
        ctrl
    }

    pub fn new_mock() -> Self {
        Self::new(
            Box::new(MockRadioAdapter::new_default()),
            Box::new(MockBrightnessAdapter::new_default()),
        )
    }

    pub fn new_windows() -> Self {
        Self::new(
            Box::new(WindowsRadioAdapter::new()),
            Box::new(WindowsBrightnessAdapter::new()),
        )
    }

    pub fn refresh(&mut self) {
        self.radios = self.radio_adapter.enumerate_radios();
        self.displays = self.brightness_adapter.enumerate_displays();

        let wifi_count = self.radios_of_kind(RadioKind::WiFi).len();
        if self.selected_wifi_idx >= wifi_count {
            self.selected_wifi_idx = 0;
        }

        let bt_count = self.radios_of_kind(RadioKind::Bluetooth).len();
        if self.selected_bt_idx >= bt_count {
            self.selected_bt_idx = 0;
        }

        if self.selected_display_idx >= self.displays.len() {
            self.selected_display_idx = 0;
        }
    }

    pub fn radios_of_kind(&self, kind: RadioKind) -> Vec<&RadioDevice> {
        self.radios.iter().filter(|r| r.kind == kind).collect()
    }

    pub fn selected_radio(&self, kind: RadioKind) -> Option<&RadioDevice> {
        let matching = self.radios_of_kind(kind);
        let idx = if kind == RadioKind::WiFi {
            self.selected_wifi_idx
        } else {
            self.selected_bt_idx
        };
        matching.get(idx).copied()
    }

    pub fn selected_display(&self) -> Option<&DisplayDevice> {
        self.displays.get(self.selected_display_idx)
    }

    pub fn toggle_radio(&mut self, kind: RadioKind, lang: &str) -> Result<RadioState, String> {
        let (id, current_state, access, avail) = {
            let radio = self.selected_radio(kind).ok_or_else(|| "No radio selected".to_string())?;
            (radio.id.clone(), radio.state, radio.access, radio.availability.clone())
        };

        if avail == RadioAvailability::HardwareDisabled {
            self.feedback_toast = Some((
                crate::i18n::t("qc.hardware_switched", lang).to_string(),
                Instant::now(),
            ));
            return Err("Hardware disabled".to_string());
        }

        if access == RadioAccess::DeniedUser || access == RadioAccess::DeniedSystem {
            self.feedback_toast = Some((
                crate::i18n::t("qc.radio_denied", lang).to_string(),
                Instant::now(),
            ));
            return Err("Access denied".to_string());
        }

        let target_on = current_state != RadioState::On;
        let res = self.radio_adapter.set_radio_state(&id, target_on);

        match res {
            Ok(new_state) => {
    
                self.refresh();
                Ok(new_state)
            }
            Err(e) => {
                self.feedback_toast = Some((format!("{}: {e}", crate::i18n::t("qc.toggle_failed", lang)), Instant::now()));
                Err(e)
            }
        }
    }

    pub fn select_radio(&mut self, kind: RadioKind, idx: usize) {
        let count = self.radios_of_kind(kind).len();
        if count == 0 { return; }
        if kind == RadioKind::WiFi {
            self.selected_wifi_idx = idx % count;
        } else {
            self.selected_bt_idx = idx % count;
        }
    }

    pub fn cycle_radio_target(&mut self, kind: RadioKind) {
        let count = self.radios_of_kind(kind).len();
        if count <= 1 { return; }
        if kind == RadioKind::WiFi {
            self.selected_wifi_idx = (self.selected_wifi_idx + 1) % count;
        } else {
            self.selected_bt_idx = (self.selected_bt_idx + 1) % count;
        }
    }

    pub fn select_display(&mut self, idx: usize) {
        if self.displays.is_empty() { return; }
        self.selected_display_idx = idx % self.displays.len();
    }

    pub fn cycle_display_target(&mut self) {
        if self.displays.len() <= 1 { return; }
        self.selected_display_idx = (self.selected_display_idx + 1) % self.displays.len();
    }

    pub fn set_brightness(&mut self, raw_percent: u32, commit: bool, lang: &str) {
        if self.displays.is_empty() { return; }
        let idx = self.selected_display_idx;
        let disp = &mut self.displays[idx];

        if disp.kind == DisplayKind::Unsupported {
            self.feedback_toast = Some((
                crate::i18n::t("qc.unsupported_display", lang).to_string(),
                Instant::now(),
            ));
            return;
        }

        if !commit {
            disp.target_percent = Some(raw_percent.min(100));
        } else {
            let id = disp.id.clone();
            match self.brightness_adapter.set_brightness(&id, raw_percent) {
                Ok(new_val) => {
                    disp.current_percent = new_val;
                    disp.target_percent = None;
                    disp.status_text = format!("Aktual: {new_val}%");
                }
                Err(e) => {
                    disp.target_percent = None;
                    self.feedback_toast = Some((
                        format!("{}: {e}", crate::i18n::t("qc.toggle_failed", lang)),
                        Instant::now(),
                    ));
        
                }
            }

        }
    }

    pub fn step_brightness(&mut self, delta: i32, lang: &str) {
        if let Some(disp) = self.selected_display() {
            if disp.kind == DisplayKind::Unsupported { return; }
            let cur = disp.target_percent.unwrap_or(disp.current_percent) as i32;
            let target = (cur + delta).clamp(0, 100) as u32;
            self.set_brightness(target, true, lang);
        }
    }

    pub fn handle_key(&mut self, vk: u16, lang: &str) -> bool {
        match vk {
            0x09 => { // Tab
                let cur = self.focused_element.unwrap_or(NUM_FOCUS_ELEMENTS - 1);
                self.focused_element = Some((cur + 1) % NUM_FOCUS_ELEMENTS);
                true
            }
            0x25 => { // Left Arrow
                self.step_brightness(-1, lang);
                true
            }
            0x27 => { // Right Arrow
                self.step_brightness(1, lang);
                true
            }
            0x28 => { // Down Arrow
                self.step_brightness(-5, lang);
                true
            }
            0x26 => { // Up Arrow
                self.step_brightness(5, lang);
                true
            }
            0x21 => { // Page Up
                self.step_brightness(10, lang);
                true
            }
            0x22 => { // Page Down
                self.step_brightness(-10, lang);
                true
            }
            0x24 => { // Home
                self.set_brightness(0, true, lang);
                true
            }
            0x23 => { // End
                self.set_brightness(100, true, lang);
                true
            }
            0x20 | 0x0D => { // Space or Enter
                if let Some(focus) = self.focused_element {
                    match focus {
                        FOCUS_WIFI_TOGGLE => {
                            let _ = self.toggle_radio(RadioKind::WiFi, lang);
                            true
                        }
                        FOCUS_BT_TOGGLE => {
                            let _ = self.toggle_radio(RadioKind::Bluetooth, lang);
                            true
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
