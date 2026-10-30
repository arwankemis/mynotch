use crate::spike::liquid_glass::*;

#[test]
fn test_single_contour_matches_alpha_mask_and_outline_bounds() {
    let bounds = compute_notch_contour_bounds(120.0, 36.0, 12.0);
    assert_eq!(bounds.mask_rect, bounds.outline_rect);
    assert_eq!(bounds.mask_rect.left, -12.0);
    assert_eq!(bounds.mask_rect.right, 132.0);
    assert_eq!(bounds.mask_rect.top, 0.0);
    assert_eq!(bounds.mask_rect.bottom, 36.0);
    assert_eq!(bounds.mask_rect.width(), 144.0);
    assert_eq!(bounds.mask_rect.height(), 36.0);
}

#[test]
fn test_material_mode_transitions_and_fallbacks() {
    assert_eq!(MaterialMode::from_str("solid"), MaterialMode::Solid);
    assert_eq!(MaterialMode::from_str("SOLID"), MaterialMode::Solid);
    assert_eq!(MaterialMode::from_str("lite"), MaterialMode::Lite);
    assert_eq!(MaterialMode::from_str("adaptive"), MaterialMode::Adaptive);
    assert_eq!(MaterialMode::from_str("unknown"), MaterialMode::Solid);

    assert_eq!(MaterialMode::Solid.as_str(), "solid");
    assert_eq!(MaterialMode::Lite.as_str(), "lite");
    assert_eq!(MaterialMode::Adaptive.as_str(), "adaptive");

    assert!(!MaterialMode::Solid.is_glass());
    assert!(MaterialMode::Lite.is_glass());
    assert!(MaterialMode::Adaptive.is_glass());
}

#[test]
fn test_stage_configurations_match_specification() {
    let solid = LiquidGlassConfig::for_stage(MaterialMode::Solid, 0, false);
    assert_eq!(solid.blur_radius, 0.0);
    assert_eq!(solid.tint_alpha, 1.0);
    assert_eq!(solid.specular_opacity, 0.0);

    let lite = LiquidGlassConfig::for_stage(MaterialMode::Lite, 1, false);
    assert_eq!(lite.blur_radius, 10.0);
    assert_eq!(lite.tint_alpha, 0.86);

    let adapt0 = LiquidGlassConfig::for_stage(MaterialMode::Adaptive, 0, false);
    let adapt1 = LiquidGlassConfig::for_stage(MaterialMode::Adaptive, 1, false);
    let adapt2 = LiquidGlassConfig::for_stage(MaterialMode::Adaptive, 2, false);

    assert_eq!(adapt0.blur_radius, 14.0);
    assert_eq!(adapt1.blur_radius, 24.0);
    assert_eq!(adapt2.blur_radius, 32.0);
    assert!(adapt0.tint_alpha > adapt1.tint_alpha);
    assert!(adapt1.tint_alpha > adapt2.tint_alpha);
}

#[test]
fn test_renderer_clean_creation_and_destruction() {
    let mut renderer = LiquidGlassRenderer::new();
    renderer.clear_cache();
    drop(renderer);
}
