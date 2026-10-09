pub mod app;
pub mod element;
pub mod picker;
pub mod round;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run()
}
