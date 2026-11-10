//! Shared design tokens and the collapsed/expanded morph used by the notch shell.
//!
//! The shell has exactly two stable geometries: a compact pill and the full panel.
//! Everything between them is a 300 ms transition driven by `Morph`, which is only
//! ticked while a transition is running so the app never keeps a render loop alive.

// This module is a design scale, not a feature. UI-2 will migrate the remaining
// pages onto these tokens, so unused entries here are expected rather than bugs.
#![allow(dead_code)]

use std::time::{Duration, Instant};
use std::cell::{Cell, RefCell};
use crate::motion::MotionCoordinator;

/// One vector icon vocabulary for navigation, settings and compact status.
pub fn module_icon(module:&str)->&'static str{match module{"Musik"=>"music","Kalender"=>"calendar","Reminder"=>"bell","Clipboard"=>"clipboard","Catatan"=>"edit","Cuaca"=>"cloud","Daya"=>"zap",_=>"bell"}}

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
pub const CANVAS_WIDTH: f32 = 660.;
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

/// Snappy and fluid transition (~600 ms) governed by continuous analytical spring physics.
pub const MORPH_DURATION: Duration = Duration::from_millis(600);
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

// --- F2 sweep aksen Nyala (mockup notifikasi-reminder-mockups.html:125-141) ---

/// Satu siklus sweep pita aksen pill Stage 1 (mockup `animation:sweep 3s ... infinite`).
pub const ACCENT_SWEEP_PERIOD_MS: u64 = 3000;
/// Decimasi repaint sweep pada timer morph 8 ms: ~40 fps cukup untuk pita lambat
/// 3 dtk sekaligus membatasi biaya render kontinyu (philosophy ui.rs: "never keeps
/// a render loop alive" — timer morph hanya hidup selama animasi berjalan).
pub const ACCENT_SWEEP_FRAME_MS: u64 = 25;
/// Rentang gerak pusat pita, sebagai pecahan lebar pill. Turunan CSS mockup:
/// background-size 240% (pita 2.4w), posisi 170% -> -70% memetakan offset
/// `p/100*(w-2.4w) = -1.4w*p/100`; pinggir gradasi (24%..68% dari 2.4w) bergeser
/// dari [-1.804w,-0.748w] ke [1.556w,2.612w], jadi pusatnya -1.276w -> 2.084w.
pub const ACCENT_SWEEP_CENTER_FROM: f32 = -1.276;
pub const ACCENT_SWEEP_CENTER_TO: f32 = 2.084;
/// Setengah lebar pita sebagai pecahan lebar pill (68% - 24% = 44% dari 2.4w).
pub const ACCENT_SWEEP_HALF: f32 = 0.528;

/// Easing persis mockup: `cubic-bezier(.55,.1,.45,.9)`. Solusi bisection atas
/// kurva parameter (cukup untuk 1 panggilan per frame).
pub fn ease_accent_sweep(t: f32) -> f32 {
    const X1: f32 = 0.55; const Y1: f32 = 0.10;
    const X2: f32 = 0.45; const Y2: f32 = 0.90;
    let t = t.clamp(0., 1.);
    if t <= 0. || t >= 1. { return t; }
    let bez=|a: f32, b: f32, u: f32| 3.*u*(1.-u)*(1.-u)*a + 3.*u*u*(1.-u)*b + u*u*u;
    let mut lo = 0.; let mut hi = 1.;
    for _ in 0..24 {
        let mid = 0.5 * (lo + hi);
        if bez(X1, X2, mid) < t { lo = mid; } else { hi = mid; }
    }
    let u = 0.5 * (lo + hi);
    bez(Y1, Y2, u)
}

/// Continuous spring motion coordinator for shell width, height, corner radius, and ear fillets.
#[derive(Clone)]
pub struct Morph {
    from: (f32, f32, f32),
    to: (f32, f32, f32),
    pub coordinator: RefCell<MotionCoordinator>,
    last_tick: Cell<Instant>,
}

impl Morph {
    /// A resting shell may already have changed its logical stage. Start at the presented frame.
    pub fn from_presented(frame:Option<crate::glass::FrameContour>,to:(f32,f32,f32),ear:f32)->Self{
        let (from,from_ear)=frame.map(|c|((c.width,c.height,c.radius),c.ear)).unwrap_or((to,ear));
        Self::new_with_ear(from,from_ear,to,ear)
    }
    pub fn new(from: (f32, f32, f32), to: (f32, f32, f32)) -> Self {
        let ear_from = if from.1 >= 24. { 12. } else if from.1 <= 10. { 6. } else { 8. };
        let ear_to = if to.1 >= 24. { 12. } else if to.1 <= 10. { 6. } else { 8. };
        Self::new_with_ear(from, ear_from, to, ear_to)
    }

    pub fn new_with_ear(from: (f32, f32, f32), ear_from: f32, to: (f32, f32, f32), ear_to: f32) -> Self {
        let mut coordinator = MotionCoordinator::new(from.0, from.1, from.2, ear_from);
        coordinator.retarget(to.0, to.1, to.2, ear_to);
        Self {
            from,
            to,
            coordinator: RefCell::new(coordinator),
            last_tick: Cell::new(Instant::now()),
        }
    }

    /// Retargets the continuous spring motion preserving instantaneous position and velocity.
    pub fn retarget(&mut self, to: (f32, f32, f32), ear: f32) {
        let current = self.sample();
        self.from = current;
        self.to = to;
        self.coordinator.borrow_mut().retarget(to.0, to.1, to.2, ear);
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.coordinator.borrow_mut().set_reduced_motion(reduced);
    }

    pub fn is_horizontal(&self) -> bool {
        (self.from.1 - self.to.1).abs() < 5.0
    }

    pub fn is_inter_tab(&self) -> bool {
        self.from.1 >= 100.0 && self.to.1 >= 100.0
    }

    pub fn from(&self) -> (f32, f32, f32) { self.from }
    pub fn to(&self) -> (f32, f32, f32) { self.to }

    pub fn ear(&self) -> f32 {
        self.sample();
        self.coordinator.borrow().ear.current
    }

    /// Relative travel (0..1) on the dominant moving axis, independent of panel dimensions.
    /// Fade timing follows the geometry actually rendered, including interrupted retargets.
    pub fn raw(&self) -> f32 {
        let current=self.sample();
        if self.coordinator.borrow().is_settled(){return 1.}
        let axes=[(self.from.0,self.to.0,current.0),(self.from.1,self.to.1,current.1),(self.from.2,self.to.2,current.2)];
        let (from,to,current)=axes.into_iter().max_by(|a,b|(a.1-a.0).abs().total_cmp(&(b.1-b.0).abs())).unwrap();
        if (to-from).abs()<=f32::EPSILON{1.}else{((current-from)/(to-from)).clamp(0.,1.)}
    }

    pub fn finished(&self) -> bool {
        self.sample();
        self.coordinator.borrow().is_settled()
    }

