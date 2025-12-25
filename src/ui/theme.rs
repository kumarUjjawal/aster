use gpui::{rgb, Rgba};

pub struct Theme;

impl Theme {
    pub fn bg() -> Rgba {
        rgb(0x000000)
    }
    pub fn panel() -> Rgba {
        rgb(0x1f1f1f)
    }
    pub fn panel_alt() -> Rgba {
        rgb(0x181818)
    }
    pub fn border() -> Rgba {
        rgb(0x2f2f2f)
    }
    pub fn text() -> Rgba {
        rgb(0xffffff)
    }
    pub fn muted() -> Rgba {
        rgb(0x9a9a9a)
    }
    pub fn accent() -> Rgba {
        rgb(0x4e9cff)
    }
    pub fn warn() -> Rgba {
        rgb(0xffb347)
    }
}
