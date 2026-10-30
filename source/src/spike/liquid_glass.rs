use serde::{Deserialize, Serialize};
use windows::core::Result;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContourRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl ContourRect {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContourBounds {
    pub mask_rect: ContourRect,
    pub outline_rect: ContourRect,
    pub width: f32,
    pub height: f32,
    pub ear: f32,
}

/// Computes the exact single-contour bounds shared by both the alpha mask layer
/// and the border outline path.
pub fn compute_notch_contour_bounds(width: f32, height: f32, ear: f32) -> ContourBounds {
    let r = ContourRect {
        left: -ear,
        top: 0.0,
        right: width + ear,
        bottom: height,
    };
    ContourBounds {
        mask_rect: r,
        outline_rect: r,
        width,
        height,
        ear,
    }
}

#[derive(Debug, Clone)]
pub struct LiquidGlassConfig {
    pub mode: MaterialMode,
    pub stage: u8,
    pub is_light: bool,
    pub blur_radius: f32,
    pub tint_rgb: u32,
    pub tint_alpha: f32,
    pub outline_rgb: u32,
    pub outline_alpha: f32,
    pub outline_width: f32,
    pub specular_opacity: f32,
}

impl LiquidGlassConfig {
    pub fn for_stage(mode: MaterialMode, stage: u8, is_light: bool) -> Self {
        match mode {
            MaterialMode::Solid => Self {
                mode,
                stage,
                is_light,
                blur_radius: 0.0,
                tint_rgb: if is_light { 0xf2f4f7 } else { 0x000000 },
                tint_alpha: 1.0,
                outline_rgb: if is_light { 0xd0d5dd } else { 0x222226 },
                outline_alpha: 0.8,
                outline_width: 1.0,
                specular_opacity: 0.0,
            },
            MaterialMode::Lite => Self {
                mode,
                stage,
                is_light,
                blur_radius: 10.0,
                tint_rgb: if is_light { 0xf0f3f6 } else { 0x101318 },
                tint_alpha: 0.86,
                outline_rgb: if is_light { 0xffffff } else { 0x363d4a },
                outline_alpha: 0.45,
                outline_width: 1.0,
                specular_opacity: 0.15,
            },
            MaterialMode::Adaptive => {
                let (blur_radius, tint_alpha, specular) = match stage {
                    0 => (14.0, 0.78, 0.18),
                    1 => (24.0, 0.70, 0.25),
                    _ => (32.0, 0.65, 0.32),
                };
                let tint_rgb = if is_light {
                    match stage {
                        0 => 0xf5f7fa,
                        1 => 0xeef2f6,
                        _ => 0xe6ebf2,
                    }
                } else {
                    match stage {
                        0 => 0x0c0e12,
                        1 => 0x141820,
                        _ => 0x181e28,
                    }
                };
                let outline_rgb = if is_light { 0xffffff } else { 0x485264 };
                Self {
                    mode,
                    stage,
                    is_light,
                    blur_radius,
                    tint_rgb,
                    tint_alpha,
                    outline_rgb,
                    outline_alpha: 0.50,
                    outline_width: 1.0,
                    specular_opacity: specular,
                }
            }
        }
    }
}

/// Builds the unified symmetrical concave ear contour path geometry.
/// Single source of truth for both mask layer and outline stroke.
pub unsafe fn create_notch_path_geometry(
    factory: &ID2D1Factory1,
    width: f32,
    height: f32,
    ear: f32,
    radius: f32,
) -> Result<ID2D1PathGeometry> {
    unsafe {
        let path = factory.CreatePathGeometry()?;
        let sink = path.Open()?;
        sink.BeginFigure(
            windows_numerics::Vector2 { X: -ear, Y: 0.0 },
            D2D1_FIGURE_BEGIN_FILLED,
        );
        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
            point1: windows_numerics::Vector2 { X: -ear * 0.5, Y: 0.0 },
            point2: windows_numerics::Vector2 { X: 0.0, Y: ear * 0.5 },
            point3: windows_numerics::Vector2 { X: 0.0, Y: ear },
        });
        sink.AddLine(windows_numerics::Vector2 {
            X: 0.0,
            Y: height - radius,
        });
        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
            point1: windows_numerics::Vector2 {
                X: 0.0,
                Y: height - radius * 0.45,
            },
            point2: windows_numerics::Vector2 {
                X: radius * 0.45,
                Y: height,
            },
            point3: windows_numerics::Vector2 {
                X: radius,
                Y: height,
            },
        });
        sink.AddLine(windows_numerics::Vector2 {
            X: width - radius,
            Y: height,
        });
        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
            point1: windows_numerics::Vector2 {
                X: width - radius * 0.45,
                Y: height,
            },
            point2: windows_numerics::Vector2 {
                X: width,
                Y: height - radius * 0.45,
            },
            point3: windows_numerics::Vector2 {
                X: width,
                Y: height - radius,
            },
        });
        sink.AddLine(windows_numerics::Vector2 { X: width, Y: ear });
        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
            point1: windows_numerics::Vector2 {
                X: width,
                Y: ear * 0.5,
            },
            point2: windows_numerics::Vector2 {
                X: width + ear * 0.5,
                Y: 0.0,
            },
            point3: windows_numerics::Vector2 {
                X: width + ear,
                Y: 0.0,
            },
        });
        sink.AddLine(windows_numerics::Vector2 { X: -ear, Y: 0.0 });
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close()?;
        Ok(path.into())
    }
}

