mod git;

#[tauri::command]
async fn scan_project(path: String) -> Result<git::Project, String> {
    tauri::async_runtime::spawn_blocking(move || git::scan_project(&path))
        .await
        .map_err(|error| format!("project scan failed: {error}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_project])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
