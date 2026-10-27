//! Shared design tokens and the collapsed/expanded morph used by the notch shell.
//!
//! The shell has exactly two stable geometries: a compact pill and the full panel.
//! Everything between them is a 300 ms transition driven by `Morph`, which is only
//! ticked while a transition is running so the app never keeps a render loop alive.

// This module is a design scale, not a feature. UI-2 will migrate the remaining
// pages onto these tokens, so unused entries here are expected rather than bugs.
#![allow(dead_code)]

use std::time::{Duration, Instant};

/// One vector icon vocabulary for navigation, settings and compact status.
pub fn module_icon(module:&str)->&'static str{match module{"Musik"=>"music","Kalender"=>"calendar","Reminder"=>"bell","Clipboard"=>"clipboard","Catatan"=>"edit",_=>"bell"}}

/// Match the integer HWND position and the D2D origin in physical pixels.
pub fn panel_origin_y(bottom:bool,height:f32,scale:f32)->f32{
    let pad=(SHADOW_PAD*scale).round();
    if bottom{((CANVAS_HEIGHT*scale).floor()-pad)/scale-height}else{pad/scale}
}
/// Keep the HWND region one physical row above the attached contour; the monitor clips it.
pub fn attached_clip_top(scale:f32)->i32{((SHADOW_PAD*scale).round()as i32-1).max(0)}
pub fn window_top(bottom:bool,gap:i32,scale:f32,work_top:i32,work_bottom:i32,monitor_top:i32)->i32{
    let pad=(SHADOW_PAD*scale).round()as i32;
    if bottom{work_bottom-(CANVAS_HEIGHT*scale).floor()as i32-gap+pad}
    else{if gap==0{monitor_top-pad}else{work_top+gap-pad}}
}

// --- geometry -------------------------------------------------------------

pub const COLLAPSED_WIDTH: f32 = 300.;
pub const COLLAPSED_HEIGHT: f32 = 32.;
pub const EXPANDED_WIDTH: f32 = 460.;
pub const EXPANDED_HEIGHT: f32 = 266.;

// Dimensions from notch-mockup.html specification
pub const PLAYER_WIDTH: f32 = 460.;
pub const PLAYER_HEIGHT: f32 = 180.;
pub const LYRICS_WIDTH: f32 = 520.;
pub const LYRICS_HEIGHT: f32 = 180.;
pub const STRIP_WIDTH: f32 = 300.;
pub const STRIP_HEIGHT: f32 = 32.;
pub const CLOCK_WIDTH: f32 = 148.;
pub const CLOCK_HEIGHT: f32 = 36.;
pub const TINY_WIDTH: f32 = 50.;
pub const TINY_HEIGHT: f32 = 10.;

/// Fixed canvas dimensions. The composition surface and Direct3D swap chain buffers are
/// allocated once at this static size, eliminating all ResizeBuffers and SetWindowPos
/// stutter during fluid morph animations.
pub const CANVAS_WIDTH: f32 = 620.;
pub const CANVAS_HEIGHT: f32 = 420.;

/// Corner radius of the panel. The pill uses half its height, which makes it a true capsule.
pub const RADIUS_PANEL: f32 = 18.;
pub const RADIUS_CARD: f32 = 12.;
pub const RADIUS_CONTROL: f32 = 8.;
pub const RADIUS_CHIP: f32 = 10.;

/// Margin around the capsule reserved for the drop shadow, in logical pixels.
pub const SHADOW_PAD: f32 = 30.;

// --- spacing scale --------------------------------------------------------

pub const SPACE_1: f32 = 4.;
pub const SPACE_2: f32 = 8.;
pub const SPACE_3: f32 = 12.;
pub const SPACE_4: f32 = 16.;
pub const SPACE_5: f32 = 20.;
pub const SPACE_6: f32 = 24.;
pub const SPACE_8: f32 = 32.;

// --- type scale -----------------------------------------------------------

pub const TEXT_DISPLAY: f32 = 22.;
pub const TEXT_TITLE: f32 = 18.;
pub const TEXT_HEADING: f32 = 16.;
pub const TEXT_BODY: f32 = 13.;
pub const TEXT_LABEL: f32 = 12.;
pub const TEXT_CAPTION: f32 = 10.;

// --- motion ---------------------------------------------------------------

/// Snappy and fluid transition (~260 ms) matching Apple Dynamic Island and Nusa.
pub const MORPH_DURATION: Duration = Duration::from_millis(260);
/// Timer resolution while a transition runs (8 ms -> up to 120/144 Hz). Killed as soon as it settles.
pub const MORPH_TICK_MS: u32 = 8;
/// Cross-fade between tab pages; fast enough to feel responsive.
pub const TAB_FADE_DURATION: Duration = Duration::from_millis(220);

pub fn lerp(from: f32, to: f32, t: f32) -> f32 { from + (to - from) * t }

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    1. - (1. - t).powi(3)
}

/// Slight overshoot, used for width so the capsule feels springy rather than linear.
/// Tuned to peak around 1.8 %, which is a few pixels at panel width — enough to read
/// as a spring, not enough to look like the window is bouncing.
pub fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 0.7;
    const C3: f32 = C1 + 1.;
    let t = t.clamp(0., 1.);
    let u = t - 1.;
    1. + C3 * u * u * u + C1 * u * u
}

