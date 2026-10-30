use crate::spike::liquid_glass::*;
use crate::spike::visual_qa::*;

#[test]
fn test_zero_rgb_leak_outside_ear_and_body_contour() {
    let frame = render_test_frame_with_notch(
        120.0,
        36.0,
        12.0,
        12.0,
        MaterialMode::Adaptive,
        0,
        false,
        0xff00ff, // High-contrast magenta
    );
    let report = inspect_frame_contour_leak(&frame, 120.0, 36.0, 12.0, 12.0, 0xff00ff);
    assert_eq!(report.delta_outside_rgb, 0);
    assert!(!report.double_outline_detected);
    assert!(report.checked_pixels > 0);
}

#[test]
fn test_visual_qa_across_all_stages_and_themes() {
    let stages = [
        (0u8, 120.0f32, 36.0f32, 12.0f32, 12.0f32),  // Stage 0
        (1u8, 360.0f32, 48.0f32, 12.0f32, 14.0f32),  // Stage 1
        (2u8, 736.0f32, 260.0f32, 16.0f32, 18.0f32), // Stage 2
    ];

    let themes = [
        false, // Gelap (Dark)
        true,  // Terang (Light)
    ];

    let backgrounds = [
        0xff00ff, // Magenta
        0x00ff00, // Neon Green
        0xffffff, // White
        0x000000, // Black
    ];

    let modes = [
        MaterialMode::Solid,
        MaterialMode::Lite,
        MaterialMode::Adaptive,
    ];

    for mode in modes {
        for &(stage, w, h, ear, rad) in &stages {
            for &is_light in &themes {
                for &bg in &backgrounds {
                    let frame = render_test_frame_with_notch(w, h, ear, rad, mode, stage, is_light, bg);
                    let report = inspect_frame_contour_leak(&frame, w, h, ear, rad, bg);
                    assert_eq!(
                        report.delta_outside_rgb, 0,
                        "Leaked pixels detected! mode={:?} stage={} is_light={} bg=0x{:06x} delta={}",
                        mode, stage, is_light, bg, report.delta_outside_rgb
                    );
                    assert!(
                        !report.double_outline_detected,
                        "Double outline detected! mode={:?} stage={} is_light={}",
                        mode, stage, is_light
                    );
                    assert!(report.checked_pixels > 0);
                }
            }
        }
    }
}

#[test]
fn test_visual_qa_catches_simulated_box_leak_regression() {
    let mut frame = render_test_frame_with_notch(
        120.0,
        36.0,
        12.0,
        12.0,
        MaterialMode::Adaptive,
        0,
        false,
        0xff00ff,
    );
    // Simulate a box leak into the left side strip (x = -4, y = 20)
    frame.set_pixel(-4, 20, 0x112233);

    let report = inspect_frame_contour_leak(&frame, 120.0, 36.0, 12.0, 12.0, 0xff00ff);
    assert!(report.delta_outside_rgb > 0, "Inspector should have caught the box leak!");
    assert_eq!(report.max_delta, 476);
}