    pub fn sample(&self) -> (f32, f32, f32) {
        let now = Instant::now();
        let last = self.last_tick.get();
        let dt = now.duration_since(last).as_secs_f32();
        if dt > 0.0 {
            self.coordinator.borrow_mut().step(dt);
            self.last_tick.set(now);
        }
        let (w, h, r, _) = self.coordinator.borrow().current_values();
        (w, h, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_is_relative_to_actual_travel_for_arbitrary_panel_sizes(){
        for scale in [0.5,1.,2.5]{
            let from=(413.*scale,79.*scale,17.*scale);let to=(729.*scale,347.*scale,29.*scale);
            let morph=Morph::new(from,to);morph.coordinator.borrow_mut().step(0.1);
            let current=morph.sample();let expected=((current.0-from.0)/(to.0-from.0)).clamp(0.,1.);
            assert!((morph.raw()-expected).abs()<0.02,"progress must follow rendered travel, not elapsed time");
        }
    }
    #[test]
    fn resting_stage_transitions_animate_from_the_last_presented_shape(){
        for (from,to) in [((340.,30.,15.),(560.,300.,24.)),((560.,300.,24.),(340.,30.,15.))]{
            let frame=crate::glass::FrameContour{width:from.0,height:from.1,radius:from.2,ear:12.,round_top:false,origin_dip:(0.,0.),scale:1.};
            let morph=Morph::from_presented(Some(frame),to,12.);
            let start=morph.sample();assert!((start.1-from.1).abs()<0.5,"transition jumped from {from:?} to {start:?}");
            morph.coordinator.borrow_mut().step(0.10);let mid=morph.coordinator.borrow().current_values();
            assert!(mid.1>30.5&&mid.1<299.5,"must show intermediate sizes: {mid:?}");
            morph.coordinator.borrow_mut().step(2.);assert!((morph.sample().1-to.1).abs()<0.1);
        }
    }

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
        std::thread::sleep(Duration::from_millis(850));
        assert!(morph.finished());
        let end = morph.sample();
        assert!((end.0 - 680.).abs() < 0.05, "width settles at the panel width, got {}", end.0);
        assert!((end.1 - 620.).abs() < 0.05, "height settles at the panel height, got {}", end.1);
        assert!((end.2 - 24.).abs() < 0.05, "radius settles at the panel radius, got {}", end.2);
        assert_eq!(morph.raw(), 1.);
    }
}

/// Shared compact footer. Desktop: no battery group at all. Charging keeps the
/// orange bolt glyph (Stage1 0.0.2.2 order: time · separator · bolt · percent);
/// the bolt is emitted as a separate draw so it can use the orange fill.
pub fn stage1_footer(time:&str,battery:Option<(u32,bool)>,tws:Option<Option<u8>>)->String{
    let mut out=time.to_string();
    if let Some(tws)=tws{out.push_str(&format!(" • 🎧{}",tws.map(|p|format!("{p}%")).unwrap_or_default()));}
    if let Some((percent,charging))=battery{out.push_str(&format!(" • {}{percent}%",if charging{"⚡"}else if percent<=20{"🪫"}else{"🔋"}));}
    out
}

/// Micro layout for the 0.0.2.2 Stage 1 idle/calendar bar (mockup 024).
/// Logical coordinates; `icon_x` is the content left edge and `right_x` the
/// chevron hit-box left edge (kept separate from the chevron glyph).
pub const STAGE1_ICON_SIZE: f32 = 12.;
pub const STAGE1_GAP: f32 = 6.;
pub const STAGE1_DATE_GAP: f32 = 5.;
pub const STAGE1_SEPARATOR_W: f32 = 1.4;
pub const STAGE1_INFO_GAP: f32 = 7.;
pub const STAGE1_TEXT_SIZE: f32 = 12.;
pub const STAGE1_DOT_RADIUS: f32 = 1.6;
pub const STAGE1_BOLT_W: f32 = 8.;
pub const STAGE1_BOLT_H: f32 = 12.;
pub const STAGE1_CHEVRON_HIT_W: f32 = 20.;

/// Vertices of a filled 6-point lightning bolt (zap glyph) within bounding box `(bx, by, bw, bh)`.
/// Features 180° rotational symmetry and distinct horizontal teeth forming a true lightning step.
pub fn power_bolt_polygon(bx: f32, by: f32, bw: f32, bh: f32) -> [(f32, f32); 6] {
    [
        (bx + bw * 0.62, by),
        (bx + bw * 0.08, by + bh * 0.54),
        (bx + bw * 0.50, by + bh * 0.54),
        (bx + bw * 0.38, by + bh),
        (bx + bw * 0.92, by + bh * 0.46),
        (bx + bw * 0.50, by + bh * 0.46),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage1Layout {
    pub icon_x: f32,
    pub date_x: f32,
    pub separator_x: f32,
    pub time_x: f32,
    pub dot_x: f32,
    pub bolt_x: f32,
    pub battery_x: f32,
    pub chevron_hit_x: f32,
    pub chevron_glyph_x: f32,
    pub row_height: f32,
}

/// Pure, side-effect-free layout used by both the painter and the tests.
/// Returns `None` when the date/time/battery row cannot fit between `left_x`
/// and the chevron hit box; the caller then falls back to the existing
/// truncated single-string footer instead of hard-coding sample data.
pub fn stage1_layout(
    left_x: f32,
    chevron_hit_x: f32,
    date_w: f32,
    time_w: f32,
    battery_w: f32,
    date_gap: f32,
    info_gap: f32,
    row_height: f32,
) -> Option<Stage1Layout> {
    let has_battery = battery_w > 0.;
    // Total must include BOTH date_gap uses: dot→bolt and bolt→percent. Counting only one
    // underestimates the true content end by one date_gap (Validator counterexample: overlap 3.6 DIP).
    let battery_extra = if has_battery {
        info_gap + STAGE1_DOT_RADIUS * 2. + STAGE1_BOLT_W + date_gap * 2. + battery_w
    } else {
        0.
    };
    let total = STAGE1_ICON_SIZE
        + date_gap
        + date_w
        + info_gap
        + STAGE1_SEPARATOR_W
        + info_gap
        + time_w
        + battery_extra;
    let available = chevron_hit_x - left_x;
    if total > available {
        return None;
    }
    let icon_x = left_x;
    let date_x = icon_x + STAGE1_ICON_SIZE + date_gap;
    let separator_x = date_x + date_w + info_gap;
    let time_x = separator_x + STAGE1_SEPARATOR_W + info_gap;
    let dot_x = time_x + time_w + info_gap + STAGE1_DOT_RADIUS;
    let bolt_x = dot_x + STAGE1_DOT_RADIUS + date_gap;
    let battery_x = bolt_x + STAGE1_BOLT_W + date_gap;
    let chevron_glyph_x = chevron_hit_x + STAGE1_CHEVRON_HIT_W * 0.5;
    Some(Stage1Layout {
        icon_x,
        date_x,
        separator_x,
        time_x,
        dot_x,
        bolt_x,
        battery_x,
        chevron_hit_x,
        chevron_glyph_x,
        row_height,
    })
}

/// Presentation state for system battery in compact/idle Stage 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerPresentation {
    Unavailable,
    Charging(u32),
    Low(u32),
    Normal(u32),
}

impl PowerPresentation {
    pub fn new(percentage: Option<u32>, is_charging: bool) -> Self {
        match percentage {
            Some(pct) => {
                if is_charging {
                    PowerPresentation::Charging(pct)
                } else if pct <= 20 {
                    PowerPresentation::Low(pct)
                } else {
                    PowerPresentation::Normal(pct)
                }
            }
            None => PowerPresentation::Unavailable,
        }
    }

    pub fn from_battery(b: Option<&crate::system::BatteryInfo>) -> Self {
        match b {
            Some(b) if b.has_battery => Self::new(Some(b.percentage as u32), b.is_charging),
            _ => PowerPresentation::Unavailable,
        }
    }
}

/// Presentation state for TWS earbuds in compact/idle Stage 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TwsPresentation {
    Absent,
    Known(u8),
    Unknown,
}

impl TwsPresentation {
    pub fn new(battery: Option<Option<u8>>) -> Self {
        match battery {
            Some(Some(pct)) => TwsPresentation::Known(pct),
            Some(None) => TwsPresentation::Unknown,
            None => TwsPresentation::Absent,
        }
    }

    pub fn from_tws(t: Option<&crate::system::TwsInfo>) -> Self {
        match t {
            Some(t) => Self::new(Some(t.battery)),
            None => TwsPresentation::Absent,
        }
    }
}

/// Layout status of the idle clock bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IdlePlanStatus {
    Full,
    DateTrimmed,
    TooNarrow,
}

/// Comprehensive plan for the Stage 1 0.0.2.2 idle/calendar bar (mockup 024).
/// Single source of truth for both geometry calculation and native rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct IdleClockPlan {
    pub status: IdlePlanStatus,
    pub width: f32,
    pub height: f32,
    pub date_rect: (f32, f32, f32, f32),
    pub date_trimmed: bool,
    pub separator_rect: (f32, f32, f32, f32),
    pub time_rect: (f32, f32, f32, f32),
    // Optional TWS group
    pub tws_dot: Option<(f32, f32, f32)>, // cx, cy, r
    pub tws_glyph_rect: Option<(f32, f32, f32, f32)>,
    pub tws_label_rect: Option<(f32, f32, f32, f32)>,
    pub tws_label: String,
    // Optional Power group
    pub power_dot: Option<(f32, f32, f32)>,
    pub power_bolt_rect: Option<(f32, f32, f32, f32)>,
    pub power_glyph_rect: Option<(f32, f32, f32, f32)>,
    pub power_glyph: &'static str,
    pub power_label_rect: Option<(f32, f32, f32, f32)>,
    pub power_label: String,
    // Right slots
    pub privacy_slot: Option<(f32, f32, f32, f32)>,
    pub preset_slot: Option<(f32, f32, f32, f32)>,
    pub chevron_hit: (f32, f32, f32, f32),
    pub chevron_glyph_x: f32,
    // Interactive bounds
    pub calendar_hit: Option<(f32, f32, f32, f32)>,
    pub content_end: f32,
    pub content_limit: f32,
}

