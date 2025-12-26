use gpui::{Rgba, rgb, rgba};

pub struct Theme;

impl Theme {
    pub fn bg() -> Rgba {
        rgb(0xf7f8fa)
    }
    pub fn panel() -> Rgba {
        rgb(0xffffff)
    }
    pub fn sidebar() -> Rgba {
        rgb(0xececec)
    }
    pub fn panel_alt() -> Rgba {
        rgb(0xf2f3f7)
    }
    pub fn border() -> Rgba {
        rgb(0xd8dde3)
    }
    pub fn text() -> Rgba {
        rgb(0x243446)
    }
    pub fn muted() -> Rgba {
        rgb(0x7c8a99)
    }
    pub fn accent() -> Rgba {
        rgb(0x2d7fd2)
    }
    pub fn selection_bg() -> Rgba {
        rgba(0x2d7fd233) // accent with low alpha for text selection
    }

    pub fn strong() -> Rgba {
        rgb(0xc02f4d)
    }
}
