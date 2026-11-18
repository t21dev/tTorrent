//! Icons and fonts compiled into the binary.

use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../../../assets/icons/", $name, ".svg")))),*]
    };
}

const ICONS: &[(&str, &[u8])] = icons![
    "plus",
    "magnet",
    "play",
    "pause",
    "trash",
    "gauge",
    "search",
    "sun",
    "moon",
    "file",
    "arrow-down",
    "arrow-up",
    "check",
    "x",
    "alert",
    "zap",
    "tag",
    "server",
    "folder",
    "grid",
    "clock",
    "chevron-down",
    "chevron-up",
    "chevron-right",
    "minus",
    "square",
    "drive",
    "globe",
    "chip",
    "plug",
    "sliders",
    "stop",
    "spinner",
    "list",
    "copy",
    "pencil",
    "refresh",
    "bar-chart",
    "rss",
    "keyboard",
    "sprout",
];

pub const FONTS: &[&[u8]] = &[
    include_bytes!("../../../assets/fonts/InstrumentSans-Regular.ttf"),
    include_bytes!("../../../assets/fonts/InstrumentSans-Medium.ttf"),
    include_bytes!("../../../assets/fonts/InstrumentSans-SemiBold.ttf"),
    include_bytes!("../../../assets/fonts/DMMono-Regular.ttf"),
    include_bytes!("../../../assets/fonts/DMMono-Medium.ttf"),
    include_bytes!("../../../assets/fonts/SchibstedGrotesk-Bold.ttf"),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let name = path
            .strip_prefix("icons/")
            .and_then(|p| p.strip_suffix(".svg"))
            .unwrap_or(path);
        Ok(ICONS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, b)| Cow::Borrowed(*b)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .map(|(n, _)| SharedString::from(format!("icons/{n}.svg")))
            .filter(|p| p.starts_with(path))
            .collect())
    }
}
