mod git;
mod run;
mod watch;

use tauri::Manager;

#[tauri::command]
async fn scan_project(path: String) -> Result<git::Project, String> {
    tauri::async_runtime::spawn_blocking(move || git::scan_project(&path))
        .await
        .map_err(|error| format!("project scan failed: {error}"))?
}

#[tauri::command]
fn start_project_watch(
    app: tauri::AppHandle,
    state: tauri::State<'_, watch::WatchManager>,
    path: String,
) -> Result<(), String> {
    state.start(app, &path)
}

#[tauri::command]
fn stop_project_watch(
    state: tauri::State<'_, watch::WatchManager>,
    project_id: String,
) -> Result<(), String> {
    state.stop(&project_id)
}

#[tauri::command]
fn get_run_settings(app: tauri::AppHandle, project_id: String) -> Result<run::RunSettings, String> {
    run::load_settings(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        &project_id,
    )
}

#[tauri::command]
fn save_run_script(
    app: tauri::AppHandle,
    project_id: String,
    script: run::ScriptInput,
) -> Result<run::RunSettings, String> {
    run::save_script(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        &project_id,
        script,
    )
}

#[tauri::command]
fn delete_run_script(
    app: tauri::AppHandle,
    project_id: String,
    script_id: String,
) -> Result<run::RunSettings, String> {
    run::delete_script(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        &project_id,
        &script_id,
    )
}

#[tauri::command]
fn run_script(
    app: tauri::AppHandle,
    state: tauri::State<'_, run::RunManager>,
    request: run::RunRequest,
) -> Result<run::RunStarted, String> {
    state.start(app, request)
}

#[tauri::command]
fn cancel_run(state: tauri::State<'_, run::RunManager>, run_id: String) -> Result<(), String> {
    state.cancel(&run_id)
}

#[tauri::command]
fn write_run_input(
    state: tauri::State<'_, run::RunManager>,
    run_id: String,
    input: String,
) -> Result<(), String> {
    state.write(&run_id, &input)
}

#[tauri::command]
fn resize_run_terminal(
    state: tauri::State<'_, run::RunManager>,
    run_id: String,
    size: run::TerminalSize,
) -> Result<(), String> {
    state.resize(&run_id, size)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(run::RunManager::default())
        .manage(watch::WatchManager::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_project,
            start_project_watch,
            stop_project_watch,
            get_run_settings,
            save_run_script,
            delete_run_script,
            run_script,
            cancel_run,
            write_run_input,
            resize_run_terminal
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            handle.state::<run::RunManager>().terminate_all();
            handle.state::<watch::WatchManager>().stop_all();
        }
    });
}