pub fn plan_idle_clock(
    width: f32,
    height: f32,
    _date_text: &str,
    date_w: f32,
    _time_text: &str,
    time_w: f32,
    power: PowerPresentation,
    power_glyph_w: f32,
    power_percent_w: f32,
    tws: TwsPresentation,
    tws_glyph_w: f32,
    tws_label_w: f32,
    privacy_rail_w: f32,
    preset_active: bool,
) -> IdleClockPlan {
    let tws_present = matches!(tws, TwsPresentation::Known(_) | TwsPresentation::Unknown);
    let tws_extra = if tws_present {
        STAGE1_INFO_GAP + STAGE1_DOT_RADIUS * 2. + STAGE1_DATE_GAP + tws_glyph_w + STAGE1_DATE_GAP + tws_label_w
    } else {
        0.
    };

    let power_present = matches!(power, PowerPresentation::Charging(_) | PowerPresentation::Low(_) | PowerPresentation::Normal(_));
    let effective_power_glyph_w = if matches!(power, PowerPresentation::Charging(_)) {
        STAGE1_BOLT_W
    } else {
        power_glyph_w
    };
    let power_extra = if power_present {
        STAGE1_INFO_GAP + STAGE1_DOT_RADIUS * 2. + STAGE1_DATE_GAP + effective_power_glyph_w + STAGE1_DATE_GAP + power_percent_w
    } else {
        0.
    };

    let has_secondary = tws_present || power_present || preset_active || privacy_rail_w > 0.;

    let left_x = 10.;
    // Tanpa ikon kalender: tanggal mulai tepat di tepi kiri konten pill.
    let date_x = left_x;

    let fixed_without_date = left_x
        + STAGE1_INFO_GAP
        + STAGE1_SEPARATOR_W
        + STAGE1_INFO_GAP
        + time_w;

    let full_content_end = fixed_without_date + date_w + tws_extra + power_extra;

    let chevron_hit_x = if has_secondary {
        width - 24.
    } else {
        (full_content_end + 8.0).min(width - 24.0)
    };
    let chevron_hit_y = (height - 20.) * 0.5;
    let chevron_hit = (chevron_hit_x, chevron_hit_y, 20., 20.);
    let chevron_glyph_x = chevron_hit_x + 10.;
    let mut cursor_right = chevron_hit_x - 2. - STAGE1_GAP;

    let preset_slot = if preset_active {
        let slot_w = 36.;
        let slot_x = cursor_right - slot_w;
        cursor_right = slot_x - STAGE1_GAP;
        Some((slot_x + 3., (height - 24.) * 0.5, 30., 24.))
    } else {
        None
    };

    let privacy_slot = if privacy_rail_w > 0. {
        let pad = 2.;
        let slot_w = privacy_rail_w + pad * 2.;
        let slot_x = cursor_right - slot_w;
        cursor_right = slot_x - 10.;
        Some((slot_x + pad, (height - 18.) * 0.5, privacy_rail_w, 18.))
    } else {
        None
    };

    let content_limit = cursor_right;

    let (status, effective_date_w, date_trimmed) = if full_content_end <= content_limit {
        (IdlePlanStatus::Full, date_w, false)
    } else {
        let fixed_end = fixed_without_date + tws_extra + power_extra;
        let date_budget = content_limit - fixed_end;
        if date_budget >= 16. {
            (IdlePlanStatus::DateTrimmed, date_budget, true)
        } else {
            (IdlePlanStatus::TooNarrow, date_w, false)
        }
    };

    let date_rect = (date_x, 0., effective_date_w, height);
    let separator_x = date_x + effective_date_w + STAGE1_INFO_GAP;
    let separator_rect = (separator_x, (height - 12.) * 0.5, STAGE1_SEPARATOR_W, 12.);
    let time_x = separator_x + STAGE1_SEPARATOR_W + STAGE1_INFO_GAP;
    let time_rect = (time_x, 0., time_w, height);

    let mut cursor = time_x + time_w;

    let (tws_dot, tws_glyph_rect, tws_label_rect, tws_label) = if tws_present {
        let dot_cx = cursor + STAGE1_INFO_GAP + STAGE1_DOT_RADIUS;
        let dot_cy = height * 0.5;
        let dot_r = STAGE1_DOT_RADIUS;
        let dot_end = dot_cx + dot_r;
        let glyph_x = dot_end + STAGE1_DATE_GAP;
        let label_x = glyph_x + tws_glyph_w + STAGE1_DATE_GAP;
        cursor = label_x + tws_label_w;
        let label = match tws {
            TwsPresentation::Known(pct) => format!("{pct}%"),
            TwsPresentation::Unknown => "—".to_string(),
            TwsPresentation::Absent => String::new(),
        };
        (
            Some((dot_cx, dot_cy, dot_r)),
            Some((glyph_x, 0., tws_glyph_w, height)),
            Some((label_x, 0., tws_label_w, height)),
            label,
        )
    } else {
        (None, None, None, String::new())
    };

    let (power_dot, power_bolt_rect, power_glyph_rect, power_glyph, power_label_rect, power_label) = if power_present {
        let dot_cx = cursor + STAGE1_INFO_GAP + STAGE1_DOT_RADIUS;
        let dot_cy = height * 0.5;
        let dot_r = STAGE1_DOT_RADIUS;
        let dot_end = dot_cx + dot_r;
        let glyph_x = dot_end + STAGE1_DATE_GAP;
        let label_x = glyph_x + effective_power_glyph_w + STAGE1_DATE_GAP;
        cursor = label_x + power_percent_w;
        let pct = match power {
            PowerPresentation::Charging(p) | PowerPresentation::Low(p) | PowerPresentation::Normal(p) => p,
            PowerPresentation::Unavailable => 0,
        };
        let label = format!("{pct}%");
        let (bolt_rect, glyph_rect, glyph_str) = match power {
            PowerPresentation::Charging(_) => (
                Some((glyph_x, (height - STAGE1_BOLT_H) * 0.5, STAGE1_BOLT_W, STAGE1_BOLT_H)),
                None,
                "",
            ),
            PowerPresentation::Low(_) => (
                None,
                Some((glyph_x, 0., power_glyph_w, height)),
                "🪫",
            ),
            PowerPresentation::Normal(_) => (
                None,
                Some((glyph_x, 0., power_glyph_w, height)),
                "🔋",
            ),
            PowerPresentation::Unavailable => (None, None, ""),
        };
        (
            Some((dot_cx, dot_cy, dot_r)),
            bolt_rect,
            glyph_rect,
            glyph_str,
            Some((label_x, 0., power_percent_w, height)),
            label,
        )
    } else {
        (None, None, None, "", None, String::new())
    };

    let content_end = cursor;
    let calendar_hit = if status != IdlePlanStatus::TooNarrow {
        let hit_w = (time_x + time_w + 4. - 8.).min(content_limit - 8.).max(1.);
        Some((8., 0., hit_w, height))
    } else {
        None
    };

    IdleClockPlan {
        status,
        width,
        height,
        date_rect,
        date_trimmed,
        separator_rect,
        time_rect,
        tws_dot,
        tws_glyph_rect,
        tws_label_rect,
        tws_label,
        power_dot,
        power_bolt_rect,
        power_glyph_rect,
        power_glyph,
        power_label_rect,
        power_label,
        privacy_slot,
        preset_slot,
        chevron_hit,
        chevron_glyph_x,
        calendar_hit,
        content_end,
        content_limit,
    }
}

pub fn calculate_idle_clock_target_width(
    date_w: f32,
    time_w: f32,
    power: PowerPresentation,
    power_glyph_w: f32,
    power_percent_w: f32,
    tws: TwsPresentation,
    tws_glyph_w: f32,
    tws_label_w: f32,
    privacy_rail_w: f32,
    preset_active: bool,
) -> f32 {
    let left_x = 10.;
    // Tanpa ikon kalender: lebar pill tidak lagi menyisakan ruang untuk ikon.
    let fixed_without_date = left_x
        + STAGE1_INFO_GAP
        + STAGE1_SEPARATOR_W
        + STAGE1_INFO_GAP
        + time_w;
    let tws_present = matches!(tws, TwsPresentation::Known(_) | TwsPresentation::Unknown);
    let tws_extra = if tws_present {
        STAGE1_INFO_GAP + STAGE1_DOT_RADIUS * 2. + STAGE1_DATE_GAP + tws_glyph_w + STAGE1_DATE_GAP + tws_label_w
    } else {
        0.
    };
    let power_present = matches!(power, PowerPresentation::Charging(_) | PowerPresentation::Low(_) | PowerPresentation::Normal(_));
    let effective_power_glyph_w = if matches!(power, PowerPresentation::Charging(_)) {
        STAGE1_BOLT_W
    } else {
        power_glyph_w
    };
    let power_extra = if power_present {
        STAGE1_INFO_GAP + STAGE1_DOT_RADIUS * 2. + STAGE1_DATE_GAP + effective_power_glyph_w + STAGE1_DATE_GAP + power_percent_w
    } else {
        0.
    };

    let content_end = fixed_without_date + date_w + tws_extra + power_extra;
    let has_secondary = tws_present || power_present || preset_active || privacy_rail_w > 0.;
    let suffix_needed = if has_secondary {
        32.
            + (if preset_active { 42. } else { 0. })
            + (if privacy_rail_w > 0. { privacy_rail_w + 10. } else { 0. })
    } else {
        32.
    };
    let needed = content_end + suffix_needed;
    let min_w = if has_secondary { 168. } else { 130. };
    needed.ceil().clamp(min_w, 440.)
}

pub const LYRIC_MIN_WIDTH: f32 = 270.0;
pub const LYRIC_MAX_WIDTH: f32 = 480.0;
pub const LYRIC_BASELINE_HEIGHT: f32 = 30.0;
pub const LYRIC_EXPANDED_HEIGHT: f32 = 50.0;
pub const LYRIC_OVERHEAD_WIDTH: f32 = 134.0;
/// The stretch notch shows at most two short lines. Cap the text we wrap so
/// pathological input (an entire unsynced lyric block handed in as one "line")
/// cannot turn the split search below into unbounded per-frame work.
const LYRIC_WRAP_MAX_CHARS: usize = 240;
/// Bound the number of split points evaluated; the search is quadratic in the
/// word count and runs on the UI thread every paint.
const LYRIC_WRAP_MAX_CANDIDATES: usize = 48;

pub fn calculate_lyric_stretch_width(text_w: f32) -> f32 {
    (text_w + LYRIC_OVERHEAD_WIDTH).ceil().clamp(LYRIC_MIN_WIDTH, LYRIC_MAX_WIDTH)
}