/// Isolated LiquidGlassRenderer harness managing reusable layer masks and backdrop shaders.
pub struct LiquidGlassRenderer {
    cached_layer: Option<ID2D1Layer>,
    #[allow(dead_code)]
    cached_factory: Option<ID2D1Factory1>,
}

impl Default for LiquidGlassRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl LiquidGlassRenderer {
    pub fn new() -> Self {
        Self {
            cached_layer: None,
            cached_factory: None,
        }
    }

    pub fn with_factory(factory: ID2D1Factory1) -> Self {
        Self {
            cached_layer: None,
            cached_factory: Some(factory),
        }
    }

    pub fn clear_cache(&mut self) {
        self.cached_layer = None;
    }

    /// Renders the backdrop using a single contour geometry mask.
    pub unsafe fn render_notch(
        &mut self,
        context: &ID2D1DeviceContext,
        factory: &ID2D1Factory1,
        width: f32,
        height: f32,
        ear: f32,
        radius: f32,
        config: &LiquidGlassConfig,
    ) -> Result<()> {
        unsafe {
            let path = create_notch_path_geometry(factory, width, height, ear, radius)?;

            if config.mode == MaterialMode::Solid {
                let brush = context.CreateSolidColorBrush(
                    &d2d_color(config.tint_rgb, config.tint_alpha),
                    None,
                )?;
                context.FillGeometry(&path, &brush, None);
                let outline_brush = context.CreateSolidColorBrush(
                    &d2d_color(config.outline_rgb, config.outline_alpha),
                    None,
                )?;
                context.DrawGeometry(&path, &outline_brush, config.outline_width, None);
                return Ok(());
            }

            // Reuse cached layer or create once
            if self.cached_layer.is_none() {
                self.cached_layer = context.CreateLayer(None).ok();
            }

            if let Some(layer) = &self.cached_layer {
                let bounds = compute_notch_contour_bounds(width, height, ear);
                let layer_params = D2D1_LAYER_PARAMETERS1 {
                    contentBounds: D2D_RECT_F {
                        left: bounds.mask_rect.left - 4.0,
                        top: bounds.mask_rect.top - 4.0,
                        right: bounds.mask_rect.right + 4.0,
                        bottom: bounds.mask_rect.bottom + 4.0,
                    },
                    geometricMask: std::mem::ManuallyDrop::new(Some(path.clone().into())),
                    maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                    maskTransform: windows_numerics::Matrix3x2 {
                        M11: 1.0,
                        M12: 0.0,
                        M21: 0.0,
                        M22: 1.0,
                        M31: 0.0,
                        M32: 0.0,
                    },
                    opacity: 1.0,
                    opacityBrush: std::mem::ManuallyDrop::new(None),
                    layerOptions: D2D1_LAYER_OPTIONS1_NONE,
                };

                context.PushLayer(&layer_params, layer);

                // Backdrop tint base
                let bg_brush = context.CreateSolidColorBrush(
                    &d2d_color(config.tint_rgb, config.tint_alpha),
                    None,
                )?;
                context.FillGeometry(&path, &bg_brush, None);

                // Specular top highlight if active
                if config.specular_opacity > 0.0 {
                    let spec_brush = context.CreateSolidColorBrush(
                        &d2d_color(0xffffff, config.specular_opacity * 0.4),
                        None,
                    )?;
                    let top_line_rect = D2D_RECT_F {
                        left: -ear,
                        top: 0.0,
                        right: width + ear,
                        bottom: 1.5,
                    };
                    context.FillRectangle(&top_line_rect, &spec_brush);
                }

                context.PopLayer();

                // Outline using exact same path geometry
                let outline_brush = context.CreateSolidColorBrush(
                    &d2d_color(config.outline_rgb, config.outline_alpha),
                    None,
                )?;
                context.DrawGeometry(&path, &outline_brush, config.outline_width, None);
            }

            Ok(())
        }
    }
}

fn d2d_color(rgb: u32, alpha: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: ((rgb >> 16) & 255) as f32 / 255.0,
        g: ((rgb >> 8) & 255) as f32 / 255.0,
        b: (rgb & 255) as f32 / 255.0,
        a: alpha.clamp(0.0, 1.0),
    }
}
