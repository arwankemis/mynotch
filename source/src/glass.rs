//! Liquid Glass material: pure decision/token/contour logic plus the native DWM backdrop hook.
//!
//! Real blur is produced by DWM (`SetWindowCompositionAttribute`), not by capturing the desktop:
//! DWM blurs whatever is behind the window *inside the window region*. The region is therefore
//! the blur mask, and it is built from the very same contour description (`notch_contour`)
//! that the Direct2D path, the outline and the alpha mask use. Everything in the top half of
//! this file is pure and unit-tested; the bottom half is the thin Win32 layer.

use std::ffi::c_void;

// ---------------------------------------------------------------------------------------------
// Settings vocabulary
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialMode {
    #[default]
    Solid,
    Lite,
    Adaptive,
}

impl MaterialMode {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "lite" => MaterialMode::Lite,
            "adaptive" => MaterialMode::Adaptive,
            _ => MaterialMode::Solid,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            MaterialMode::Solid => "solid",
            MaterialMode::Lite => "lite",
            MaterialMode::Adaptive => "adaptive",
        }
    }
    pub fn is_glass(&self) -> bool {
        matches!(self, MaterialMode::Lite | MaterialMode::Adaptive)
    }
    /// Index used by the settings popup (0 Solid, 1 Lite, 2 Adaptive).
    pub fn index(&self) -> u32 {
        match self {
            MaterialMode::Solid => 0,
            MaterialMode::Lite => 1,
            MaterialMode::Adaptive => 2,
        }
    }
    pub fn from_index(i: u32) -> Self {
        match i {
            1 => MaterialMode::Lite,
            2 => MaterialMode::Adaptive,
            _ => MaterialMode::Solid,
        }
    }
}

/// Gelap / Terang. Dark is the historic palette and the default.
pub fn is_light_mode(color_mode: &str) -> bool {
    color_mode.trim().eq_ignore_ascii_case("light")
}

/// Everything that can force the glass path off, gathered once and cached by the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlassEnv {
    pub reduced_transparency: bool,
    pub high_contrast: bool,
    /// WARP / `--software-renderer` / the "Low RAM" setting.
    pub software: bool,
    /// The panel is attached to the screen edge / floating (either is supported by the contour).
    pub backdrop_unavailable: bool,
}

/// Why the effective material is what it is. Used by the settings UI hint and by tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fallback {
    None,
    UserSolid,
    ReducedTransparency,
    HighContrast,
    Software,
    NoBackdrop,
}

/// Real fallback decision: glass only when the user asked for it AND nothing forbids it.
pub fn decide(mode: MaterialMode, env: GlassEnv) -> (MaterialMode, Fallback) {
    if !mode.is_glass() {
        return (MaterialMode::Solid, Fallback::UserSolid);
    }
    if env.high_contrast {
        return (MaterialMode::Solid, Fallback::HighContrast);
    }
    if env.reduced_transparency {
        return (MaterialMode::Solid, Fallback::ReducedTransparency);
    }
    if env.software {
        return (MaterialMode::Solid, Fallback::Software);
    }
    if env.backdrop_unavailable {
        return (MaterialMode::Solid, Fallback::NoBackdrop);
    }
    (mode, Fallback::None)
}

/// Light colors are honored unless High Contrast owns the palette.
pub fn effective_light(light_requested: bool, env: GlassEnv) -> bool {
    light_requested && !env.high_contrast
}

// ---------------------------------------------------------------------------------------------
// Shared colour tokens (light/dark). Glyphs, icons, buttons, sliders, cards, tabs, tooltips and
// toasts all draw through `Painter::color`, which calls `map_token`; there is no per-glyph blur.
// ---------------------------------------------------------------------------------------------

const LIGHT_HI: [f32; 3] = [236., 239., 243.]; // what black becomes (surface)
const LIGHT_LO: [f32; 3] = [16., 19., 25.]; // what white becomes (text)