#[derive(Debug, Clone, PartialEq)]
pub struct LyricWrappedLines {
    pub line1: String,
    pub line2: Option<String>,
    pub is_multiline: bool,
    pub target_width: f32,
    pub target_height: f32,
}

pub fn plan_lyric_lines<F>(text: &str, romanized: Option<&str>, max_single_w: f32, mut measure_fn: F) -> LyricWrappedLines
where
    F: FnMut(&str) -> f32,
{
    if let Some(rom) = romanized.filter(|r| !r.trim().is_empty()) {
        let text_trimmed = text.trim();
        let rom_trimmed = rom.trim();
        let w1 = measure_fn(text_trimmed);
        let w2 = measure_fn(rom_trimmed);
        let max_w = w1.max(w2);
        let target_w = calculate_lyric_stretch_width(max_w);
        LyricWrappedLines {
            line1: text_trimmed.to_string(),
            line2: Some(rom_trimmed.to_string()),
            is_multiline: true,
            target_width: target_w,
            target_height: 42.0,
        }
    } else {
        wrap_lyric_lines(text, max_single_w, measure_fn)
    }
}

pub fn wrap_lyric_lines<F>(text: &str, max_single_w: f32, mut measure_fn: F) -> LyricWrappedLines
where
    F: FnMut(&str) -> f32,
{
    let full = text.trim();
    let trimmed = match full.char_indices().nth(LYRIC_WRAP_MAX_CHARS) {
        Some((idx, _)) => full[..idx].trim_end(),
        None => full,
    };
    if trimmed.is_empty() {
        return LyricWrappedLines {
            line1: String::new(),
            line2: None,
            is_multiline: false,
            target_width: LYRIC_MIN_WIDTH,
            target_height: LYRIC_BASELINE_HEIGHT,
        };
    }

    let full_w = measure_fn(trimmed);
    let single_target_w = calculate_lyric_stretch_width(full_w);

    if full_w + LYRIC_OVERHEAD_WIDTH <= 450.0 && full_w <= max_single_w {
        return LyricWrappedLines {
            line1: trimmed.to_string(),
            line2: None,
            is_multiline: false,
            target_width: single_target_w,
            target_height: LYRIC_BASELINE_HEIGHT,
        };
    }

    // Wrap into 2 lines
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() > 1 {
        let mut best_split = 1;
        let mut min_diff = f32::MAX;

        let n = words.len();
        let stride = ((n + LYRIC_WRAP_MAX_CANDIDATES - 1) / LYRIC_WRAP_MAX_CANDIDATES).max(1);
        let mut i = 1;
        while i < n {
            let l1 = words[..i].join(" ");
            let l2 = words[i..].join(" ");
            let w1 = measure_fn(&l1);
            let w2 = measure_fn(&l2);
            let diff = (w1 - w2).abs();
            if diff < min_diff {
                min_diff = diff;
                best_split = i;
            }
            i += stride;
        }

        let l1 = words[..best_split].join(" ");
        let l2 = words[best_split..].join(" ");
        let w1 = measure_fn(&l1);
        let w2 = measure_fn(&l2);
        let max_w = w1.max(w2);
        let target_w = calculate_lyric_stretch_width(max_w);

        LyricWrappedLines {
            line1: l1,
            line2: Some(l2),
            is_multiline: true,
            target_width: target_w,
            target_height: LYRIC_EXPANDED_HEIGHT,
        }
    } else {
        let char_count = trimmed.chars().count();
        if char_count > 30 {
            let half = char_count / 2;
            let l1: String = trimmed.chars().take(half).collect();
            let l2: String = trimmed.chars().skip(half).collect();
            let w1 = measure_fn(&l1);
            let w2 = measure_fn(&l2);
            let max_w = w1.max(w2);
            let target_w = calculate_lyric_stretch_width(max_w);

            LyricWrappedLines {
                line1: l1,
                line2: Some(l2),
                is_multiline: true,
                target_width: target_w,
                target_height: LYRIC_EXPANDED_HEIGHT,
            }
        } else {
            LyricWrappedLines {
                line1: trimmed.to_string(),
                line2: None,
                is_multiline: false,
                target_width: single_target_w,
                target_height: LYRIC_BASELINE_HEIGHT,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LyricStretchPlan {
    pub width: f32,
    pub height: f32,
    pub music_icon_rect: (f32, f32, f32, f32),
    pub separator_rect: (f32, f32, f32, f32),
    pub lyric_area_rect: (f32, f32, f32, f32),
    pub time_rect: (f32, f32, f32, f32),
    pub chevron_hit: (f32, f32, f32, f32),
    pub privacy_slot: Option<(f32, f32, f32, f32)>,
    pub preset_slot: Option<(f32, f32, f32, f32)>,
    pub is_multiline: bool,
    pub line1: String,
    pub line2: Option<String>,
}

pub fn plan_lyric_stretch(
    width: f32,
    height: f32,
    time_w: f32,
    privacy_rail_w: f32,
    preset_active: bool,
    wrapped: &LyricWrappedLines,
) -> LyricStretchPlan {
    let chevron_hit_x = width - 24.;
    let chevron_hit_y = (height - 20.) * 0.5;
    let chevron_hit = (chevron_hit_x, chevron_hit_y, 20., 20.);

    let mut cursor_right = chevron_hit_x - 6.; // gap between time and chevron

    let time_rect = (cursor_right - time_w, (height - 20.) * 0.5, time_w, 20.);
    cursor_right = time_rect.0 - STAGE1_GAP;

    let preset_slot = if preset_active {
        let slot_w = 30.;
        let slot_x = cursor_right - slot_w;
        cursor_right = slot_x - STAGE1_GAP;
        Some((slot_x, (height - 24.) * 0.5, slot_w, 24.))
    } else {
        None
    };

    let privacy_slot = if privacy_rail_w > 0. {
        let pad = 2.;
        let slot_w = privacy_rail_w + pad * 2.;
        let slot_x = cursor_right - slot_w;
        cursor_right = slot_x - 10.;
        Some((slot_x + pad, (height - 18.) * 0.5, privacy_rail_w, 18.))
    } else {
        None
    };

    let left_x = 10.;
    let icon_rect = (left_x, (height - STAGE1_ICON_SIZE) * 0.5, STAGE1_ICON_SIZE, STAGE1_ICON_SIZE);
    let sep_x = left_x + STAGE1_ICON_SIZE + 6.;
    let separator_rect = (sep_x, (height - 16.) * 0.5, STAGE1_SEPARATOR_W, 16.);

    let lyric_start_x = sep_x + STAGE1_SEPARATOR_W + 8.;
    let lyric_end_x = cursor_right;
    let lyric_area_w = (lyric_end_x - lyric_start_x).max(10.);
    let lyric_area_rect = (lyric_start_x, 0., lyric_area_w, height);

    LyricStretchPlan {
        width,
        height,
        music_icon_rect: icon_rect,
        separator_rect,
        lyric_area_rect,
        time_rect,
        chevron_hit,
        privacy_slot,
        preset_slot,
        is_multiline: wrapped.is_multiline,
        line1: wrapped.line1.clone(),
        line2: wrapped.line2.clone(),
    }
}


// --- Task 035: Stage 1 Hardware Monitor Plan & Layout ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HwThresholdState {
    Normal,
    High,
    Critical,
}

impl HwThresholdState {
    pub fn for_value(pct: f32, high: u8, critical: u8) -> Self {
        if pct > critical as f32 {
            HwThresholdState::Critical
        } else if pct > high as f32 {
            HwThresholdState::High
        } else {
            HwThresholdState::Normal
        }
    }

    pub fn color_u32(&self) -> u32 {
        match self {
            HwThresholdState::Normal => 0xececec,   // Clean white
            HwThresholdState::High => 0xf98a47,     // Amber / warning
            HwThresholdState::Critical => 0xf87171, // Red / danger
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HwChipPlan {
    pub metric: crate::core::HwMetric,
    pub label: &'static str,
    pub percent_text: String,
    pub percent_val: Option<f32>,
    pub threshold: HwThresholdState,
    pub chip_rect: (f32, f32, f32, f32),
    pub label_rect: (f32, f32, f32, f32),
    pub percent_rect: (f32, f32, f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HwRing {
    pub color: u32,
    pub ratio: f32,
    pub radius: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HardwareMonitorPlan {
    pub width: f32,
    pub height: f32,
    pub status_dot: (f32, f32, f32, f32), // (x, y, diameter, radius)
    pub status_color: u32,
    pub is_critical: bool,
    pub rings: Vec<HwRing>,
    pub gauge_center: (f32, f32),
    pub chips: Vec<HwChipPlan>,
    pub separators: Vec<(f32, f32, f32, f32)>,
    pub time_rect: (f32, f32, f32, f32),
    pub chevron_hit: (f32, f32, f32, f32),
    pub privacy_slot: Option<(f32, f32, f32, f32)>,
    pub preset_slot: Option<(f32, f32, f32, f32)>,
}

/// Distinct colour per hardware metric, used for both the multi-level gauge
/// rings and the percentage text.
pub fn hw_metric_color(m: crate::core::HwMetric) -> u32 {
    match m {
        crate::core::HwMetric::Cpu => 0x38bdf8, // cyan
        crate::core::HwMetric::Ram => 0x34d399, // green
        crate::core::HwMetric::Gpu => 0xa78bfa, // violet
    }
}

pub fn calculate_hardware_monitor_width(
    metrics: &[crate::core::HwMetric],
    _sample: Option<&crate::system::HardwareSample>,
    time_w: f32,
    privacy_rail_w: f32,
    preset_active: bool,
    mut measure_fn: impl FnMut(&str, f32, bool) -> f32,
) -> f32 {
    if metrics.is_empty() {
        return 150.0;
    }

    // Fixed value box so the pill width does not change as percentages tick
    // (e.g. "9%" vs "100%"). Width then depends only on the number of metrics.
    let value_box_w = measure_fn("100%", 12.5, true).max(measure_fn("--%", 12.5, true));

    let mut chips_w = 0.0;
    for (idx, m) in metrics.iter().enumerate() {
        if idx > 0 { chips_w += 16.0; } // gap + separator + gap
        let label = match m {
            crate::core::HwMetric::Ram => "RAM",
            crate::core::HwMetric::Cpu => "CPU",
            crate::core::HwMetric::Gpu => "GPU",
        };
        chips_w += measure_fn(label, 11.5, false) + 5.0 + value_box_w;
    }

    let left_zone = 12.0 + 20.0 + 12.0; // left pad + multi-level gauge + gap
    let gap_chips_right = 16.0;
    let preset_w = if preset_active { 30.0 + STAGE1_GAP } else { 0.0 };
    let privacy_w = if privacy_rail_w > 0.0 { privacy_rail_w + 4.0 + STAGE1_GAP } else { 0.0 };
    let right_zone = time_w + 6.0 + 24.0 + preset_w + privacy_w;

    (left_zone + chips_w + gap_chips_right + right_zone).ceil().clamp(150.0, 520.0)
}

pub fn plan_hardware_monitor(
    width: f32,
    height: f32,
    metrics: &[crate::core::HwMetric],
    sample: Option<&crate::system::HardwareSample>,
    threshold_high: u8,
    threshold_critical: u8,
    time_w: f32,
    privacy_rail_w: f32,
    preset_active: bool,
    mut measure_fn: impl FnMut(&str, f32, bool) -> f32,
) -> HardwareMonitorPlan {
    let chevron_hit_x = width - 24.;
    let chevron_hit_y = (height - 20.) * 0.5;
    let chevron_hit = (chevron_hit_x, chevron_hit_y, 20., 20.);

    let mut cursor_right = chevron_hit_x - 6.;
    let time_rect = (cursor_right - time_w, (height - 20.) * 0.5, time_w, 20.);
    cursor_right = time_rect.0 - STAGE1_GAP;

    let preset_slot = if preset_active {
        let slot_w = 30.;
        let slot_x = cursor_right - slot_w;
        cursor_right = slot_x - STAGE1_GAP;
        Some((slot_x, (height - 24.) * 0.5, slot_w, 24.))
    } else {
        None
    };
    let privacy_slot = if privacy_rail_w > 0. {
        let pad = 2.;
        let slot_w = privacy_rail_w + pad * 2.;
        let slot_x = cursor_right - slot_w;
        Some((slot_x + pad, (height - 18.) * 0.5, privacy_rail_w, 18.))
    } else {
        None
    };

    let dot_x = 12.0;
    let dot_d = 6.0;
    let dot_r = dot_d * 0.5;
    let dot_y = (height - dot_d) * 0.5;
    let status_dot = (dot_x, dot_y, dot_d, dot_r);

    // Fixed value box keeps every chip the same width regardless of its value.
    let value_box_w = measure_fn("100%", 12.5, true).max(measure_fn("--%", 12.5, true));

    // Every selected metric is shown (no shrinking when a value is missing).
    let mut chips = Vec::new();
    let mut separators = Vec::new();
    let mut most_critical = HwThresholdState::Normal;

    let mut chip_widths = Vec::with_capacity(metrics.len());
    for m in metrics.iter() {
        let label = match m {
            crate::core::HwMetric::Ram => "RAM",
            crate::core::HwMetric::Cpu => "CPU",
            crate::core::HwMetric::Gpu => "GPU",
        };
        chip_widths.push((*m, label, measure_fn(label, 11.5, false)));
    }
    let chips_total: f32 = chip_widths.iter().map(|(_, _, lw)| lw + 5.0 + value_box_w).sum::<f32>()
        + if chip_widths.len() > 1 { (chip_widths.len() as f32 - 1.0) * 16.0 } else { 0.0 };

    let left_zone_end = dot_x + 20.0 + 12.0;
    let right_limit = {
        let mut r = time_rect.0;
        if let Some(p) = preset_slot { r = r.min(p.0); }
        if let Some(p) = privacy_slot { r = r.min(p.0); }
        r
    };
    let avail_start = left_zone_end;
    let avail_end = (right_limit - 8.0).max(avail_start);
    let mut cursor_x = avail_start + ((avail_end - avail_start - chips_total).max(0.0)) * 0.5;

    for (idx, (metric, label, lbl_w)) in chip_widths.into_iter().enumerate() {
        if idx > 0 {
            let sep_x = cursor_x + 6.0;
            let sep_y = (height - 12.0) * 0.5;
            separators.push((sep_x, sep_y, 1.4, 12.0));
            cursor_x = sep_x + 1.4 + 6.0;
        }

        let val = match metric {
            crate::core::HwMetric::Ram => sample.and_then(|s| s.ram_percent),
            crate::core::HwMetric::Cpu => sample.and_then(|s| s.cpu_percent),
            crate::core::HwMetric::Gpu => sample.and_then(|s| s.gpu_percent),
        };
        let pct_str = match val {
            Some(p) => format!("{:.0}%", p.round()),
            None => "--%".to_string(),
        };
        let thresh = match val {
            Some(p) => HwThresholdState::for_value(p, threshold_high, threshold_critical),
            None => HwThresholdState::Normal,
        };
        if thresh == HwThresholdState::Critical {
            most_critical = HwThresholdState::Critical;
        } else if thresh == HwThresholdState::High && most_critical != HwThresholdState::Critical {
            most_critical = HwThresholdState::High;
        }

        let chip_w = lbl_w + 5.0 + value_box_w;
        let chip_rect = (cursor_x, 0.0, chip_w, height);
        let label_rect = (cursor_x, (height - 18.0) * 0.5, lbl_w, 18.0);
        let percent_rect = (cursor_x + lbl_w + 5.0, (height - 18.0) * 0.5, value_box_w, 18.0);

        chips.push(HwChipPlan {
            metric,
            label,
            percent_text: pct_str,
            percent_val: val,
            threshold: thresh,
            chip_rect,
            label_rect,
            percent_rect,
        });

        cursor_x += chip_w;
    }

    // Multi-level circular gauge: one concentric arc per selected metric, each
    // coloured by its metric and filled to its percentage.
    let gauge_center = (dot_x + 10.0, height * 0.5);
    let mut rings = Vec::new();
    for (i, m) in metrics.iter().take(3).enumerate() {
        let val = match m {
            crate::core::HwMetric::Ram => sample.and_then(|s| s.ram_percent),
            crate::core::HwMetric::Cpu => sample.and_then(|s| s.cpu_percent),
            crate::core::HwMetric::Gpu => sample.and_then(|s| s.gpu_percent),
        };
        let ratio = val.map(|p| (p / 100.0).clamp(0.0, 1.0)).unwrap_or(0.0);
        rings.push(HwRing { color: hw_metric_color(*m), ratio, radius: 9.0 - i as f32 * 2.8 });
    }

    HardwareMonitorPlan {
        width,
        height,
        status_dot,
        status_color: most_critical.color_u32(),
        is_critical: most_critical == HwThresholdState::Critical,
        rings,
        gauge_center,
        chips,
        separators,
        time_rect,
        chevron_hit,
        privacy_slot,
        preset_slot,
    }
}

#[cfg(test)] mod stage1_tests{

    #[test]
    fn test_hardware_monitor_threshold_colors() {
        use super::HwThresholdState;
        assert_eq!(HwThresholdState::for_value(50.0, 80, 90), HwThresholdState::Normal);
        assert_eq!(HwThresholdState::for_value(80.0, 80, 90), HwThresholdState::Normal);
        assert_eq!(HwThresholdState::for_value(81.0, 80, 90), HwThresholdState::High);
        assert_eq!(HwThresholdState::for_value(90.0, 80, 90), HwThresholdState::High);
        assert_eq!(HwThresholdState::for_value(91.0, 80, 90), HwThresholdState::Critical);
        assert_eq!(HwThresholdState::for_value(100.0, 80, 90), HwThresholdState::Critical);

        assert_eq!(HwThresholdState::Normal.color_u32(), 0xececec);
        assert_eq!(HwThresholdState::High.color_u32(), 0xf98a47);
        assert_eq!(HwThresholdState::Critical.color_u32(), 0xf87171);
    }

    #[test]
    fn test_plan_hardware_monitor_slots_and_geometry() {
        use crate::core::HwMetric;
        use crate::system::{HardwareDataQuality, HardwareSample};

        let sample = HardwareSample {
            ram_percent: Some(89.0),
            cpu_percent: Some(24.0),
            gpu_percent: Some(94.0),
            quality: HardwareDataQuality::Valid,
            timestamp: std::time::Instant::now(),
        };

        // 1. Two metrics: RAM + GPU (matches user screenshot condition)
        let metrics = [HwMetric::Ram, HwMetric::Gpu];
        let plan = plan_hardware_monitor(
            330.0,
            30.0,
            &metrics,
            Some(&sample),
            80,
            90,
            36.0,
            0.0,
            false,
            |s, _sz, _bold| s.len() as f32 * 7.0,
        );

        // Status dot at left: diameter 6, radius 3
        assert_eq!(plan.status_dot.0, 12.0);
        assert_eq!(plan.status_dot.2, 6.0);
        assert_eq!(plan.status_dot.3, 3.0);
        // Most critical metric is GPU (94% > 90%) -> status color is critical red
        assert_eq!(plan.status_color, 0xf87171);
        assert!(plan.is_critical);

        // Chips: 2 chips
        assert_eq!(plan.chips.len(), 2);
        assert_eq!(plan.chips[0].label, "RAM");
        assert_eq!(plan.chips[0].percent_text, "89%");
        assert_eq!(plan.chips[0].threshold, super::HwThresholdState::High);
        assert_eq!(plan.chips[0].threshold.color_u32(), 0xf98a47);

        assert_eq!(plan.chips[1].label, "GPU");
        assert_eq!(plan.chips[1].percent_text, "94%");
        assert_eq!(plan.chips[1].threshold, super::HwThresholdState::Critical);
        assert_eq!(plan.chips[1].threshold.color_u32(), 0xf87171);

        // 1 separator between 2 chips
        assert_eq!(plan.separators.len(), 1);

        // Slot Kanan: Chevron at width - 24, Time to the left of chevron
        assert_eq!(plan.chevron_hit.0, 330.0 - 24.0);
        assert!(plan.time_rect.0 + plan.time_rect.2 <= plan.chevron_hit.0);
        assert!(plan.time_rect.0 > plan.chips[1].chip_rect.0 + plan.chips[1].chip_rect.2);

        // 2. Three metrics: RAM + CPU + GPU
        let all_metrics = [HwMetric::Ram, HwMetric::Cpu, HwMetric::Gpu];
        let plan_all = plan_hardware_monitor(
            410.0,
            30.0,
            &all_metrics,
            Some(&sample),
            80,
            90,
            36.0,
            0.0,
            false,
            |s, _sz, _bold| s.len() as f32 * 7.0,
        );
        assert_eq!(plan_all.chips.len(), 3);
        assert_eq!(plan_all.separators.len(), 2);

        // 3. GPU unavailable: the selected GPU chip is still shown (as "--%") so the
        // pill width/geometry stays stable regardless of sampling availability.
        let sample_no_gpu = HardwareSample {
            ram_percent: Some(45.0),
            cpu_percent: Some(30.0),
            gpu_percent: None,
            quality: HardwareDataQuality::Valid,
            timestamp: std::time::Instant::now(),
        };
        let measure = |s: &str, _sz: f32, _bold: bool| s.len() as f32 * 7.0;
        let plan_fallback = plan_hardware_monitor(
            330.0,
            30.0,
            &[HwMetric::Ram, HwMetric::Gpu],
            Some(&sample_no_gpu),
            80,
            90,
            36.0,
            0.0,
            false,
            measure,
        );
        assert_eq!(plan_fallback.chips.len(), 2);
        assert_eq!(plan_fallback.chips[0].label, "RAM");
        assert_eq!(plan_fallback.chips[1].label, "GPU");
        assert_eq!(plan_fallback.chips[1].percent_text, "--%");
        assert_eq!(plan_fallback.status_color, 0xececec);
        assert!(!plan_fallback.is_critical);

        // 4. Width must not depend on the percentage value or GPU availability.
        let w_small = calculate_hardware_monitor_width(&[HwMetric::Ram, HwMetric::Gpu], Some(&sample_no_gpu), 36.0, 0.0, false, measure);
        let mut sample_low = sample.clone(); sample_low.ram_percent = Some(9.0); sample_low.gpu_percent = Some(7.0);
        let w_low = calculate_hardware_monitor_width(&[HwMetric::Ram, HwMetric::Gpu], Some(&sample_low), 36.0, 0.0, false, measure);
        let mut sample_full = sample.clone(); sample_full.ram_percent = Some(100.0); sample_full.gpu_percent = Some(100.0);
        let w_full = calculate_hardware_monitor_width(&[HwMetric::Ram, HwMetric::Gpu], Some(&sample_full), 36.0, 0.0, false, measure);
        assert_eq!(w_small, w_low);
        assert_eq!(w_low, w_full);
        // 1 vs 2 vs 3 metrics have distinct widths.
        let w1 = calculate_hardware_monitor_width(&[HwMetric::Cpu], Some(&sample), 36.0, 0.0, false, measure);
        let w2 = calculate_hardware_monitor_width(&[HwMetric::Cpu, HwMetric::Ram], Some(&sample), 36.0, 0.0, false, measure);
        let w3 = calculate_hardware_monitor_width(&[HwMetric::Cpu, HwMetric::Ram, HwMetric::Gpu], Some(&sample), 36.0, 0.0, false, measure);
        assert!(w1 < w2 && w2 < w3);
    }

    use super::*;
    #[test] fn weather_navigation_is_a_cloud(){assert_eq!(module_icon("Cuaca"),"cloud");}
    #[test] fn media_temporarily_overrides_reminder_without_changing_selection(){
        assert_eq!(stage1_tab("Reminder",true),"Musik");
        assert_eq!(stage1_tab("Reminder",false),"Reminder");
        assert_eq!(stage1_tab("Cuaca",true),"Musik");
    }
    #[test] fn compact_groups_have_equal_gaps_and_never_overlap(){
        let(a,b,c)=stage1_positions(28.,194.,40.,95.);
        assert!((b-(a+40.)-8.).abs()<0.01);assert!((c-(b+6.)-8.).abs()<0.01);
        let(a,b,c)=stage1_positions(28.,194.,300.,95.);
        assert_eq!(a,28.);assert_eq!(c,99.);assert!(b>=a);
    }
    #[test] fn compact_footer_preserves_charging_low_battery_and_desktop(){
        assert_eq!(stage1_footer("09:05",None,None),"09:05");
        assert_eq!(stage1_footer("09:05",Some((100,true)),None),"09:05 • ⚡100%");
        assert_eq!(stage1_footer("09:05",Some((20,false)),None),"09:05 • 🪫20%");
        assert_eq!(stage1_footer("09:05",Some((21,false)),None),"09:05 • 🔋21%");
    }
    #[test] fn compact_footer_adds_tws_before_system_battery(){
        assert_eq!(stage1_footer("09:05",None,Some(Some(80))),"09:05 • 🎧80%");
        assert_eq!(stage1_footer("09:05",Some((90,false)),Some(Some(80))),"09:05 • 🎧80% • 🔋90%");
        assert_eq!(stage1_footer("09:05",None,Some(None)),"09:05 • 🎧");
    }
    #[test] fn stage1_layout_keeps_mockup_order_without_overlap(){
        // Mockup 024 order: calendar -> date -> separator -> time -> dot -> bolt -> battery -> chevron.
        let layout=stage1_layout(43.,250.,date_w(38.),time_w(36.),battery_w(34.),5.,7.,30.);
        let layout=layout.expect("base row must fit");
        assert!(layout.icon_x < layout.date_x);
        assert!(layout.date_x < layout.separator_x);
        assert!(layout.separator_x < layout.time_x);
        assert!(layout.time_x + 36. + 7. <= layout.dot_x);
        assert!(layout.dot_x < layout.bolt_x);
        assert!(layout.bolt_x < layout.battery_x);
        assert!(layout.battery_x + 34. <= layout.chevron_hit_x);
        assert!((layout.chevron_glyph_x - layout.chevron_hit_x - 10.).abs() < 0.01);
    }
    #[test] fn stage1_layout_falls_back_when_localized_text_is_too_long(){
        assert!(stage1_layout(43.,250.,150.,140.,34.,5.,7.,30.).is_none(),"long locale text must overflow to fallback, not overlap the chevron");
    }
    #[test] fn stage1_layout_without_battery_has_no_bolt_slot(){
        let layout=stage1_layout(43.,250.,38.,36.,0.,5.,7.,30.).expect("desktop row must fit");
        assert!(layout.battery_x <= layout.chevron_hit_x);
    }
    #[test] fn stage1_layout_validator_counterexample_and_boundary_checks(){
        // L01: Validator counterexample left=10, right=175, D=38, T=36, B=34, g=5, k=7
        // Content end is 178.6 > 175 -> must return None
        assert!(stage1_layout(10., 175., 38., 36., 34., 5., 7., 30.).is_none(), "counterexample 175 must be rejected");
        // L02: Exact equality boundary: available = 168.6, total = 168.6, right = 178.6
        let exact = stage1_layout(10., 178.6, 38., 36., 34., 5., 7., 30.).expect("exact boundary 178.6 must fit");
        assert!((exact.battery_x + 34. - 178.6).abs() < 0.001);
        // L02: Just below boundary: 178.59 -> must return None
        assert!(stage1_layout(10., 178.59, 38., 36., 34., 5., 7., 30.).is_none(), "boundary 178.59 must be rejected");
    }
    #[test] fn stage1_plan_idle_clock_power_presentations(){
        // P09: Verify charging bolt vs low icon vs normal icon vs unavailable
        let p_unavail = plan_idle_clock(250., 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Unavailable, 0., 0., TwsPresentation::Absent, 0., 0., 0., false);
        assert!(p_unavail.power_dot.is_none());
        assert!(p_unavail.power_bolt_rect.is_none());
        assert!(p_unavail.power_glyph_rect.is_none());
        assert!(p_unavail.power_label.is_empty());

        let p_charging = plan_idle_clock(250., 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Charging(100), 0., 34., TwsPresentation::Absent, 0., 0., 0., false);
        assert!(p_charging.power_dot.is_some());
        assert!(p_charging.power_bolt_rect.is_some());
        assert!(p_charging.power_glyph_rect.is_none());
        assert_eq!(p_charging.power_label, "100%");

        let p_low = plan_idle_clock(250., 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Low(20), 12., 30., TwsPresentation::Absent, 0., 0., 0., false);
        assert!(p_low.power_dot.is_some());
        assert!(p_low.power_bolt_rect.is_none());
        assert_eq!(p_low.power_glyph, "🪫");
        assert_eq!(p_low.power_label, "20%");

        let p_normal = plan_idle_clock(250., 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Normal(85), 12., 30., TwsPresentation::Absent, 0., 0., 0., false);
        assert!(p_normal.power_dot.is_some());
        assert!(p_normal.power_bolt_rect.is_none());
        assert_eq!(p_normal.power_glyph, "🔋");
        assert_eq!(p_normal.power_label, "85%");
    }
    #[test] fn stage1_plan_idle_clock_tws_presentations(){
        // P10: Verify TWS absent, known, unknown
        let p_absent = plan_idle_clock(250., 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Unavailable, 0., 0., TwsPresentation::Absent, 0., 0., 0., false);
        assert!(p_absent.tws_dot.is_none());
        assert!(p_absent.tws_glyph_rect.is_none());
        assert!(p_absent.tws_label.is_empty());

        let p_known = plan_idle_clock(300., 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Unavailable, 0., 0., TwsPresentation::Known(80), 12., 30., 0., false);
        assert!(p_known.tws_dot.is_some());
        assert!(p_known.tws_glyph_rect.is_some());
        assert_eq!(p_known.tws_label, "80%");

        let p_unknown = plan_idle_clock(300., 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Unavailable, 0., 0., TwsPresentation::Unknown, 12., 10., 0., false);
        assert!(p_unknown.tws_dot.is_some());
        assert!(p_unknown.tws_glyph_rect.is_some());
        assert_eq!(p_unknown.tws_label, "—");
    }
    #[test] fn stage1_plan_idle_clock_privacy_and_preset_slots(){
        // P11, L05, L06: privacy rail + preset + chevron right packing
        let p = plan_idle_clock(380., 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Charging(100), 0., 34., TwsPresentation::Known(80), 12., 30., 40., true);
        assert_eq!(p.status, IdlePlanStatus::Full);
        assert!(p.content_end <= p.content_limit);
        let chevron = p.chevron_hit;
        let preset = p.preset_slot.expect("preset slot present");
        let privacy = p.privacy_slot.expect("privacy slot present");
        // Right to left ordering: chevron -> preset -> privacy -> content
        assert!(chevron.0 > preset.0);
        assert!(preset.0 > privacy.0);
        assert!(privacy.0 > p.content_end);
        assert!(p.calendar_hit.is_some());
    }
    #[test] fn stage1_plan_idle_clock_date_trimmed_and_too_narrow(){
        // L07, L08: window narrower than full content trims date, tiny window is TooNarrow
        let p_trimmed = plan_idle_clock(220., 30., "25 Desember 2026", 120., "10:45", 36., PowerPresentation::Charging(95), 0., 34., TwsPresentation::Absent, 0., 0., 0., false);
        assert_eq!(p_trimmed.status, IdlePlanStatus::DateTrimmed);
        assert!(p_trimmed.date_trimmed);
        assert!(p_trimmed.date_rect.2 < 120.);
        assert!(p_trimmed.calendar_hit.is_some());

        let p_narrow = plan_idle_clock(50., 10., "5 Nov", 38., "10:45", 36., PowerPresentation::Charging(95), 0., 34., TwsPresentation::Absent, 0., 0., 0., false);
        assert_eq!(p_narrow.status, IdlePlanStatus::TooNarrow);
        assert!(p_narrow.calendar_hit.is_none());
    }
    #[test] fn stage1_calculate_target_width_exact_table(){
        // Values from 025a-integration-plan.md Section 2.3 Table, minus the 17 px
        // calendar icon slot removed from the idle plan (12 icon + 5 gap).
        // D38/T36, no status -> 132 (content-driven adaptive on desktop without secondary elements)
        assert_eq!(calculate_idle_clock_target_width(38., 36., PowerPresentation::Unavailable, 0., 0., TwsPresentation::Absent, 0., 0., 0., false), 132.);
        // D38/T36, charging B34 -> 194
        assert_eq!(calculate_idle_clock_target_width(38., 36., PowerPresentation::Charging(100), 0., 34., TwsPresentation::Absent, 0., 0., 0., false), 194.);
        // D38/T36, charging B34 + TWS group 46 (glyph 12 + gap 5 + label 29) -> 255
        assert_eq!(calculate_idle_clock_target_width(38., 36., PowerPresentation::Charging(100), 0., 34., TwsPresentation::Known(80), 12., 29., 0., false), 255.);
        // Plus privacy2 (R=40) + preset -> 347
        assert_eq!(calculate_idle_clock_target_width(38., 36., PowerPresentation::Charging(100), 0., 34., TwsPresentation::Known(80), 12., 29., 40., true), 347.);
    }
    #[test] fn stage1_desktop_adaptive_content_driven_layout(){
        // Desktop profile: no battery, no TWS, no privacy rail, no preset
        let target_w = calculate_idle_clock_target_width(38., 36., PowerPresentation::Unavailable, 0., 0., TwsPresentation::Absent, 0., 0., 0., false);
        // Must be compact between 125 and 140 DIP (icon slot removed from the plan)
        assert!(target_w >= 125. && target_w <= 140., "target width was {target_w}, expected 125-140");

        let plan = plan_idle_clock(target_w, 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Unavailable, 0., 0., TwsPresentation::Absent, 0., 0., 0., false);
        assert_eq!(plan.status, IdlePlanStatus::Full);
        assert!(!plan.date_trimmed);
        assert!(plan.power_dot.is_none());
        assert!(plan.tws_dot.is_none());

        // Chevron must sit tightly after time text (~8-10 DIP gap), no huge gap (> 20 DIP)
        let time_end = plan.time_rect.0 + plan.time_rect.2;
        let chevron_start = plan.chevron_hit.0;
        let gap = chevron_start - time_end;
        assert!(gap >= 7.0 && gap <= 12.0, "gap between time and chevron was {gap}, expected 7-12 DIP");
    }
    #[test] fn stage1_laptop_battery_preserves_slot_without_clipping(){
        // Laptop profile: battery charging 85%, no TWS
        let target_w = calculate_idle_clock_target_width(38., 36., PowerPresentation::Charging(85), 0., 30., TwsPresentation::Absent, 0., 0., 0., false);
        assert!(target_w >= 190., "laptop target width must expand for battery, got {target_w}");

        let plan = plan_idle_clock(target_w, 30., "5 Nov", 38., "10:45", 36., PowerPresentation::Charging(85), 0., 30., TwsPresentation::Absent, 0., 0., 0., false);
        assert_eq!(plan.status, IdlePlanStatus::Full);
        assert!(!plan.date_trimmed);
        assert!(plan.power_dot.is_some());
        assert!(plan.power_bolt_rect.is_some());
        assert_eq!(plan.power_label, "85%");

        // Verify order: time -> power -> chevron
        let time_end = plan.time_rect.0 + plan.time_rect.2;
        let power_label_end = plan.power_label_rect.unwrap().0 + plan.power_label_rect.unwrap().2;
        assert!(power_label_end > time_end);
        assert!(plan.chevron_hit.0 >= power_label_end);
    }
    #[test] fn stage1_power_bolt_polygon_geometry_and_symmetry() {
        let pts = power_bolt_polygon(10., 20., 8., 12.);
        assert_eq!(pts.len(), 6);
        // Top and bottom endpoints
        assert!((pts[0].1 - 20.).abs() < 1e-4);
        assert!((pts[3].1 - 32.).abs() < 1e-4);
        // Horizontal tooth cuts (pts[1] to pts[2] and pts[4] to pts[5])
        assert!((pts[1].1 - pts[2].1).abs() < 1e-4);
        assert!((pts[4].1 - pts[5].1).abs() < 1e-4);
        assert!(pts[2].0 > pts[1].0, "Left tooth must step rightwards");
        assert!(pts[4].0 > pts[5].0, "Right tooth must step rightwards");
        // Overlapping waist height (connects without gap)
        assert!(pts[2].1 > pts[5].1, "Waist must have non-zero vertical overlap");
        // Rotational symmetry around center (14, 26)
        let cx = 10. + 4.;
        let cy = 20. + 6.;
        for i in 0..3 {
            let j = i + 3;
            assert!(((pts[i].0 - cx) + (pts[j].0 - cx)).abs() < 1e-3);
            assert!(((pts[i].1 - cy) + (pts[j].1 - cy)).abs() < 1e-3);
        }
    }
    #[test] fn stage1_lyric_stretch_width_clamping(){
        // Short lyric (e.g. 76 DIP) -> clamp to 270 (min)
        assert_eq!(calculate_lyric_stretch_width(76.0), 270.0);
        // Medium lyric (e.g. 231 DIP) -> 231 + 134 = 365 DIP
        assert_eq!(calculate_lyric_stretch_width(231.0), 365.0);
        // Long lyric (e.g. 500 DIP) -> clamp to 480 (max)
        assert_eq!(calculate_lyric_stretch_width(500.0), 480.0);
    }
    #[test] fn stage1_lyric_stretch_wrap_and_height_expansion(){
        // Short lyric stays 1 line, height 30 DIP
        let short_res = wrap_lyric_lines("Hello, it's me", 346.0, |s| s.len() as f32 * 7.5);
        assert!(!short_res.is_multiline);
        assert_eq!(short_res.target_height, 30.0);
        assert_eq!(short_res.line1, "Hello, it's me");
        assert!(short_res.line2.is_none());

        // Long lyric exceeding 450-480 single line space wraps into 2 lines, height expands to 50 DIP
        let long_text = "At least I can say that I've tried to tell you I'm sorry for breaking your heart";
        let long_res = wrap_lyric_lines(long_text, 346.0, |s| s.len() as f32 * 7.5);
        assert!(long_res.is_multiline);
        assert_eq!(long_res.target_height, 50.0);
        assert!(long_res.line2.is_some());
        assert_eq!(format!("{} {}", long_res.line1, long_res.line2.unwrap()), long_text);
    }
    #[test] fn stage1_lyric_wrap_bounds_pathological_input(){
        // An entire unsynced lyric block handed in as one "line" must not produce
        // unbounded wrapped text or unbounded split-search work (this used to run
        // the UI thread at 100% CPU every frame).
        let huge = "word ".repeat(2000);
        let res = wrap_lyric_lines(&huge, 346.0, |s| s.len() as f32 * 7.5);
        let total = res.line1.chars().count() + res.line2.as_deref().map(|s| s.chars().count()).unwrap_or(0);
        assert!(total <= LYRIC_WRAP_MAX_CHARS, "wrapped text was {total} chars, expected <= {LYRIC_WRAP_MAX_CHARS}");
    }
    #[test] fn stage1_lyric_stretch_plan_layout_slots_and_clock_on_right(){
        let wrapped = wrap_lyric_lines("Hello, it's me", 346.0, |s| s.len() as f32 * 7.5);
        let plan = plan_lyric_stretch(300.0, 30.0, 36.0, 0.0, false, &wrapped);

        // Slot Kiri: icon at left 10, separator at 30
        assert_eq!(plan.music_icon_rect.0, 10.0);
        assert_eq!(plan.separator_rect.0, 28.0);

        // Slot Kanan: chevron at width - 24, time is right before chevron, NEVER at left near music
        assert_eq!(plan.chevron_hit.0, 300.0 - 24.0);
        assert!(plan.time_rect.0 + plan.time_rect.2 <= plan.chevron_hit.0);
        assert!(plan.time_rect.0 > 200.0, "Time must be on the right, not on the left");

        // Slot Tengah: lyric area spans between separator and time
        assert!(plan.lyric_area_rect.0 > plan.separator_rect.0);
        assert!(plan.lyric_area_rect.0 + plan.lyric_area_rect.2 <= plan.time_rect.0);
    }
    #[test] fn stage1_lyric_stretch_plan_layout_privacy_clearance(){
        let wrapped = wrap_lyric_lines("Can you tell me is it hard for you", 346.0, |s| s.len() as f32 * 7.5);
        let plan = plan_lyric_stretch(380.0, 50.0, 36.0, 20.0, false, &wrapped);
        let priv_slot = plan.privacy_slot.expect("privacy slot should be present");
        assert!(plan.lyric_area_rect.0 + plan.lyric_area_rect.2 <= priv_slot.0 - 8.0, 
            "Lyric area must maintain clearance from privacy badge");
    }
    fn date_w(v:f32)->f32{v}
    fn time_w(v:f32)->f32{v}
    fn battery_w(v:f32)->f32{v}
}