/// Interpolates width, height and corner radius between two shell geometries.
pub struct Morph {
    started: Instant,
    duration: Duration,
    from: (f32, f32, f32),
    to: (f32, f32, f32),
}

impl Morph {
    pub fn new(from: (f32, f32, f32), to: (f32, f32, f32)) -> Self {
        Self { started: Instant::now(), duration: MORPH_DURATION, from, to }
    }

    pub fn is_horizontal(&self) -> bool {
        (self.from.1 - self.to.1).abs() < 5.0
    }

    pub fn is_inter_tab(&self) -> bool {
        self.from.1 >= 100.0 && self.to.1 >= 100.0
    }

    pub fn from(&self) -> (f32, f32, f32) { self.from }
    pub fn to(&self) -> (f32, f32, f32) { self.to }

    /// Linear 0..1 progress. Content fades are driven by this, not by the eased value.
    pub fn raw(&self) -> f32 {
        (self.started.elapsed().as_secs_f32() / self.duration.as_secs_f32()).clamp(0., 1.)
    }

    pub fn finished(&self) -> bool { self.started.elapsed() >= self.duration }

    pub fn sample(&self) -> (f32, f32, f32) {
        let t = self.raw();
        let inter_tab = self.is_inter_tab();
        let (w_t, h_t, r_t) = if inter_tab {
            let ease = ease_out_cubic(t);
            (ease, ease, ease)
        } else {
            let expanding = self.to.1 > self.from.1;
            let w_t = if self.is_horizontal() {
                ease_out_cubic(t)
            } else if expanding {
                ease_out_back(t)
            } else {
                ease_out_cubic(t)
            };
            (w_t, ease_out_cubic(t), ease_out_cubic(t))
        };
        let width = lerp(self.from.0, self.to.0, w_t);
        let height = lerp(self.from.1, self.to.1, h_t);
        let radius = lerp(self.from.2, self.to.2, r_t);
        (width, height, radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_edge_has_no_pixel_gap_for_dpi_scale_or_stage(){
        for scale in [0.8,1.,1.1,1.25,1.35,1.5,1.6875,2.]{for monitor_top in [0,-1080]{for height in [10.,30.,180.,228.,346.,362.]{
            let win=window_top(false,0,scale,monitor_top+1,1080,monitor_top);
            let actual=win as f32+panel_origin_y(false,height,scale)*scale;
            assert_eq!(win+attached_clip_top(scale),monitor_top-1);
            assert!((actual-monitor_top as f32).abs()<0.001,"gap at scale {scale}, height {height}: {actual}");
            let bottom=window_top(true,0,scale,1,1080,0)as f32+(panel_origin_y(true,height,scale)+height)*scale;
            assert!((bottom-1080.).abs()<0.001,"bottom gap at {scale}: {bottom}");
        }}}
        assert_eq!(window_top(false,8,1.,1,1080,0)+panel_origin_y(false,30.,1.)as i32,9);
    }
    #[test]
    fn easing_endpoints_are_stable() {
        // f32 constants are not exact, so endpoints are compared with a tolerance.
        assert!(ease_out_cubic(0.).abs() < 1e-6);
        assert!((ease_out_cubic(1.) - 1.).abs() < 1e-6);
        assert!(ease_out_back(0.).abs() < 1e-6);
        assert!((ease_out_back(1.) - 1.).abs() < 1e-6);
        // Clamped outside the unit interval instead of extrapolating.
        assert_eq!(ease_out_cubic(-3.), 0.);
        assert_eq!(ease_out_cubic(9.), 1.);
    }

    #[test]
    fn overshoot_stays_subtle() {
        let peak = (0..=100).map(|i| ease_out_back(i as f32 / 100.)).fold(0.0_f32, f32::max);
        assert!(peak > 1.0, "width should overshoot to feel springy");
        assert!(peak < 1.03, "overshoot must stay subtle, got {peak}");
    }

    #[test]
    fn morph_settles_within_its_duration() {
        let morph = Morph::new((340., 52., 26.), (680., 620., 24.));
        // The first sample may already be a few milliseconds in, and f32 rounding can put the
        // easing a hair below zero, so the bounds get a small tolerance.
        let start = morph.sample();
        assert!((339.5..=680.).contains(&start.0), "starts inside the morph range, got {}", start.0);
        assert!((51.5..=620.).contains(&start.1), "height starts inside the morph range, got {}", start.1);
        // The clock is monotonic, so a finished morph must report the target exactly.
        std::thread::sleep(MORPH_DURATION + Duration::from_millis(20));
        assert!(morph.finished());
        let end = morph.sample();
        assert!((end.0 - 680.).abs() < 0.05, "width settles at the panel width, got {}", end.0);
        assert!((end.1 - 620.).abs() < 0.05, "height settles at the panel height, got {}", end.1);
        assert!((end.2 - 24.).abs() < 0.05, "radius settles at the panel radius, got {}", end.2);
        assert_eq!(morph.raw(), 1.);
    }
}
