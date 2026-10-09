//! Linux input for a picking round: not built yet. X11 can grab the pointer; Wayland does not
//! let an app watch or take input outside its own windows, so picking there needs another way in.

use super::Input;

pub struct Source;

impl Source {
    pub fn start(_on_input: impl Fn(Input) -> bool + Send + 'static) -> Result<Source, String> {
        Err("Picking is not built for Linux yet.".into())
    }
}