pub fn stage1_tab(selected:&str,media:bool)->&str{if media{"Musik"}else{selected}}
/// Minimum logical width of an idle/calendar Stage 1 row: icon+date+time row
/// with the battery/privacy extras above; keeps long locale text from
/// overlapping the chevron hit box (caller falls back via `stage1_layout`).
pub fn stage1_row_min_width(battery_extra: f32) -> f32 {
    168. + battery_extra
}
pub fn stage1_positions(left:f32,right:f32,info:f32,footer:f32)->(f32,f32,f32){
    let available=(right-left).max(0.);let info=info.min((available-footer-22.).max(0.));
    let total=info+22.+footer;let start=left+((available-total)*0.5).max(0.);
    (start,start+info+8.,start+info+22.)
}

pub fn stage1_deadline_ms(media:Option<Instant>,temporary:Option<Instant>,now:Instant)->Option<u32>{
    [media,temporary].into_iter().flatten().filter(|until|*until>now).min().map(|until|until.duration_since(now).as_millis().clamp(1,u32::MAX as u128)as u32)
}
#[cfg(test)]mod stage1_deadline_tests{
    use super::*;
    #[test]fn volume_or_hover_never_delays_media_expiry(){
        let now=Instant::now();let media=Some(now+Duration::from_secs(5));
        assert_eq!(stage1_deadline_ms(media,Some(now+Duration::from_secs(2)),now),Some(2000));
        assert_eq!(stage1_deadline_ms(media,Some(now+Duration::from_secs(9)),now),Some(5000));
        assert_eq!(stage1_deadline_ms(media,None,now+Duration::from_secs(3)),Some(2000));
        assert_eq!(stage1_deadline_ms(media,None,now+Duration::from_secs(5)),None);
    }
}

