mod agents;
mod git;
mod github;
mod open;
mod run;
mod shipping;
mod watch;

use tauri::Manager;

#[tauri::command]
async fn get_agent_configuration(
    app: tauri::AppHandle,
) -> Result<agents::AgentConfiguration, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || agents::configuration(&base))
        .await
        .map_err(|error| format!("agent detection failed: {error}"))?
}

#[tauri::command]
async fn save_agent_settings(
    app: tauri::AppHandle,
    settings: agents::AgentSettings,
) -> Result<agents::AgentConfiguration, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || agents::save(&base, settings))
        .await
        .map_err(|error| format!("agent settings save failed: {error}"))?
}

#[tauri::command]
async fn scan_project(path: String) -> Result<git::Project, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (root, _) = git::resolve(&path)?;
        let mut project = git::scan_project(&path)?;
        github::enrich_project(&root, &mut project);
        Ok(project)
    })
    .await
    .map_err(|error| format!("project scan failed: {error}"))?
}

#[tauri::command]
async fn checkout_pull_request(
    app: tauri::AppHandle,
    request: git::CheckoutPullRequestRequest,
) -> Result<git::CheckoutPullRequestResult, String> {
    let data_dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || git::checkout_pull_request(&data_dir, request))
        .await
        .map_err(|error| format!("pull request checkout failed: {error}"))?
}

#[tauri::command]
async fn get_work_item_diff(
    request: git::WorkItemDiffRequest,
) -> Result<git::WorkItemDiff, String> {
    tauri::async_runtime::spawn_blocking(move || git::read_work_item_diff(request))
        .await
        .map_err(|error| format!("work item diff failed: {error}"))?
}

#[tauri::command]
async fn inspect_work_item_deletion(
    request: git::DeleteWorkItemRequest,
) -> Result<git::DeletionPlan, String> {
    tauri::async_runtime::spawn_blocking(move || git::inspect_work_item_deletion(request))
        .await
        .map_err(|error| format!("work item inspection failed: {error}"))?
}

#[tauri::command]
async fn delete_work_item(
    request: git::DeleteWorkItemRequest,
    confirmed_plan: git::DeletionPlan,
) -> Result<git::DeletionResult, String> {
    tauri::async_runtime::spawn_blocking(move || git::delete_work_item(request, confirmed_plan))
        .await
        .map_err(|error| format!("work item deletion failed: {error}"))?
}

#[tauri::command]
async fn get_github_status() -> Result<github::GitHubStatus, String> {
    tauri::async_runtime::spawn_blocking(github::status)
        .await
        .map_err(|error| format!("GitHub status check failed: {error}"))
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
fn ship_work(
    app: tauri::AppHandle,
    state: tauri::State<'_, run::RunManager>,
    request: shipping::ShippingRequest,
) -> Result<run::RunStarted, String> {
    state.start_shipping(app, request)
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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_agent_configuration,
            save_agent_settings,
            scan_project,
            checkout_pull_request,
            get_work_item_diff,
            inspect_work_item_deletion,
            delete_work_item,
            get_github_status,
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
            ship_work,
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
