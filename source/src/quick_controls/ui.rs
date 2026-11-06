use crate::render::{self, Painter};
use super::model::*;

pub const QC_PANEL_WIDTH: f32 = 460.0;
pub const QC_PANEL_HEIGHT: f32 = 286.0;
pub const QC_CARD_RADIUS: f32 = 10.0;

pub struct QuickControlsLayout {
    pub back_btn: render::D2D_RECT_F,
    pub pin_btn: render::D2D_RECT_F,
    pub chevron_btn: render::D2D_RECT_F,
    pub settings_btn: render::D2D_RECT_F,

    pub card_wifi: render::D2D_RECT_F,
    pub wifi_toggle: render::D2D_RECT_F,
    pub wifi_selector: render::D2D_RECT_F,
    pub wifi_settings_btn: render::D2D_RECT_F,

    pub card_bt: render::D2D_RECT_F,
    pub bt_toggle: render::D2D_RECT_F,
    pub bt_selector: render::D2D_RECT_F,
    pub bt_settings_btn: render::D2D_RECT_F,

    pub card_display: render::D2D_RECT_F,
    pub display_selector: render::D2D_RECT_F,
    pub brightness_slider_hit: render::D2D_RECT_F,
    pub display_settings_btn: render::D2D_RECT_F,
}

pub fn calculate_layout() -> QuickControlsLayout {
    QuickControlsLayout {
        back_btn: render::rect(14.0, 10.0, 76.0, 24.0),
        pin_btn: render::rect(366.0, 10.0, 24.0, 24.0),
        chevron_btn: render::rect(396.0, 10.0, 24.0, 24.0),
        settings_btn: render::rect(426.0, 10.0, 24.0, 24.0),

        card_wifi: render::rect(14.0, 42.0, 432.0, 68.0),
        wifi_toggle: render::rect(404.0, 52.0, 32.0, 18.0),
        wifi_selector: render::rect(58.0, 66.0, 200.0, 18.0),
        wifi_settings_btn: render::rect(348.0, 84.0, 88.0, 20.0),

        card_bt: render::rect(14.0, 116.0, 432.0, 68.0),
        bt_toggle: render::rect(404.0, 126.0, 32.0, 18.0),
        bt_selector: render::rect(58.0, 140.0, 200.0, 18.0),
        bt_settings_btn: render::rect(348.0, 158.0, 88.0, 20.0),

        card_display: render::rect(14.0, 190.0, 432.0, 86.0),
        display_selector: render::rect(130.0, 196.0, 200.0, 18.0),
        brightness_slider_hit: render::rect(22.0, 216.0, 408.0, 22.0),
        display_settings_btn: render::rect(348.0, 248.0, 88.0, 20.0),
    }
}

pub unsafe fn draw_toggle(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, state: RadioState, avail: &RadioAvailability, focused: bool) {
    unsafe {
        let radius = h * 0.5;
        let (track_col, knob_col, knob_x) = match state {
            RadioState::On => (0xd8d8de, 0x101015, x + w - h + 2.0),
            RadioState::Off => (0x35353d, 0xaaaab3, x + 2.0),
            RadioState::Unknown => (0x3a3830, 0xeab308, x + (w - h) * 0.5 + 2.0),
        };

        let disabled = *avail == RadioAvailability::HardwareDisabled || *avail == RadioAvailability::Absent;
        let alpha = if disabled { 0.45 } else { 1.0 };

        p.fill_alpha(x, y, w, h, radius, track_col, alpha);
        p.fill_alpha(knob_x, y + 2.0, h - 4.0, h - 4.0, (h - 4.0) * 0.5, knob_col, alpha);

        if focused {
            p.stroke(x - 2.0, y - 2.0, w + 4.0, h + 4.0, radius + 2.0, 0xd8d8de, 1.5);
        }
    }
}