/// At most two source icons; Auto is always visible and is cursor zero.
pub fn media_source_range(count:usize,cursor:usize)->std::ops::Range<usize>{
    let start=cursor.saturating_sub(2).min(count.saturating_sub(2));
    start..(start+2).min(count)
}
#[cfg(test)]mod media_source_tests{
    use super::*;
    #[test]fn source_overflow_keeps_every_focused_session_visible(){
        for count in 0..40{
            for cursor in 0..=count{
                let range=media_source_range(count,cursor);
                assert!(range.len()<=2);assert!(range.end<=count);
                if cursor>0{assert!(range.contains(&(cursor-1)));}
            }
        }
    }
    #[test]fn source_tray_stays_above_timeline_and_outside_metadata(){
        for count in 0..=2{
            let width=38.+count as f32*32.;let left=PLAYER_WIDTH-20.-width;
            assert!(left>=338.);assert!(70.+38.<114.);
            assert!(94.+(left-106.)<left);
        }
        assert_eq!(PLAYER_HEIGHT,180.);
    }
}
#[cfg(test)]mod accent_sweep_tests{
    use super::*;
    #[test]fn sweep_easing_hits_endpoints_and_stays_in_range(){
        assert_eq!(ease_accent_sweep(0.),0.);
        assert_eq!(ease_accent_sweep(1.),1.);
        assert_eq!(ease_accent_sweep(-0.5),0.);
        assert_eq!(ease_accent_sweep(1.5),1.);
        let mut prev=0.;
        for i in 0..=100{
            let t=i as f32/100.;
            let y=ease_accent_sweep(t);
            assert!((0.0..=1.0).contains(&y),"ease keluar [0,1] di t={t}: {y}");
            assert!(y>=prev-1e-6,"ease tidak monotonik di t={t}: {y} < {prev}");
            prev=y;
        }
    }
    #[test]fn sweep_easing_matches_mockup_bezier_midpoints(){
        // cubic-bezier(.55,.1,.45,.9): simetris — y(0.25)+y(0.75)=1, y(0.5)=0.5.
        let y25=ease_accent_sweep(0.25);let y75=ease_accent_sweep(0.75);let y50=ease_accent_sweep(0.5);
        assert!((y50-0.5).abs()<1e-3,"y(0.5)={y50}");
        assert!((y25+y75-1.).abs()<1e-3,"y(0.25)+y(0.75)={}",y25+y75);
        // Lambat di awal (ease-in): kuartal pertama di bawah garis lurus.
        assert!(y25<0.25+1e-6,"y(0.25)={y25} harus < 0.25");
        assert!(y75>0.75-1e-6,"y(0.75)={y75} harus > 0.75");
    }
    #[test]fn sweep_travel_covers_pill_with_offscreen_entry_and_exit(){
        // Pusat dari -1.276w ke 2.084w dengan setengah pita 0.528w: seluruh pita
        // berada di luar pill di kedua ujung siklus.
        let from=ACCENT_SWEEP_CENTER_FROM+ACCENT_SWEEP_HALF;
        let to=ACCENT_SWEEP_CENTER_TO-ACCENT_SWEEP_HALF;
        assert!(from<0.,"pita belum keluar dari kiri saat t=0: {from}");
        assert!(to>1.,"pita belum keluar dari kanan saat t=1: {to}");
    }
}
