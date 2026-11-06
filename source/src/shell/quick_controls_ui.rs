use crate::render::Painter;
use crate::shell::{Action, Hit, Shell};

impl Shell {
    pub(super) unsafe fn draw_quick_controls(&mut self, p: &mut Painter) { unsafe {
        let layout = crate::quick_controls::calculate_layout();
        let bg_col = 0x000000;
        p.fill(0., 0., self.width, self.height, 18., bg_col);

        // Header: Back button [← Kembali]
        let back_hover = self.pressed == Some(Action::QuickControlsBack);
        let back_bg = if back_hover { 0x292930 } else { 0x1c1c23 };
        p.fill(layout.back_btn.left, layout.back_btn.top, layout.back_btn.right - layout.back_btn.left, layout.back_btn.bottom - layout.back_btn.top, 6., back_bg);
        p.outline_icon("left", layout.back_btn.left + 8., layout.back_btn.top + 6., 12., 0x9ca3af);
        p.text("Kembali", layout.back_btn.left + 24., layout.back_btn.top + 4., 48., 16., 11., 0xf3f3f5, true);
        if self.hit_enabled {
            self.hits.push(Hit { rect: layout.back_btn, action: Action::QuickControlsBack });
        }

        // Title: Kontrol cepat
        p.text("Kontrol cepat", 98., 14., 90., 18., 13.5, 0xffffff, true);
        // Badge: KONTROL
        p.fill_alpha(194., 14., 52., 18., 4., 0x1c212d, 1.0);
        p.stroke(194., 14., 52., 18., 4., 0xd8d8de, 1.0);
        p.text_centered("KONTROL", 194., 14., 52., 18., 9.5, 0xd8d8de, 1.0, true);

        // Header right controls: Pin, Chevron up (Fold to Stage 1), Settings
        let y_hdr = 10.;
        self.navigation_control(p, "pin", layout.pin_btn.left, y_hdr, Action::TogglePin, self.pinned);
        self.navigation_control(p, "up", layout.chevron_btn.left, y_hdr, Action::ToStage1, false);
        self.navigation_control(p, "settings", layout.settings_btn.left, y_hdr, Action::Settings, false);

        // Card 1: Wi-Fi
        let card1_w = layout.card_wifi.right - layout.card_wifi.left;
        let card1_h = layout.card_wifi.bottom - layout.card_wifi.top;
        p.fill(layout.card_wifi.left, layout.card_wifi.top, card1_w, card1_h, 10., 0x12141a);
        p.stroke(layout.card_wifi.left, layout.card_wifi.top, card1_w, card1_h, 10., 0x222229, 1.0);

        let (wifi_on, wifi_avail, wifi_name, wifi_status, wifi_sub) = if let Some(r) = self.quick_controls.selected_radio(crate::quick_controls::RadioKind::WiFi) {
            (r.state == crate::quick_controls::RadioState::On, r.availability.clone(), r.name.clone(), r.status_text.clone(), r.sub_status_text.clone())
        } else {
            (false, crate::quick_controls::RadioAvailability::Absent, "Wi-Fi".to_string(), "Tidak ada adaptor Wi-Fi".to_string(), "".to_string())
        };

        // Icon badge
        let badge1_bg = if wifi_on { 0x1b1e26 } else { 0x16181f };
        p.fill(22., 48., 28., 28., 7., badge1_bg);
        p.outline_icon("wifi", 28., 54., 16., if wifi_on { 0xd8d8de } else { 0x9ca3af });

        // Title & target selector
        p.text(&wifi_name, 58., 48., 280., 18., 12.5, 0xf3f3f5, true);
        if self.quick_controls.radios_of_kind(crate::quick_controls::RadioKind::WiFi).len() > 1 {
            p.fill(58., 66., 180., 16., 4., 0x181b22);
            p.stroke(58., 66., 180., 16., 4., 0x2b2e3a, 1.0);
            p.text_ellipsis("Klik untuk ganti radio", 64., 67., 168., 14., 9.5, 0x9ca3af, false);
            if self.hit_enabled {
                self.hits.push(Hit { rect: layout.wifi_selector, action: Action::QuickControlsSelectRadio(crate::quick_controls::RadioKind::WiFi, 0) });
            }
        }

        // Toggle button
        let wifi_focus = self.quick_controls.focused_element == Some(crate::quick_controls::FOCUS_WIFI_TOGGLE);
        let wifi_state = if wifi_on { crate::quick_controls::RadioState::On } else { crate::quick_controls::RadioState::Off };
        crate::quick_controls::draw_toggle(p, layout.wifi_toggle.left, layout.wifi_toggle.top, 32., 18., wifi_state, &wifi_avail, wifi_focus);
        if self.hit_enabled {
            self.hits.push(Hit { rect: layout.wifi_toggle, action: Action::QuickControlsToggleRadio(crate::quick_controls::RadioKind::WiFi) });
        }

        // Status row & Settings button
        p.text(&wifi_status, 22., 82., 260., 16., 11., if wifi_on { 0xd8d8de } else { 0x9ca3af }, false);
        p.text(&wifi_sub, 22., 96., 320., 12., 9.5, 0x777781, false);

        p.fill(layout.wifi_settings_btn.left, layout.wifi_settings_btn.top, 88., 20., 5., 0x1c1c23);
        p.stroke(layout.wifi_settings_btn.left, layout.wifi_settings_btn.top, 88., 20., 5., 0x2d3242, 1.0);
        p.text("Pengaturan", layout.wifi_settings_btn.left + 8., layout.wifi_settings_btn.top + 3., 60., 14., 10., 0x9ca3af, false);
        p.outline_icon("right", layout.wifi_settings_btn.left + 70., layout.wifi_settings_btn.top + 4., 11., 0x9ca3af);
        if self.hit_enabled {
            self.hits.push(Hit { rect: layout.wifi_settings_btn, action: Action::OpenSettingsPage("ms-settings:network-wifi") });
        }

        // Card 2: Bluetooth
        let card2_w = layout.card_bt.right - layout.card_bt.left;
        let card2_h = layout.card_bt.bottom - layout.card_bt.top;
        p.fill(layout.card_bt.left, layout.card_bt.top, card2_w, card2_h, 10., 0x12141a);
        p.stroke(layout.card_bt.left, layout.card_bt.top, card2_w, card2_h, 10., 0x222229, 1.0);

        let (bt_on, bt_avail, bt_name, bt_status, bt_sub) = if let Some(r) = self.quick_controls.selected_radio(crate::quick_controls::RadioKind::Bluetooth) {
            (r.state == crate::quick_controls::RadioState::On, r.availability.clone(), r.name.clone(), r.status_text.clone(), r.sub_status_text.clone())
        } else {
            (false, crate::quick_controls::RadioAvailability::Absent, "Bluetooth".to_string(), "Tidak ada adaptor Bluetooth".to_string(), "".to_string())
        };

        let badge2_bg = if bt_on { 0x1b1e26 } else { 0x16181f };
        p.fill(22., 122., 28., 28., 7., badge2_bg);
        p.outline_icon("bluetooth", 28., 128., 16., if bt_on { 0xd8d8de } else { 0x9ca3af });

        p.text(&bt_name, 58., 122., 280., 18., 12.5, 0xf3f3f5, true);
        if self.quick_controls.radios_of_kind(crate::quick_controls::RadioKind::Bluetooth).len() > 1 {
            p.fill(58., 140., 180., 16., 4., 0x181b22);
            p.stroke(58., 140., 180., 16., 4., 0x2b2e3a, 1.0);
            p.text_ellipsis("Klik untuk ganti radio", 64., 141., 148., 14., 9.5, 0x9ca3af, false);
            if self.hit_enabled {
                self.hits.push(Hit { rect: layout.bt_selector, action: Action::QuickControlsSelectRadio(crate::quick_controls::RadioKind::Bluetooth, 0) });
            }
        }

        let bt_focus = self.quick_controls.focused_element == Some(crate::quick_controls::FOCUS_BT_TOGGLE);
        let bt_state = if bt_on { crate::quick_controls::RadioState::On } else { crate::quick_controls::RadioState::Off };
        crate::quick_controls::draw_toggle(p, layout.bt_toggle.left, layout.bt_toggle.top, 32., 18., bt_state, &bt_avail, bt_focus);
        if self.hit_enabled {
            self.hits.push(Hit { rect: layout.bt_toggle, action: Action::QuickControlsToggleRadio(crate::quick_controls::RadioKind::Bluetooth) });
        }

        p.text(&bt_status, 22., 156., 260., 16., 11., if bt_on { 0xd8d8de } else { 0x9ca3af }, false);
        p.text(&bt_sub, 22., 170., 320., 12., 9.5, 0x777781, false);

        p.fill(layout.bt_settings_btn.left, layout.bt_settings_btn.top, 88., 20., 5., 0x1c1c23);
        p.stroke(layout.bt_settings_btn.left, layout.bt_settings_btn.top, 88., 20., 5., 0x2d3242, 1.0);
        p.text("Pengaturan", layout.bt_settings_btn.left + 8., layout.bt_settings_btn.top + 3., 60., 14., 10., 0x9ca3af, false);
        p.outline_icon("right", layout.bt_settings_btn.left + 70., layout.bt_settings_btn.top + 4., 11., 0x9ca3af);
        if self.hit_enabled {
            self.hits.push(Hit { rect: layout.bt_settings_btn, action: Action::OpenSettingsPage("ms-settings:bluetooth") });
        }

        // Card 3: Brightness
        let card3_w = layout.card_display.right - layout.card_display.left;
        let card3_h = layout.card_display.bottom - layout.card_display.top;
        p.fill(layout.card_display.left, layout.card_display.top, card3_w, card3_h, 10., 0x12141a);
        p.stroke(layout.card_display.left, layout.card_display.top, card3_w, card3_h, 10., 0x222229, 1.0);

        let (disp_name, disp_percent, disp_target, disp_status, disp_sub, disp_unsupported) = if let Some(d) = self.quick_controls.selected_display() {
            (d.name.clone(), d.current_percent, d.target_percent, d.status_text.clone(), d.sub_status_text.clone(), d.kind == crate::quick_controls::DisplayKind::Unsupported)
        } else {
            ("Kecerahan".to_string(), 50, None, "Tidak ada monitor terdeteksi".to_string(), "".to_string(), true)
        };

        p.fill(22., 196., 28., 28., 7., 0x1b1e26);
        p.outline_icon("sun", 28., 202., 16., if disp_unsupported { 0x555966 } else { 0xd8d8de });

        p.text("Kecerahan", 58., 196., 70., 18., 12.5, 0xf3f3f5, true);

        // Display selector button
        p.fill(130., 196., 210., 18., 4., 0x181b22);
        p.stroke(130., 196., 210., 18., 4., 0x2b2e3a, 1.0);
        p.text_ellipsis(&disp_name, 136., 197., 198., 16., 10., 0x9ca3af, false);
        if self.hit_enabled {
            self.hits.push(Hit { rect: layout.display_selector, action: Action::QuickControlsSelectDisplay(0) });
        }

        let display_val = disp_target.unwrap_or(disp_percent);
        p.text_centered(&format!("{display_val}%"), 404., 196., 32., 18., 11., 0xffffff, 1.0, true);

        // Slider track
        let slider_track_w = 408.0;
        let slider_x = 22.0;
        let slider_y = 224.0;
        p.fill(slider_x, slider_y, slider_track_w, 6., 3., 0x252833);
        if !disp_unsupported {
            let fill_w = slider_track_w * (display_val as f32 / 100.0);
            p.fill(slider_x, slider_y, fill_w, 6., 3., 0xd8d8de);
            let thumb_x = slider_x + fill_w;
            p.fill(thumb_x - 7., slider_y + 3. - 7., 14., 14., 7., 0xffffff);
            p.stroke(thumb_x - 7., slider_y + 3. - 7., 14., 14., 7., 0xd8d8de, 1.5);
        }

        // Ticks
        p.text("0%", 22., 234., 24., 12., 9., 0x777781, false);
        p.text("25%", 120., 234., 24., 12., 9., 0x777781, false);
        p.text("50%", 220., 234., 24., 12., 9., 0x777781, false);
        p.text("75%", 320., 234., 24., 12., 9., 0x777781, false);
        p.text("100%", 406., 234., 26., 12., 9., 0x777781, false);

        if self.hit_enabled && !disp_unsupported {
            self.hits.push(Hit { rect: layout.brightness_slider_hit, action: Action::QuickBrightnessPointer });
        }

        p.text(&disp_status, 22., 250., 260., 14., 11., 0xf3f3f5, false);
        p.text(&disp_sub, 22., 264., 320., 12., 9.5, 0x777781, false);

        p.fill(layout.display_settings_btn.left, layout.display_settings_btn.top, 88., 20., 5., 0x1c1c23);
        p.stroke(layout.display_settings_btn.left, layout.display_settings_btn.top, 88., 20., 5., 0x2d3242, 1.0);
        p.text("Tampilan", layout.display_settings_btn.left + 12., layout.display_settings_btn.top + 3., 56., 14., 10., 0x9ca3af, false);
        p.outline_icon("right", layout.display_settings_btn.left + 70., layout.display_settings_btn.top + 4., 11., 0x9ca3af);
        if self.hit_enabled {
            self.hits.push(Hit { rect: layout.display_settings_btn, action: Action::OpenSettingsPage("ms-settings:display") });
        }

        // Feedback toast if present
        if let Some((ref msg, at)) = self.quick_controls.feedback_toast {
            if at.elapsed().as_secs_f32() < 4.0 {
                let toast_w = 400.0;
                let toast_x = (self.width - toast_w) * 0.5;
                let toast_y = self.height - 34.0;
                p.fill(toast_x, toast_y, toast_w, 24., 6., 0x231c14);
                p.stroke(toast_x, toast_y, toast_w, 24., 6., 0x5c3a12, 1.0);
                p.text_ellipsis(msg, toast_x + 8., toast_y + 4., toast_w - 16., 16., 10.5, 0xfde68a, false);
            }
        }
    }}
}
