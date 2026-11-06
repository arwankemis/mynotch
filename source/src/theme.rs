//! Built-in colour palette for the public build.
//!
//! Custom JSON themes are not part of the public build. Appearance is driven only by
//! the built-in material presets (Solid / Glass Lite / Glass Adaptive) plus the
//! dark/light colour switch.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedPalette {
    pub background: u32,
    pub panel_bg: u32,
    pub card_bg: u32,
    pub border: u32,
    pub accent: u32,
    pub accent_secondary: u32,
    pub text_primary: u32,
    pub text_muted: u32,
    pub sticky_notes: [u32; 5],
}

impl Default for ResolvedPalette {
    fn default() -> Self {
        Self {
            // Solid black notch body.
            background: 0x000000,
            panel_bg: 0x16161d,
            card_bg: 0x1a1a24,
            border: 0x2b2b36,
            accent: 0xd8d8de,
            accent_secondary: 0xd8d8de,
            text_primary: 0xf4f4f6,
            text_muted: 0x92929d,
            sticky_notes: [0xfef08a, 0xbae6fd, 0xbbf7d0, 0xfbcfe8, 0xe9d5ff],
        }
    }
}