/// Maps a dark-theme colour to its light-theme counterpart. Neutral/cool-grey colours (all
/// surfaces and text) are inverted into the light range; saturated accents keep their hue and are
/// darkened just enough to stay readable on a light surface. Dark mode is the identity.
pub fn map_token(rgb: u32, light: bool) -> u32 {
    if !light {
        return rgb;
    }
    let c = [((rgb >> 16) & 255) as f32, ((rgb >> 8) & 255) as f32, (rgb & 255) as f32];
    let mx = c[0].max(c[1]).max(c[2]);
    let mn = c[0].min(c[1]).min(c[2]);
    let out: [f32; 3] = if mx - mn <= 48. {
        [
            LIGHT_HI[0] - c[0] * (LIGHT_HI[0] - LIGHT_LO[0]) / 255.,
            LIGHT_HI[1] - c[1] * (LIGHT_HI[1] - LIGHT_LO[1]) / 255.,
            LIGHT_HI[2] - c[2] * (LIGHT_HI[2] - LIGHT_LO[2]) / 255.,
        ]
    } else {
        let lum = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let k = if lum > 110. { 0.62 } else { 1.0 };
        [c[0] * k, c[1] * k, c[2] * k]
    };
    ((out[0].round().clamp(0., 255.) as u32) << 16) | ((out[1].round().clamp(0., 255.) as u32) << 8) | (out[2].round().clamp(0., 255.) as u32)
}

