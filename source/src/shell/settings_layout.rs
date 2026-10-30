//! Settings content geometry is shared by paint, scroll and hit testing.
use crate::render::{rect, D2D_RECT_F};
pub(super) struct SettingsLayout {
    pub viewport: D2D_RECT_F,
    pub scroll_y: f32,
    pub max_scroll_y: f32,
}
impl SettingsLayout {
    pub fn compute(width: f32, height: f32, content_bottom: f32, scroll_y: f32) -> Self {
        let viewport = rect(190., 112., (width - 204.).max(1.), (height - 132.).max(1.));
        let max_scroll_y = (content_bottom - viewport.bottom).max(0.);
        Self { viewport, scroll_y: scroll_y.clamp(0., max_scroll_y), max_scroll_y }
    }
    pub fn visible_hit(&self, mut r: D2D_RECT_F) -> Option<D2D_RECT_F> {
        r.top -= self.scroll_y; r.bottom -= self.scroll_y;
        if r.top < self.viewport.top || r.bottom > self.viewport.bottom { return None; }
        r.left = r.left.max(self.viewport.left); r.right = r.right.min(self.viewport.right);
        (r.right > r.left).then_some(r)
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn settings_layout_keeps_last_row_reachable() {
        let l = SettingsLayout::compute(560.,360.,414.,1000.);
        assert_eq!(414.-l.scroll_y,340.);
        assert!(l.visible_hit(rect(448.,374.,94.,30.)).is_some());
    }
    #[test] fn settings_scroll_clamps_after_content_shrinks() {
        assert_eq!(SettingsLayout::compute(560.,360.,300.,99.).scroll_y,0.);
    }
    #[test] fn settings_hit_regions_exclude_clipped_rows() {
        let l=SettingsLayout::compute(560.,360.,414.,0.);
        assert!(l.visible_hit(rect(448.,330.,94.,30.)).is_none());
        assert!(l.visible_hit(rect(448.,122.,94.,30.)).is_some());
    }
}
