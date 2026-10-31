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

/// Shared compact footer; omit the entire battery group on desktops.
pub fn stage1_footer(time:&str,battery:Option<(u32,bool)>,tws:Option<Option<u8>>)->String{
    let mut out=time.to_string();
    if let Some(tws)=tws{out.push_str(&format!(" • 🎧{}",tws.map(|p|format!("{p}%")).unwrap_or_default()));}
    if let Some((percent,charging))=battery{out.push_str(&format!(" • {}{percent}%",if charging{"⚡"}else if percent<=20{"🪫"}else{"🔋"}));}
    out
}
#[cfg(test)] mod stage1_tests{
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
}

pub fn stage1_tab(selected:&str,media:bool)->&str{if media{"Musik"}else{selected}}
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
