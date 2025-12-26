use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub struct AsterAssetSource;

impl AsterAssetSource {
    pub fn new() -> Self {
        Self
    }
}

static PANEL_LEFT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/panel-left.svg"
));
static PANEL_RIGHT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/panel-right.svg"
));
static LAYOUT_DASHBOARD: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/layout-dashboard.svg"
));
static CLOSE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/close.svg"
));
static INFO: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/info.svg"
));
static CIRCLE_CHECK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/circle-check.svg"
));
static CIRCLE_X: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/circle-x.svg"
));
static TRIANGLE_ALERT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/triangle-alert.svg"
));
static FOLDER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/folder.svg"
));
static FILE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/file.svg"
));
static CHEVRON_RIGHT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/chevron-right.svg"
));
static CHEVRON_DOWN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/chevron-down.svg"
));

impl AssetSource for AsterAssetSource {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes = match path {
            "icons/circle-check.svg" => CIRCLE_CHECK,
            "icons/circle-x.svg" => CIRCLE_X,
            "icons/close.svg" => CLOSE,
            "icons/info.svg" => INFO,
            "icons/layout-dashboard.svg" => LAYOUT_DASHBOARD,
            "icons/panel-left.svg" => PANEL_LEFT,
            "icons/panel-right.svg" => PANEL_RIGHT,
            "icons/triangle-alert.svg" => TRIANGLE_ALERT,
            "icons/folder.svg" => FOLDER,
            "icons/file.svg" => FILE,
            "icons/chevron-right.svg" => CHEVRON_RIGHT,
            "icons/chevron-down.svg" => CHEVRON_DOWN,
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let assets = [
            "icons/circle-check.svg",
            "icons/circle-x.svg",
            "icons/close.svg",
            "icons/info.svg",
            "icons/layout-dashboard.svg",
            "icons/panel-left.svg",
            "icons/panel-right.svg",
            "icons/triangle-alert.svg",
            "icons/folder.svg",
            "icons/file.svg",
            "icons/chevron-right.svg",
            "icons/chevron-down.svg",
        ];

        if path.is_empty() || path == "." {
            return Ok(assets.iter().map(|p| (*p).into()).collect());
        }

        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };

        Ok(assets
            .iter()
            .filter(|p| p.starts_with(&prefix))
            .map(|p| (*p).into())
            .collect())
    }
}
