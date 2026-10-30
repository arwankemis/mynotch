//! Shell side of Liquid Glass: effective-material decision, DWM backdrop sync, the exact-contour
//! window region and the optional real-process benchmark log.
//!
//! One contour feeds everything: `glass::notch_contour` -> Direct2D path (material, alpha mask,
//! outline) and -> region polygon (DWM blur mask, input clip, hit-testing). The region is
//! re-derived from the *live spring geometry* inside `paint`, so material and region never
//! disagree for a frame, including while a morph is being retargeted.

use super::*;
use crate::glass::{self, Backdrop, Fallback, GlassEnv, LiquidGlassConfig, MaterialMode, RegionKey};
use std::cell::Cell;
use std::io::Write;

pub(super) struct Bench {
    file: std::fs::File,
    t0: Instant,
    pub paints: u64,
}

impl Bench {
    pub fn open(path: &str) -> Option<Self> {
        let mut file = std::fs::File::create(path).ok()?;
        let _ = writeln!(file, "t_ms,paint_ms,height,material,backdrop,region_sets");
        Some(Self { file, t0: Instant::now(), paints: 0 })
    }
    pub fn frame(&mut self, paint_ms: f64, height: f32, material: &str, backdrop: &str, region_sets: u32) {
        self.paints += 1;
        let _ = writeln!(self.file, "{:.3},{:.3},{:.1},{},{},{}", self.t0.elapsed().as_secs_f64() * 1000., paint_ms, height, material, backdrop, region_sets);
    }
    pub fn note(&mut self, text: &str) {
        let _ = writeln!(self.file, "# {:.3} {}", self.t0.elapsed().as_secs_f64() * 1000., text);
    }
}

pub(super) struct GlassRt {
    pub reduced: bool,
    pub high_contrast: bool,
    pub warp: bool,
    /// Set when the OS refused the backdrop: the shell then behaves as Solid (real fallback).
    pub failed: Cell<bool>,
    pub backdrop: Cell<Backdrop>,
    pub region: Cell<Option<RegionKey>>,
    pub region_sets: Cell<u32>,
    /// QA-only `--glass-backdrop=blur|acrylic|none` override used by the mechanism investigation.
    pub override_backdrop: Option<Backdrop>,
    pub bench: Option<RefCell<Bench>>,
}

impl GlassRt {
    pub fn new(args: &[String]) -> Self {
        let override_backdrop = args.iter().find_map(|a| a.strip_prefix("--glass-backdrop=")).and_then(|v| match v {
            "blur" => Some(Backdrop::Blur),
            "acrylic" => Some(Backdrop::Acrylic),
            "none" => Some(Backdrop::Off),
            _ => None,
        });
        let bench = args.iter().find_map(|a| a.strip_prefix("--bench-log=")).and_then(Bench::open).map(RefCell::new);
        Self {
            reduced: render::is_reduced_transparency(),
            high_contrast: render::is_high_contrast(),
            warp: false,
            failed: Cell::new(false),
            backdrop: Cell::new(Backdrop::Off),
            region: Cell::new(None),
            region_sets: Cell::new(0),
            override_backdrop,
            bench,
        }
    }
}

impl Shell {
    pub(super) fn glass_env(&self) -> GlassEnv {
        // Preview windows skip the region entirely unless --edge-check is passed, and without a
        // region DWM would blur the whole canvas rectangle: never allow glass there.
        let preview_without_region = self.preview && !std::env::args().any(|a| a == "--edge-check");
        GlassEnv {
            reduced_transparency: self.glass.reduced,
            high_contrast: self.glass.high_contrast,
            software: self.glass.warp || self.settings.software_renderer,
            backdrop_unavailable: preview_without_region || self.glass.failed.get(),
        }
    }

    pub(super) fn glass_fallback(&self) -> Fallback {
        glass::decide(MaterialMode::from_str(&self.settings.theme_material), self.glass_env()).1
    }

    pub(super) fn ui_light(&self) -> bool {
        glass::effective_light(glass::is_light_mode(&self.settings.theme_color), self.glass_env())
    }

    /// The material for a body of `height` DIPs, or None when Solid is in effect.
    pub(super) fn glass_config(&self, height: f32) -> Option<LiquidGlassConfig> {
        let (mode, _) = glass::decide(MaterialMode::from_str(&self.settings.theme_material), self.glass_env());
        if !mode.is_glass() {
            return None;
        }
        let cfg = LiquidGlassConfig::for_height(mode, height, self.ui_light());
        Some(if self.settings_open || self.tab == "Reminder" && self.hover_stage >= 2 { cfg.dense() } else { cfg })
    }

    pub(super) fn refresh_glass_env(&mut self) {
        self.glass.reduced = render::is_reduced_transparency();
        self.glass.high_contrast = render::is_high_contrast();
    }

    /// (width, height, ear, radius, round_top, capsule origin) of the contour being drawn *now*.
    pub(super) fn contour_params(&self) -> (f32, f32, f32, f32, bool, (f32, f32)) {
        if let Some(c)=self.frame_contour{return(c.width,c.height,c.ear,c.radius,c.round_top,c.origin_dip)}
        let (w, h, r) = self.geometry();
        let attached = !self.settings.bottom && self.settings.edge_gap == 0;
        let ear = if attached {
            match &self.morph {
                Some(m) => m.ear(),
                None => self.target_ear(),
            }
        } else {
            0.
        };
        (w, h, ear, r, !attached, self.capsule_origin())
    }

    /// Exact-contour region (the DWM blur mask). A no-op when the contour did not change.
    pub(super) fn apply_glass_region(&self) {
        let (w, h, ear, r, round_top, origin) = self.contour_params();
        let key = RegionKey { w, h, ear, radius: r, round_top, ox: origin.0, oy: origin.1, scale: self.scale };
        if self.glass.region.get() == Some(key) {
            return;
        }
        let contour = glass::notch_contour(w, h, ear, r, round_top);
        let bleed = if round_top { 0 } else { 1 };
        let poly = glass::region_polygon(&contour, origin, self.scale, w, h, bleed);
        if glass::set_region_polygon(self.hwnd, &poly) {
            self.glass.region.set(Some(key));
            self.glass.region_sets.set(self.glass.region_sets.get().wrapping_add(1));
        }
    }

    /// Idempotent: brings the DWM backdrop, the palette flag and the region in line with the
    /// settings and the environment. Cheap when nothing changed.
    pub(super) fn sync_glass(&self) {
        render::LIGHT_UI.store(self.ui_light(), Ordering::Relaxed);
        let want = match self.glass_config(self.geometry().1) {
            Some(cfg) => self.glass.override_backdrop.unwrap_or(cfg.backdrop),
            None => Backdrop::Off,
        };
        let have = self.glass.backdrop.get();
        if want == have {
            return;
        }
        // Host backdrop is clipped on the compositor visual, not the HWND.
        self.glass.backdrop.set(want);
        self.glass.region.set(None);
        self.update_window_region();
        if let Some(b) = &self.glass.bench {
            b.borrow_mut().note(&format!("backdrop {:?} -> {:?}", have, self.glass.backdrop.get()));
        }
    }

    pub(super) fn glass_active(&self) -> bool {
        self.glass.backdrop.get() != Backdrop::Off
    }

    /// Called after Material or Gelap/Terang changed in Settings: live, no restart.
    pub(super) fn glass_settings_changed(&mut self) {
        self.sync_glass();
        self.invalidate();
    }
}
