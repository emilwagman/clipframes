//! Linux: not built yet. The plan is AT-SPI (the `atspi` crate) for elements; the pointer
//! position and screen capture go through the desktop portal on Wayland.

use super::{ElementInfo, ReadError};

pub fn permitted() -> bool {
    true
}

pub fn pointer() -> Option<(f64, f64)> {
    None
}

pub fn element_at(_x: f64, _y: f64) -> Result<ElementInfo, ReadError> {
    Err(ReadError::Unsupported("Reading elements on Linux is not built yet.".into()))
}

pub fn sleep_idle(_older_than: std::time::Duration) {}
