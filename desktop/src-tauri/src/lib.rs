pub mod app;
pub mod element;
pub mod picker;
pub mod round;
pub mod settings;
pub mod store;
pub mod updates;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run()
}
