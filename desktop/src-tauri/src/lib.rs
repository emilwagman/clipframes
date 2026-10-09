pub mod element;

/// The element under the pointer right now, for the UI.
#[tauri::command]
fn element_under_pointer() -> Result<element::ElementInfo, element::ReadError> {
    let (x, y) = element::pointer().ok_or(element::ReadError::Unsupported("No pointer position on this system yet.".into()))?;
    element::element_at(x, y)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![element_under_pointer])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
