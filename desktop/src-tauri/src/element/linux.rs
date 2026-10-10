//! Linux: not built yet. The plan is AT-SPI (the `atspi` crate) for elements; the pointer
//! position and screen capture go through the desktop portal on Wayland.

use super::{ElementInfo, ReadError};

pub fn permitted() -> bool {
    true
}

pub fn ask_permission() {}

pub fn pointer() -> Option<(f64, f64)> {
    None
}

pub fn element_at(_x: f64, _y: f64) -> Result<ElementInfo, ReadError> {
    Err(ReadError::Unsupported("Reading elements on Linux is not built yet.".into()))
}

pub fn sleep_idle(_older_than: std::time::Duration) {}

pub fn page_at(x: f64, y: f64, _may_wake: bool) -> String {
    element_full_at(x, y).map(|e| e.url).unwrap_or_default()
}

pub fn element_full_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    element_at(x, y)
}

pub fn foreground() -> Option<super::Foreground> {
    None
}

pub fn element_picked_at(x: f64, y: f64) -> Result<super::Picked, ReadError> {
    element_full_at(x, y).map(|info| super::Picked { info, later: None })
}
