pub mod app;
pub mod element;
pub mod picker;
pub mod places;
pub mod round;
pub mod settings;
pub mod shot;
pub mod store;
pub mod tab;
pub mod telemetry;
pub mod updates;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run()
}
