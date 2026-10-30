use crate::spike::liquid_glass::{LiquidGlassConfig, MaterialMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContourLeakReport {
    pub delta_outside_rgb: u32,
    pub double_outline_detected: bool,
    pub checked_pixels: usize,
    pub max_delta: u32,
    pub left_strip_checked: usize,
    pub right_strip_checked: usize,
    pub bottom_strip_checked: usize,
}

#[derive(Debug, Clone)]
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub pixels: Vec<u32>,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32, origin_x: i32, origin_y: i32, bg_rgb: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            origin_x,
            origin_y,
            pixels: vec![bg_rgb & 0x00ffffff; size],
        }
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        let bx = x + self.origin_x;
        let by = y + self.origin_y;
        if bx >= 0 && bx < self.width as i32 && by >= 0 && by < self.height as i32 {
            let idx = (by as u32 * self.width + bx as u32) as usize;
            Some(self.pixels[idx])
        } else {
            None
        }
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, rgb: u32) {
        let bx = x + self.origin_x;
        let by = y + self.origin_y;
        if bx >= 0 && bx < self.width as i32 && by >= 0 && by < self.height as i32 {
            let idx = (by as u32 * self.width + bx as u32) as usize;
            self.pixels[idx] = rgb & 0x00ffffff;
        }
    }
}

/// Evaluates if a point (x, y) relative to notch top-left (0, 0) is inside
/// the symmetrical concave ear contour geometry.
pub fn is_inside_notch_contour(x: f32, y: f32, width: f32, height: f32, ear: f32, radius: f32) -> bool {
    if y < 0.0 || y > height {
        return false;
    }

    if y < ear {
        // Ear region: Bézier curve from (-ear, 0) to (0, ear)
        // t in [0, 1]: y(t) = ear * (1.5 * t^2 - 0.5 * t^3)
        // Approximate x_left(y) monotonically from -ear to 0.0
        let t = (y / ear).clamp(0.0, 1.0);
        let x_left = -ear * (1.0 - t).powi(2);
        let x_right = width - x_left;
        return x >= x_left && x <= x_right;
    }

    // Body region between y = ear and y = height - radius
    if y <= height - radius {
        return x >= 0.0 && x <= width;
    }

    // Bottom corners region: rounded radius
    let dy = y - (height - radius);
    if x < radius {
        let dx = radius - x;
        dx * dx + dy * dy <= radius * radius
    } else if x > width - radius {
        let dx = x - (width - radius);
        dx * dx + dy * dy <= radius * radius
    } else {
        x >= 0.0 && x <= width
    }
}

/// Renders a frame buffer with the single-contour notch geometry on top of a background.
pub fn render_test_frame_with_notch(
    width: f32,
    height: f32,
    ear: f32,
    radius: f32,
    mode: MaterialMode,
    stage: u8,
    is_light: bool,
    bg_rgb: u32,
) -> FrameBuffer {
    let config = LiquidGlassConfig::for_stage(mode, stage, is_light);
    let pad_x = (ear + 24.0).ceil() as i32;
    let pad_y = 20;
    let fb_w = (width + 2.0 * ear + 60.0).ceil() as u32;
    let fb_h = (height + 40.0).ceil() as u32;

    let mut fb = FrameBuffer::new(fb_w, fb_h, pad_x, pad_y, bg_rgb);

    let min_x = (-ear - 4.0).floor() as i32;
    let max_x = (width + ear + 4.0).ceil() as i32;
    let min_y = 0;
    let max_y = (height + 4.0).ceil() as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            if is_inside_notch_contour(fx, fy, width, height, ear, radius) {
                // Apply material fill with single contour alpha mask
                let blended = blend_colors(bg_rgb, config.tint_rgb, config.tint_alpha);
                fb.set_pixel(x, y, blended);

                // Add specular highlight on top screen edge
                if config.specular_opacity > 0.0 && fy <= 1.5 {
                    let spec_blended = blend_colors(blended, 0xffffff, config.specular_opacity * 0.4);
                    fb.set_pixel(x, y, spec_blended);
                }

                // Check outline distance (exact same contour)
                let is_edge = is_contour_edge(fx, fy, width, height, ear, radius, config.outline_width);
                if is_edge {
                    let out_blended = blend_colors(blended, config.outline_rgb, config.outline_alpha);
                    fb.set_pixel(x, y, out_blended);
                }
            }
        }
    }

    fb
}

fn is_contour_edge(x: f32, y: f32, width: f32, height: f32, ear: f32, radius: f32, thickness: f32) -> bool {
    let step = thickness.max(1.0);
    let neighbors = [
        (x - step, y),
        (x + step, y),
        (x, y - step),
        (x, y + step),
    ];
    for (nx, ny) in neighbors {
        if !is_inside_notch_contour(nx, ny, width, height, ear, radius) {
            return true;
        }
    }
    false
}

