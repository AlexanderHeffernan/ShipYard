mod git;
mod open;
mod run;
mod ship;
mod watch;

use tauri::Manager;

#[tauri::command]
async fn scan_project(app: tauri::AppHandle, path: String) -> Result<git::Project, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let project_id = git::project_id(&path)?;
        let states = ship::active_states(&data_dir, &project_id)?;
        git::scan_project_with_conflicts(&path, &states.conflicts, &states.shipped)
    })
    .await
    .map_err(|error| format!("project scan failed: {error}"))?
}

#[tauri::command]
fn get_ship_settings(
    app: tauri::AppHandle,
    project_id: String,
) -> Result<run::RunSettings, String> {
    ship::load_settings(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        &project_id,
    )
}

#[tauri::command]
fn save_ship_script(
    app: tauri::AppHandle,
    project_id: String,
    script: run::ScriptInput,
) -> Result<run::RunSettings, String> {
    ship::save_script(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        &project_id,
        script,
    )
}

#[tauri::command]
fn delete_ship_script(
    app: tauri::AppHandle,
    project_id: String,
    script_id: String,
) -> Result<run::RunSettings, String> {
    ship::delete_script(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        &project_id,
        &script_id,
    )
}

#[tauri::command]
fn get_open_settings(app: tauri::AppHandle) -> Result<open::OpenSettings, String> {
    open::load_settings(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
    )
}

#[tauri::command]
fn save_open_application(
    app: tauri::AppHandle,
    application: open::OpenApplicationInput,
) -> Result<open::OpenSettings, String> {
    open::save_application(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        application,
    )
}

#[tauri::command]
fn delete_open_application(
    app: tauri::AppHandle,
    application_id: String,
) -> Result<open::OpenSettings, String> {
    open::delete_application(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        &application_id,
    )
}

#[tauri::command]
fn open_checkout(app: tauri::AppHandle, request: open::OpenRequest) -> Result<(), String> {
    open::open_checkout(
        &app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?,
        request,
    )
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
fn ship_script(
    app: tauri::AppHandle,
    state: tauri::State<'_, run::RunManager>,
    request: ship::ShipRequest,
) -> Result<run::RunStarted, String> {
    state.start_ship(app, request)
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
            get_ship_settings,
            save_ship_script,
            delete_ship_script,
            get_open_settings,
            save_open_application,
            delete_open_application,
            open_checkout,
            start_project_watch,
            stop_project_watch,
            get_run_settings,
            save_run_script,
            delete_run_script,
            run_script,
            ship_script,
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