pub fn relative_luminance(rgb: u32) -> f32 {
    let f = |v: u32| {
        let s = v as f32 / 255.;
        if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * f((rgb >> 16) & 255) + 0.7152 * f((rgb >> 8) & 255) + 0.0722 * f(rgb & 255)
}

pub fn contrast_ratio(a: u32, b: u32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

// ---------------------------------------------------------------------------------------------
// Material configuration per mode / stage / colour mode
// ---------------------------------------------------------------------------------------------

/// Which DWM backdrop is requested for the window region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    /// No backdrop (Solid).
    Off,
    /// Legacy DWM Gaussian blur-behind (`ACCENT_ENABLE_BLURBEHIND`).
    Blur,
    /// Acrylic (`ACCENT_ENABLE_ACRYLICBLURBEHIND`): stronger blur + noise.
    Acrylic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquidGlassConfig {
    pub mode: MaterialMode,
    pub stage: u8,
    pub is_light: bool,
    pub backdrop: Backdrop,
    pub tint_rgb: u32,
    pub tint_alpha: f32,
    pub outline_rgb: u32,
    pub outline_alpha: f32,
    pub outline_width: f32,
    /// Strength of the top sheen gradient merged into the same material fill (no child gloss).
    pub sheen: f32,
}

impl LiquidGlassConfig {
    /// Lite = stronger tint and the lighter DWM blur. Adaptive = acrylic whose tint thins out as
    /// the surface grows (Stage 0 stays dense so a 50x10 pill stays readable, Stage 2 shows
    /// the most backdrop). Solid = no backdrop at all.
    pub fn for_stage(mode: MaterialMode, stage: u8, is_light: bool) -> Self {
        let stage = stage.min(2);
        match mode {
            MaterialMode::Solid => Self {
                mode,
                stage,
                is_light,
                backdrop: Backdrop::Off,
                tint_rgb: if is_light { 0xeceff3 } else { 0x000000 },
                tint_alpha: 1.0,
                outline_rgb: if is_light { 0xd0d5dd } else { 0x222226 },
                outline_alpha: 0.8,
                outline_width: 1.0,
                sheen: 0.0,
            },
            MaterialMode::Lite => Self {
                mode,
                stage,
                is_light,
                backdrop: Backdrop::Blur,
                tint_rgb: if is_light { 0xf0f3f6 } else { 0x101318 },
                tint_alpha: if is_light { 0.82 } else { 0.84 },
                outline_rgb: if is_light { 0xffffff } else { 0x4a5262 },
                outline_alpha: 0.55,
                outline_width: 1.0,
                sheen: 0.10,
            },
            MaterialMode::Adaptive => {
                let (tint_alpha, sheen) = match stage {
                    0 => (0.80, 0.12),
                    1 => (0.62, 0.18),
                    _ => (0.50, 0.22),
                };
                let tint_rgb = if is_light {
                    match stage {
                        0 => 0xf5f7fa,
                        1 => 0xeef2f6,
                        _ => 0xe9eef4,
                    }
                } else {
                    match stage {
                        0 => 0x0c0e12,
                        1 => 0x141820,
                        _ => 0x181e28,
                    }
                };
                Self {
                    mode,
                    stage,
                    is_light,
                    backdrop: Backdrop::Acrylic,
                    tint_rgb,
                    tint_alpha,
                    outline_rgb: if is_light { 0xffffff } else { 0x5a667a },
                    outline_alpha: 0.60,
                    outline_width: 1.0,
                    sheen,
                }
            }
        }
    }
}

impl LiquidGlassConfig {
    /// Where on the Stage 0 -> 1 -> 2 axis a body of this height sits (0.0 ..= 2.0). Driving the
    /// material by the *live* spring height instead of the discrete stage keeps tint continuous
    /// when a morph is retargeted mid-flight, and it needs no history of the previous stage.
    pub fn stage_position(height: f32) -> f32 {
        if height <= 10. {
            0.
        } else if height <= 32. {
            (height - 10.) / 22.
        } else if height <= 180. {
            1. + (height - 32.) / 148.
        } else {
            2.
        }
    }

    pub fn for_height(mode: MaterialMode, height: f32, is_light: bool) -> Self {
        let pos = Self::stage_position(height).clamp(0., 2.);
        let lo = pos.floor() as u8;
        let hi = (lo + 1).min(2);
        let f = pos - lo as f32;
        let a = Self::for_stage(mode, lo, is_light);
        let b = Self::for_stage(mode, hi, is_light);
        let mix = |x: f32, y: f32| x + (y - x) * f;
        let mix_rgb = |x: u32, y: u32| {
            let ch = |s: u32| {
                let (p, q) = (((x >> s) & 255) as f32, ((y >> s) & 255) as f32);
                (p + (q - p) * f).round() as u32
            };
            (ch(16) << 16) | (ch(8) << 8) | ch(0)
        };
        Self {
            stage: if f >= 0.5 { hi } else { lo },
            tint_rgb: mix_rgb(a.tint_rgb, b.tint_rgb),
            tint_alpha: mix(a.tint_alpha, b.tint_alpha),
            sheen: mix(a.sheen, b.sheen),
            ..a
        }
    }

    /// Dense pages (Settings, Reminder) need more body behind small text.
    pub fn dense(mut self) -> Self {
        if self.mode.is_glass() {
            self.tint_alpha = (self.tint_alpha + 0.2).min(0.94);
        }
        self
    }
}

/// True when the OS exposes the composition-attribute entry point DWM backdrops go through.
pub fn backdrop_supported() -> bool {
    static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OK.get_or_init(|| set_wca().is_some())
}

// ---------------------------------------------------------------------------------------------
// One contour
// ---------------------------------------------------------------------------------------------

pub type P = (f32, f32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    Line(P),
    Bezier(P, P, P),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    pub start: P,
    pub segs: Vec<Seg>,
}

/// THE notch contour in capsule coordinates (origin = top-left of the body, +y down). The
/// Direct2D path (material fill, alpha mask, outline, glow clip) and the window region polygon
/// are both generated from this list.
///
/// `ear > 0` gives the attached-top silhouette: concave ears flare outwards to y = 0 on both
/// sides and the top corners are square against the screen edge. `round_top` (floating / bottom
/// panels, ear = 0) rounds the top corners too, using the same Bezier family.
pub fn notch_contour(width: f32, height: f32, ear: f32, radius: f32, round_top: bool) -> Contour {
    let k = 0.45;
    let r = radius.min(width * 0.5).min(height * 0.5).max(0.);
    if round_top && ear <= 0. {
        return Contour {
            start: (r, 0.),
            segs: vec![
                Seg::Line((width - r, 0.)),
                Seg::Bezier((width - r * k, 0.), (width, r * k), (width, r)),
                Seg::Line((width, height - r)),
                Seg::Bezier((width, height - r * k), (width - r * k, height), (width - r, height)),
                Seg::Line((r, height)),
                Seg::Bezier((r * k, height), (0., height - r * k), (0., height - r)),
                Seg::Line((0., r)),
                Seg::Bezier((0., r * k), (r * k, 0.), (r, 0.)),
            ],
        };
    }
    let r = radius;
    Contour {
        start: (-ear, 0.),
        segs: vec![
            Seg::Bezier((-ear * 0.5, 0.), (0., ear * 0.5), (0., ear)),
            Seg::Line((0., height - r)),
            Seg::Bezier((0., height - r * k), (r * k, height), (r, height)),
            Seg::Line((width - r, height)),
            Seg::Bezier((width - r * k, height), (width, height - r * k), (width, height - r)),
            Seg::Line((width, ear)),
            Seg::Bezier((width, ear * 0.5), (width + ear * 0.5, 0.), (width + ear, 0.)),
            Seg::Line((-ear, 0.)),
        ],
    }
}

fn bezier(p0: P, p1: P, p2: P, p3: P, t: f32) -> P {
    let u = 1. - t;
    let (a, b, c, d) = (u * u * u, 3. * u * u * t, 3. * u * t * t, t * t * t);
    (a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1)
}

/// Flattens the contour to a closed polygon (first point is not repeated).
pub fn flatten(contour: &Contour, steps: usize) -> Vec<P> {
    let mut out = vec![contour.start];
    let mut cur = contour.start;
    for seg in &contour.segs {
        match *seg {
            Seg::Line(p) => {
                out.push(p);
                cur = p;
            }
            Seg::Bezier(a, b, c) => {
                for i in 1..=steps {
                    out.push(bezier(cur, a, b, c, i as f32 / steps as f32));
                }
                cur = c;
            }
        }
    }
    // Drop consecutive duplicates (ear = 0 collapses the ear Beziers) and the closing repeat.
    out.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4);
    while out.len() > 1 {
        let (f, l) = (out[0], out[out.len() - 1]);
        if (f.0 - l.0).abs() < 1e-4 && (f.1 - l.1).abs() < 1e-4 {
            out.pop();
        } else {
            break;
        }
    }
    out
}

/// Region polygon in window pixels, from the same contour. `origin` is the capsule origin in
/// canvas DIPs and `scale` the DIP->pixel factor. Vertices are rounded *outwards* (away from the
/// body centre) so the integer region always contains the anti-aliased contour; the half-pixel
/// outline stroke is therefore never clipped by the region.
pub fn region_polygon(contour: &Contour, origin: P, scale: f32, body_w: f32, body_h: f32, top_bleed_px: i32) -> Vec<(i32, i32)> {
    let cx = body_w * 0.5;
    let cy = body_h * 0.5;
    flatten(contour, 14)
        .into_iter()
        .map(|(x, y)| {
            let px = (origin.0 + x) * scale;
            let py = (origin.1 + y) * scale;
            let ox = if x < cx { px.floor() } else { px.ceil() };
            let oy = if y < cy { py.floor() } else { py.ceil() };
            // The row above the contour covers the screen-edge hairline (it is off-screen when attached).
            let bleed = if y <= 1e-4 { top_bleed_px } else { 0 };
            (ox as i32, oy as i32 - bleed)
        })
        .collect()
}

pub fn polygon_contains(poly: &[(i32, i32)], x: f32, y: f32) -> bool {
    // Even-odd ray cast, boundary counts as inside for our purposes (tolerance applied by caller).
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (poly[i].0 as f32, poly[i].1 as f32);
        let (xj, yj) = (poly[j].0 as f32, poly[j].1 as f32);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Cheap change detector so `SetWindowRgn` only runs when the contour really changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionKey {
    pub w: f32,
    pub h: f32,
    pub ear: f32,
    pub radius: f32,
    pub round_top: bool,
    pub ox: f32,
    pub oy: f32,
    pub scale: f32,
}

// ---------------------------------------------------------------------------------------------
// Win32: DWM backdrop (SetWindowCompositionAttribute) and region from polygon
// ---------------------------------------------------------------------------------------------

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    gradient: u32,
    animation: u32,
}
#[repr(C)]
struct WinCompAttrData {
    attrib: u32,
    data: *mut c_void,
    size: usize,
}

type SetWcaFn = unsafe extern "system" fn(windows::Win32::Foundation::HWND, *mut WinCompAttrData) -> i32;

fn set_wca() -> Option<SetWcaFn> {
    use windows::core::{s, w};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    unsafe {
        let user32 = LoadLibraryW(w!("user32.dll")).ok()?;
        let p = GetProcAddress(user32, s!("SetWindowCompositionAttribute"))?;
        Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, SetWcaFn>(p))
    }
}