fn blend_colors(bg: u32, fg: u32, alpha: f32) -> u32 {
    let a = alpha.clamp(0.0, 1.0);
    let inv_a = 1.0 - a;
    let bg_r = ((bg >> 16) & 0xff) as f32;
    let bg_g = ((bg >> 8) & 0xff) as f32;
    let bg_b = (bg & 0xff) as f32;

    let fg_r = ((fg >> 16) & 0xff) as f32;
    let fg_g = ((fg >> 8) & 0xff) as f32;
    let fg_b = (fg & 0xff) as f32;

    let r = (bg_r * inv_a + fg_r * a).round() as u32;
    let g = (bg_g * inv_a + fg_g * a).round() as u32;
    let b = (bg_b * inv_a + fg_b * a).round() as u32;

    (r << 16) | (g << 8) | b
}

fn rgb_delta(c1: u32, c2: u32) -> u32 {
    let r1 = ((c1 >> 16) & 0xff) as i32;
    let g1 = ((c1 >> 8) & 0xff) as i32;
    let b1 = (c1 & 0xff) as i32;

    let r2 = ((c2 >> 16) & 0xff) as i32;
    let g2 = ((c2 >> 8) & 0xff) as i32;
    let b2 = (c2 & 0xff) as i32;

    ((r1 - r2).abs() + (g1 - g2).abs() + (b1 - b2).abs()) as u32
}

/// Inspects exterior pixels to verify ZERO box leakage and check for double outlines.
/// Strictly excludes concave ear areas and anti-aliased edge pixels.
pub fn inspect_frame_contour_leak(
    frame: &FrameBuffer,
    width: f32,
    height: f32,
    ear: f32,
    _radius: f32,
    expected_bg: u32,
) -> ContourLeakReport {
    let mut total_delta = 0u32;
    let mut max_delta = 0u32;
    let mut checked_pixels = 0usize;
    let mut left_checked = 0usize;
    let mut right_checked = 0usize;
    let mut bottom_checked = 0usize;

    let bg = expected_bg & 0x00ffffff;

    // 1. Left side strip (strictly outside vertical body wall for y >= ear + 2.0)
    let y_start = (ear + 2.0).ceil() as i32;
    let y_end = (height + 2.0).ceil() as i32;
    let x_left_min = (-ear - 16.0).floor() as i32;
    let x_left_max = -2;

    for y in y_start..=y_end {
        for x in x_left_min..=x_left_max {
            if let Some(pixel) = frame.get_pixel(x, y) {
                let d = rgb_delta(pixel, bg);
                total_delta += d;
                if d > max_delta {
                    max_delta = d;
                }
                checked_pixels += 1;
                left_checked += 1;
            }
        }
    }

    // 2. Right side strip (strictly outside vertical body wall for y >= ear + 2.0)
    let x_right_min = (width + 2.0).ceil() as i32;
    let x_right_max = (width + ear + 16.0).ceil() as i32;

    for y in y_start..=y_end {
        for x in x_right_min..=x_right_max {
            if let Some(pixel) = frame.get_pixel(x, y) {
                let d = rgb_delta(pixel, bg);
                total_delta += d;
                if d > max_delta {
                    max_delta = d;
                }
                checked_pixels += 1;
                right_checked += 1;
            }
        }
    }

    // 3. Bottom exterior strip (strictly below bottom contour)
    let y_bot_min = (height + 2.0).ceil() as i32;
    let y_bot_max = (height + 16.0).ceil() as i32;
    let x_bot_min = (-ear).floor() as i32;
    let x_bot_max = (width + ear).ceil() as i32;

    for y in y_bot_min..=y_bot_max {
        for x in x_bot_min..=x_bot_max {
            if let Some(pixel) = frame.get_pixel(x, y) {
                let d = rgb_delta(pixel, bg);
                total_delta += d;
                if d > max_delta {
                    max_delta = d;
                }
                checked_pixels += 1;
                bottom_checked += 1;
            }
        }
    }

    // 4. Double outline detection:
    // Scan horizontal line through middle of notch body.
    // Count prominent color gradient transitions between exterior and interior.
    let mid_y = (height * 0.5).round() as i32;
    let mut left_peaks = 0;
    let mut right_peaks = 0;
    let mut prev_d = 0i32;

    for x in (-ear as i32)..((width * 0.5) as i32) {
        if let Some(pix) = frame.get_pixel(x, mid_y) {
            let cur_d = rgb_delta(pix, bg) as i32;
            if cur_d > 10 && prev_d <= 10 {
                left_peaks += 1;
            }
            prev_d = cur_d;
        }
    }

    prev_d = 0;
    for x in (((width * 0.5) as i32)..=(width + ear) as i32).rev() {
        if let Some(pix) = frame.get_pixel(x, mid_y) {
            let cur_d = rgb_delta(pix, bg) as i32;
            if cur_d > 10 && prev_d <= 10 {
                right_peaks += 1;
            }
            prev_d = cur_d;
        }
    }

    let double_outline_detected = left_peaks > 1 || right_peaks > 1;

    ContourLeakReport {
        delta_outside_rgb: total_delta,
        double_outline_detected,
        checked_pixels,
        max_delta,
        left_strip_checked: left_checked,
        right_strip_checked: right_checked,
        bottom_strip_checked: bottom_checked,
    }
}
