use crate::privacy::{Coverage, PrivacySnapshot};
use crate::render::{self, Painter};
use crate::shell::{Action, Hit, Shell};
use crate::i18n::t;

pub const COLOR_MIC: u32 = 0xF5AA55;      // Amber
pub const COLOR_CAMERA: u32 = 0x5BD59A;   // Green
pub const COLOR_SHARE: u32 = 0xB9A0FF;    // Purple
pub const COLOR_RECORD: u32 = 0xFF7979;   // Red

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage0Geometry {
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

pub fn compute_stage0_geometry(snapshot: &PrivacySnapshot) -> Stage0Geometry {
    let mut active_count = 0;
    if snapshot.microphone.is_active() {
        active_count += 1;
    }
    if snapshot.camera.is_active() {
        active_count += 1;
    }
    if snapshot.screen_share.is_active() {
        active_count += 1;
    }
    if snapshot.screen_recording.is_active() {
        active_count += 1;
    }

    if active_count == 0 {
        Stage0Geometry {
            width: 50.0,
            height: 10.0,
            radius: 5.0,
        }
    } else if active_count == 1 {
        Stage0Geometry {
            width: 88.0,
            height: 18.0,
            radius: 9.0,
        }
    } else {
        Stage0Geometry {
            width: 116.0,
            height: 18.0,
            radius: 9.0,
        }
    }
}

pub fn compute_privacy_rail_width(snapshot: &PrivacySnapshot) -> f32 {
    let mut active_count = 0;
    if snapshot.microphone.is_active() {
        active_count += 1;
    }
    if snapshot.camera.is_active() {
        active_count += 1;
    }
    if snapshot.screen_share.is_active() {
        active_count += 1;
    }
    if snapshot.screen_recording.is_active() {
        active_count += 1;
    }

    if active_count == 0 {
        0.0
    } else {
        // Spec 7.2: Slot 18 DIP per icon, gap 4 DIP
        active_count as f32 * 18.0 + (active_count - 1) as f32 * 4.0
    }
}

impl Shell {
    /// Renders active privacy dots inside the Stage 0 capsule (88x18 or 116x18).
    pub(super) unsafe fn draw_privacy_stage0(
        &mut self,
        p: &mut Painter,
        width: f32,
        height: f32,
    ) {
        unsafe {
            if !self.settings.privacy_enabled || !self.privacy_snapshot.any_active() {
                return;
            }

            let mut active_items = Vec::new();
            if self.privacy_snapshot.microphone.is_active() {
                active_items.push(("mic", COLOR_MIC));
            }
            if self.privacy_snapshot.camera.is_active() {
                active_items.push(("camera", COLOR_CAMERA));
            }
            if self.privacy_snapshot.screen_share.is_active() {
                active_items.push(("share", COLOR_SHARE));
            }
            if self.privacy_snapshot.screen_recording.is_active() {
                active_items.push(("record", COLOR_RECORD));
            }

            let count = active_items.len();
            if count == 0 {
                return;
            }

            let cy = height * 0.5;
            let spacing = 16.0;
            let total_w = (count - 1) as f32 * spacing;
            let start_x = (width - total_w) * 0.5;

            for (idx, (icon, color)) in active_items.iter().enumerate() {
                let cx = start_x + idx as f32 * spacing;
                // Draw glowing dot
                p.fill_alpha(cx - 3.5, cy - 3.5, 7.0, 7.0, 3.5, *color, 0.95);
                p.fill_alpha(cx - 5.5, cy - 5.5, 11.0, 11.0, 5.5, *color, 0.25);
                // Draw subtle outline icon if enough space
                if height >= 16.0 {
                    p.outline_icon(icon, cx - 4.0, cy - 4.0, 8.0, 0x111116);
                }
            }

            if self.hit_enabled {
                self.hits.push(Hit {
                    rect: render::rect(0.0, 0.0, width, height),
                    action: Action::PrivacyDetail,
                });
            }
        }
    }

    /// Renders the persistent Privacy Rail in Stage 1 without overlapping clock or controls.
    pub(super) unsafe fn draw_privacy_rail(
        &mut self,
        p: &mut Painter,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        unsafe {
            if !self.settings.privacy_enabled || !self.privacy_snapshot.any_active() {
                return;
            }

            let mut active_items = Vec::new();
            if self.privacy_snapshot.microphone.is_active() {
                active_items.push(("mic", COLOR_MIC));
            }
            if self.privacy_snapshot.camera.is_active() {
                active_items.push(("camera", COLOR_CAMERA));
            }
            if self.privacy_snapshot.screen_share.is_active() {
                active_items.push(("share", COLOR_SHARE));
            }
            if self.privacy_snapshot.screen_recording.is_active() {
                active_items.push(("record", COLOR_RECORD));
            }

            let hovered = self.pointer.is_some_and(|(px, py)| {
                px >= x && px <= x + w && py >= y && py <= y + h
            });

            let mut cur_x = x;
            for (icon, color) in &active_items {
                let slot_w = 18.0;
                let slot_h = 18.0;
                let slot_y = y + (h - slot_h) * 0.5;

                // Pill background
                p.fill(cur_x, slot_y, slot_w, slot_h, 6.0, if hovered { 0x2a2a32 } else { 0x1e1e24 });
                p.stroke(cur_x, slot_y, slot_w, slot_h, 6.0, if hovered { *color } else { 0x33333d }, 1.0);

                // Icon
                p.outline_icon(icon, cur_x + 3.0, slot_y + 3.0, 12.0, *color);

                cur_x += slot_w + 4.0;
            }

            if self.hit_enabled {
                self.hits.push(Hit {
                    rect: render::rect(x - 2.0, y - 2.0, w + 4.0, h + 4.0),
                    action: Action::PrivacyDetail,
                });
            }
        }
    }

    /// Renders the full Privacy Detail panel in Stage 2 (560x320 panel).
    pub(super) unsafe fn draw_privacy_detail(&mut self, p: &mut Painter) {
        unsafe {
            let lang = self.settings.language.clone();
            let fg = 0xf3f3f5;
            let muted = 0x92929d;
            let card_bg = 0x16161b;
            let card_border = 0x25252d;

            // Header: Shield Icon, Title, and Subtitle
            p.outline_icon("shield", 24.0, 20.0, 20.0, 0x69d2e5);
            p.text(t("privacy.title", &lang), 52.0, 18.0, 200.0, 24.0, 16.0, fg, false);
            p.text(t("privacy.subtitle", &lang), 52.0, 42.0, 360.0, 16.0, 10.0, muted, false);

            // Back button
            let back_hover = self.pointer.is_some_and(|(px, py)| {
                px >= 460.0 && px <= 540.0 && py >= 18.0 && py <= 46.0
            });
            p.fill(460.0, 18.0, 80.0, 28.0, 7.0, if back_hover { 0x292932 } else { 0x1c1c23 });
            p.outline_icon("left", 468.0, 25.0, 14.0, muted);
            p.text_centered(t("settings.back", &lang), 482.0, 18.0, 52.0, 28.0, 11.0, fg, 1.0, false);
            if self.hit_enabled {
                self.hits.push(Hit {
                    rect: render::rect(460.0, 18.0, 80.0, 28.0),
                    action: Action::PrivacyClose,
                });
            }

            // Horizontal divider below header
            p.fill(24.0, 68.0, 512.0, 1.0, 0.0, 0x222228);

            // 4 Capabilities Rows: Microphone, Camera, Screen Share, Screen Recording
            let capabilities = [
                (
                    "mic",
                    COLOR_MIC,
                    t("privacy.mic_title", &lang),
                    &self.privacy_snapshot.microphone,
                    t("privacy.mic_in_use", &lang),
                    t("privacy.mic_note", &lang),
                    Some(Action::OpenWindowsMicSettings),
                ),
                (
                    "camera",
                    COLOR_CAMERA,
                    t("privacy.camera_title", &lang),
                    &self.privacy_snapshot.camera,
                    t("privacy.cam_detected", &lang),
                    t("privacy.experimental_note", &lang),
                    Some(Action::OpenWindowsCameraSettings),
                ),
                (
                    "screen_share",
                    COLOR_SHARE,
                    t("privacy.share_title", &lang),
                    &self.privacy_snapshot.screen_share,
                    t("privacy.share_active", &lang),
                    t("privacy.unavailable", &lang),
                    None,
                ),
                (
                    "record",
                    COLOR_RECORD,
                    t("privacy.record_title", &lang),
                    &self.privacy_snapshot.screen_recording,
                    t("privacy.record_active", &lang),
                    t("privacy.unavailable", &lang),
                    None,
                ),
            ];

            let start_y = 78.0;
            let card_h = 50.0;
            let card_gap = 8.0;

            for (idx, (icon, color, title, state, active_desc, note_desc, open_action)) in capabilities.iter().enumerate() {
                let y = start_y + idx as f32 * (card_h + card_gap);
                let is_active = state.is_active();
                let is_unknown = state.is_unknown();

                p.fill(24.0, y, 512.0, card_h, 8.0, card_bg);
                p.stroke(24.0, y, 512.0, card_h, 8.0, if is_active { *color } else { card_border }, 1.0);

                // Icon pill
                let icon_bg = if is_active { *color } else { 0x22222a };
                let icon_fg = if is_active { 0x111115 } else { muted };
                p.fill(34.0, y + 10.0, 30.0, 30.0, 7.0, icon_bg);
                p.outline_icon(icon, 41.0, y + 17.0, 16.0, icon_fg);

                // Title & Status
                let title_text = if is_active { *active_desc } else { *title };
                p.text(title_text, 74.0, y + 8.0, 240.0, 18.0, 12.0, fg, false);

                // Applications / Note line
                let sub_text = if is_active && self.settings.privacy_show_app_names && !state.applications.is_empty() {
                    state.applications.join(", ")
                } else if is_active && state.applications.is_empty() {
                    t("privacy.app_unknown", &lang).to_string()
                } else if is_unknown {
                    t("privacy.status_unknown", &lang).to_string()
                } else {
                    (*note_desc).to_string()
                };
                p.text_ellipsis(&sub_text, 74.0, y + 27.0, 310.0, 16.0, 10.0, muted, false);

                // Status badge on right side
                let badge_w = 90.0;
                let badge_x = 436.0;
                let (badge_text, badge_fg, badge_bg) = if is_active {
                    ("ACTIVE", *color, 0x1e2420)
                } else if state.coverage == Coverage::Unsupported {
                    ("UNSUPPORTED", 0x777782, 0x1b1b22)
                } else {
                    ("INACTIVE", 0x777782, 0x1b1b22)
                };
                p.fill(badge_x, y + 15.0, badge_w, 20.0, 5.0, badge_bg);
                p.text_centered(badge_text, badge_x, y + 15.0, badge_w, 20.0, 9.5, badge_fg, 1.0, true);

                // Settings click button if applicable
                if let Some(action) = open_action {
                    let btn_hover = self.pointer.is_some_and(|(px, py)| {
                        px >= badge_x && px <= badge_x + badge_w && py >= y + 15.0 && py <= y + 35.0
                    });
                    if btn_hover {
                        p.stroke(badge_x, y + 15.0, badge_w, 20.0, 5.0, fg, 1.0);
                    }
                    if self.hit_enabled {
                        self.hits.push(Hit {
                            rect: render::rect(badge_x, y + 15.0, badge_w, 20.0),
                            action: action.clone(),
                        });
                    }
                }
            }

            // Coverage footer notice
            p.text_ellipsis(
                t("privacy.coverage_footer", &lang),
                26.0,
                310.0,
                508.0,
                18.0,
                9.5,
                0x666672,
                false,
            );
        }
    }

    /// Renders the Privacy Settings subpage.
    pub(super) unsafe fn draw_settings_privacy(&mut self, p: &mut Painter) {
        unsafe {
            let lang = self.settings.language.clone();
            let fg = 0xf3f3f5;
            let muted = 0x92929d;

            // Subpage Header
            p.outline_icon("shield", 196.0, 68.0, 18.0, 0x69d2e5);
            p.text(t("privacy.settings_title", &lang), 222.0, 66.0, 240.0, 25.0, 16.0, fg, false);
            p.text(t("privacy.settings_desc", &lang), 196.0, 92.0, 346.0, 16.0, 10.5, muted, false);

            // Row 1: privacy_enabled
            self.st_row_compact(p, 120.0, t("privacy.enabled", &lang), t("privacy.enabled_desc", &lang));
            self.st_toggle(p, 512.0, 131.0, self.settings.privacy_enabled, Action::Toggle("privacy_enabled"), true);

            // Row 2: privacy_show_fullscreen
            self.st_row_compact(p, 168.0, t("privacy.fullscreen", &lang), t("privacy.fullscreen_desc", &lang));
            self.st_toggle(p, 512.0, 179.0, self.settings.privacy_show_fullscreen, Action::Toggle("privacy_show_fullscreen"), self.settings.privacy_enabled);

            // Row 3: privacy_show_app_names
            self.st_row_compact(p, 216.0, t("privacy.app_names", &lang), t("privacy.app_names_desc", &lang));
            self.st_toggle(p, 512.0, 227.0, self.settings.privacy_show_app_names, Action::Toggle("privacy_show_app_names"), self.settings.privacy_enabled);

            // Return button to General Settings
            let back_btn_hover = self.pointer.is_some_and(|(px, py)| {
                px >= 196.0 && px <= 300.0 && py >= 280.0 && py <= 310.0
            });
            p.fill(196.0, 280.0, 104.0, 30.0, 7.0, if back_btn_hover { 0x2a2a34 } else { 0x1e1e26 });
            p.outline_icon("left", 204.0, 287.0, 14.0, muted);
            p.text_centered(t("settings.back", &lang), 220.0, 280.0, 72.0, 30.0, 11.5, fg, 1.0, false);
            if self.hit_enabled {
                self.hits.push(Hit {
                    rect: render::rect(196.0, 280.0, 104.0, 30.0),
                    action: Action::ClosePrivacySettings,
                });
            }
        }
    }
}