/// Applies (or removes) the DWM backdrop for the window region. `tint_abgr` is only used by the
/// OS as a base colour; mynotch draws its own tint in Direct2D, so it passes a near-transparent
/// value. Returns false if the OS entry point is missing or refuses.
pub fn apply_backdrop(hwnd: windows::Win32::Foundation::HWND, backdrop: Backdrop) -> bool {
    let Some(f) = set_wca() else { return false };
    let (state, flags, gradient) = match backdrop {
        Backdrop::Off => (0u32, 0u32, 0u32),
        Backdrop::Blur => (3, 0, 0x00000000),
        Backdrop::Acrylic => (4, 0, 0x01000000),
    };
    let mut policy = AccentPolicy { state, flags, gradient, animation: 0 };
    let mut data = WinCompAttrData { attrib: 19, data: (&mut policy as *mut AccentPolicy).cast(), size: std::mem::size_of::<AccentPolicy>() };
    unsafe { f(hwnd, &mut data) != 0 }
}

/// Sets the window region to the polygon. Ownership of the HRGN passes to the system on success.
pub fn set_region_polygon(hwnd: windows::Win32::Foundation::HWND, poly: &[(i32, i32)]) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{CreatePolygonRgn, DeleteObject, SetWindowRgn, WINDING};
    if poly.len() < 3 {
        return false;
    }
    let pts: Vec<POINT> = poly.iter().map(|&(x, y)| POINT { x, y }).collect();
    unsafe {
        let rgn = CreatePolygonRgn(&pts, WINDING);
        if rgn.0.is_null() {
            return false;
        }
        if SetWindowRgn(hwnd, Some(rgn), false) == 0 {
            let _ = DeleteObject(rgn.into());
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<(f32, f32, f32, f32, bool)> {
        vec![
            (50., 10., 6., 5., false),
            (300., 32., 12., 16., false),
            (560., 360., 12., 27., false),
            (460., 180., 12., 28., false),
            (300., 32., 0., 16., true),
            (540., 280., 0., 24., true),
        ]
    }

    #[test]
    fn material_strings_round_trip_and_default_to_solid() {
        for m in [MaterialMode::Solid, MaterialMode::Lite, MaterialMode::Adaptive] {
            assert_eq!(MaterialMode::from_str(m.as_str()), m);
            assert_eq!(MaterialMode::from_index(m.index()), m);
        }
        assert_eq!(MaterialMode::from_str(""), MaterialMode::Solid);
        assert_eq!(MaterialMode::from_str("garbage"), MaterialMode::Solid);
        assert_eq!(MaterialMode::from_str(" ADAPTIVE "), MaterialMode::Adaptive);
        assert_eq!(MaterialMode::from_index(99), MaterialMode::Solid);
        assert!(!is_light_mode("dark") && !is_light_mode("") && is_light_mode("Light"));
    }

    #[test]
    fn solid_is_never_glass_and_backdrop_off() {
        for stage in 0..=2 {
            for light in [false, true] {
                assert_eq!(LiquidGlassConfig::for_stage(MaterialMode::Solid, stage, light).backdrop, Backdrop::Off);
                assert_eq!(LiquidGlassConfig::for_stage(MaterialMode::Solid, stage, light).tint_alpha, 1.0);
            }
        }
    }

    #[test]
    fn every_fallback_condition_forces_solid() {
        let ok = GlassEnv::default();
        assert_eq!(decide(MaterialMode::Lite, ok), (MaterialMode::Lite, Fallback::None));
        assert_eq!(decide(MaterialMode::Adaptive, ok), (MaterialMode::Adaptive, Fallback::None));
        assert_eq!(decide(MaterialMode::Solid, ok).0, MaterialMode::Solid);
        let cases = [
            (GlassEnv { reduced_transparency: true, ..ok }, Fallback::ReducedTransparency),
            (GlassEnv { high_contrast: true, ..ok }, Fallback::HighContrast),
            (GlassEnv { software: true, ..ok }, Fallback::Software),
            (GlassEnv { backdrop_unavailable: true, ..ok }, Fallback::NoBackdrop),
        ];
        for (env, why) in cases {
            for m in [MaterialMode::Lite, MaterialMode::Adaptive] {
                assert_eq!(decide(m, env), (MaterialMode::Solid, why));
            }
        }
        // High contrast outranks the other reasons; it also pins the dark palette.
        assert_eq!(decide(MaterialMode::Lite, GlassEnv { high_contrast: true, software: true, ..ok }).1, Fallback::HighContrast);
        assert!(effective_light(true, ok));
        assert!(!effective_light(true, GlassEnv { high_contrast: true, ..ok }));
        assert!(!effective_light(false, ok));
    }

    #[test]
    fn adaptive_gets_more_transparent_with_stage_and_lite_is_denser() {
        let a: Vec<_> = (0..=2).map(|s| LiquidGlassConfig::for_stage(MaterialMode::Adaptive, s, false)).collect();
        assert!(a[0].tint_alpha > a[1].tint_alpha && a[1].tint_alpha > a[2].tint_alpha);
        assert!(a.iter().all(|c| c.backdrop == Backdrop::Acrylic));
        let l = LiquidGlassConfig::for_stage(MaterialMode::Lite, 2, false);
        assert_eq!(l.backdrop, Backdrop::Blur);
        assert!(l.tint_alpha > a[2].tint_alpha, "Lite must be a stronger tint than Adaptive stage 2");
        for light in [false, true] {
            for s in 0..=2 {
                for m in [MaterialMode::Lite, MaterialMode::Adaptive] {
                    let c = LiquidGlassConfig::for_stage(m, s, light);
                    assert!(c.tint_alpha > 0.4 && c.tint_alpha < 1.0 && c.outline_width == 1.0);
                }
            }
        }
    }

    #[test]
    fn height_driven_material_is_continuous_and_matches_stages() {
        for m in [MaterialMode::Lite, MaterialMode::Adaptive] {
            for light in [false, true] {
                for (h, stage) in [(10., 0u8), (32., 1), (180., 2), (360., 2)] {
                    let a = LiquidGlassConfig::for_height(m, h, light);
                    let b = LiquidGlassConfig::for_stage(m, stage, light);
                    assert_eq!((a.tint_rgb, a.tint_alpha), (b.tint_rgb, b.tint_alpha), "{m:?} h={h}");
                }
                // No jumps between neighbouring heights (retarget safety).
                let mut prev = LiquidGlassConfig::for_height(m, 5., light);
                for i in 6..400 {
                    let cur = LiquidGlassConfig::for_height(m, i as f32, light);
                    assert!((cur.tint_alpha - prev.tint_alpha).abs() < 0.02, "alpha jump at h={i}");
                    prev = cur;
                }
            }
        }
        assert!(LiquidGlassConfig::for_height(MaterialMode::Adaptive, 100., false).dense().tint_alpha <= 0.94);
        assert_eq!(LiquidGlassConfig::for_height(MaterialMode::Solid, 100., false).dense().tint_alpha, 1.0);
    }

    #[test]
    fn tokens_are_identity_in_dark_and_readable_in_light() {
        for rgb in [0xffffff, 0x000000, 0x9ca3af, 0x2fd4a3, 0x12345] {
            assert_eq!(map_token(rgb, false), rgb);
        }
        // Surface <-> text contrast in light mode.
        let surface = map_token(0x000000, true);
        let text = map_token(0xffffff, true);
        let muted = map_token(0x9ca3af, true);
        assert!(contrast_ratio(text, surface) > 12.0, "text {text:06x} on {surface:06x}");
        assert!(contrast_ratio(muted, surface) >= 4.5, "muted {muted:06x} on {surface:06x}");
        // Accents stay saturated but become readable on light.
        for accent in [0x2fd4a3u32, 0x69d2e5] {
            let m = map_token(accent, true);
            assert!(contrast_ratio(m, surface) >= 3.0, "accent {accent:06x}->{m:06x}");
        }
        // The effective light tint must also keep text readable on top of the glass tint.
        for s in 0..=2 {
            for m in [MaterialMode::Lite, MaterialMode::Adaptive] {
                let c = LiquidGlassConfig::for_stage(m, s, true);
                assert!(contrast_ratio(text, c.tint_rgb) > 12.0);
                let d = LiquidGlassConfig::for_stage(m, s, false);
                assert!(contrast_ratio(0xffffff, d.tint_rgb) > 12.0);
            }
        }
    }

    #[test]
    fn contour_is_closed_symmetric_and_ears_stay_in_footprint() {
        for (w, h, ear, r, round) in all() {
            let c = notch_contour(w, h, ear, r, round);
            let poly = flatten(&c, 14);
            assert!(poly.len() >= 8);
            let (minx, maxx) = poly.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
            let (miny, maxy) = poly.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
            assert!((minx - -ear).abs() < 1e-3 && (maxx - (w + ear)).abs() < 1e-3, "x extent {minx}..{maxx} for {w} ear {ear}");
            assert!(miny.abs() < 1e-3 && (maxy - h).abs() < 1e-3);
            // Left/right symmetry about the body centre (mirror every vertex).
            for p in &poly {
                let m = (w - p.0, p.1);
                assert!(poly.iter().any(|q| (q.0 - m.0).abs() < 0.02 && (q.1 - m.1).abs() < 0.02), "no mirror for {p:?} in {w}x{h} ear {ear}");
            }
        }
    }

    #[test]
    fn region_polygon_contains_the_whole_contour_including_ears() {
        for scale in [1.0f32, 1.25, 1.5, 2.0] {
            for (w, h, ear, r, round) in all() {
                let origin = ((620. - w) / 2., 30.);
                let c = notch_contour(w, h, ear, r, round);
                let region = region_polygon(&c, origin, scale, w, h, 0);
                // Interior points of the notch body and ear must be inside the region.
                let (cx, cy) = ((origin.0 + w / 2.) * scale, (origin.1 + h / 2.) * scale);
                assert!(polygon_contains(&region, cx, cy), "center point outside region");
                if ear > 0. {
                    // Sample inside the ear fillet (below the concave curve)
                    let (ex, ey) = ((origin.0 + ear * 0.2) * scale, (origin.1 + ear * 0.6) * scale);
                    assert!(polygon_contains(&region, ex, ey), "ear point outside region");
                }
                // Region bbox is tight: no square strip beyond the contour footprint (<= 1px growth).
                let minx = region.iter().map(|p| p.0).min().unwrap() as f32;
                let maxx = region.iter().map(|p| p.0).max().unwrap() as f32;
                let miny = region.iter().map(|p| p.1).min().unwrap() as f32;
                let maxy = region.iter().map(|p| p.1).max().unwrap() as f32;
                assert!(minx >= ((origin.0 - ear) * scale).floor() - 0.01 && maxx <= ((origin.0 + w + ear) * scale).ceil() + 0.01);
                assert!(miny >= (origin.1 * scale).floor() - 0.01 && maxy <= ((origin.1 + h) * scale).ceil() + 0.01);
            }
        }
    }

    #[test]
    fn region_excludes_the_concave_hollow_next_to_the_ears() {
        // Hollow = the area between the ear curve and the screen-edge corner; must NOT be blurred.
        let (w, h, ear, r) = (300., 32., 12., 16.);
        let origin = ((620. - w) / 2., 30.);
        let c = notch_contour(w, h, ear, r, false);
        let region = region_polygon(&c, origin, 1.0, w, h, 0);
        // A point at (-ear*0.9, ear*0.9) is under the flare: outside the material.
        let hollow = (origin.0 - ear * 0.9, origin.1 + ear * 0.9);
        assert!(!polygon_contains(&region, hollow.0, hollow.1));
        // A point inside the body is inside.
        assert!(polygon_contains(&region, origin.0 + w / 2., origin.1 + h / 2.));
        // The ear tip row (y just under 0) at x = -ear*0.6 is material.
        assert!(polygon_contains(&region, origin.0 - ear * 0.6, origin.1 + 0.6));
    }

    #[test]
    fn ear_zero_collapses_without_degenerate_vertices() {
        let poly = flatten(&notch_contour(300., 32., 0., 16., false), 14);
        for pair in poly.windows(2) {
            assert!((pair[0].0 - pair[1].0).abs() > 1e-4 || (pair[0].1 - pair[1].1).abs() > 1e-4);
        }
    }

    #[test]
    fn region_key_detects_change_only_when_contour_changes() {
        let a = RegionKey { w: 300., h: 32., ear: 12., radius: 16., round_top: false, ox: 160., oy: 30., scale: 1. };
        let mut b = a;
        assert_eq!(a, b);
        b.w = 300.5;
        assert_ne!(a, b);
    }
}
