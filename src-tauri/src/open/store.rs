use super::{
    open_application::OpenApplication, open_application_input::OpenApplicationInput,
    open_settings::OpenSettings, stored_open_application::StoredOpenApplication,
    stored_open_settings::StoredOpenSettings,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn load_settings(base: &Path) -> Result<OpenSettings, String> {
    let stored = read_settings(base)?;
    let applications = stored
        .applications
        .into_iter()
        .map(|application| OpenApplication {
            available: validate_application_path(Path::new(&application.app_path)).is_ok(),
            id: application.id,
            label: application.label,
            kind: application.kind,
            app_path: application.app_path,
        })
        .collect();
    Ok(OpenSettings {
        default_application_id: stored.default_application_id,
        applications,
    })
}

pub fn save_application(base: &Path, input: OpenApplicationInput) -> Result<OpenSettings, String> {
    validate_label(&input.label)?;
    let app_path = validate_application_path(Path::new(&input.app_path))?;
    let mut settings = read_settings(base)?;
    let id = resolve_id(&settings, input.id)?;
    let application = StoredOpenApplication {
        id: id.clone(),
        label: input.label.trim().to_owned(),
        kind: input.kind,
        app_path: app_path.to_string_lossy().into_owned(),
    };
    upsert(&mut settings, application);
    if input.make_default || settings.default_application_id.is_none() {
        settings.default_application_id = Some(id);
    }
    write_settings(base, &settings)?;
    load_settings(base)
}

pub fn delete_application(base: &Path, id: &str) -> Result<OpenSettings, String> {
    let mut settings = read_settings(base)?;
    let original_length = settings.applications.len();
    settings
        .applications
        .retain(|application| application.id != id);
    if settings.applications.len() == original_length {
        return Err("application not found".to_owned());
    }
    if settings.default_application_id.as_deref() == Some(id) {
        settings.default_application_id = settings.applications.first().map(|app| app.id.clone());
    }
    write_settings(base, &settings)?;
    load_settings(base)
}

pub(super) fn stored_application(base: &Path, id: &str) -> Result<StoredOpenApplication, String> {
    read_settings(base)?
        .applications
        .into_iter()
        .find(|application| application.id == id)
        .ok_or_else(|| "application not found; configure it in Settings → Open".to_owned())
}

pub(super) fn validate_application_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_dir() || path.extension().and_then(|value| value.to_str()) != Some("app") {
        return Err(format!(
            "{} is not an available macOS .app bundle",
            path.display()
        ));
    }
    path.canonicalize()
        .map_err(|error| format!("could not resolve application {}: {error}", path.display()))
}

fn validate_label(label: &str) -> Result<(), String> {
    if label.trim().is_empty() {
        Err("application label is required".to_owned())
    } else {
        Ok(())
    }
}

fn resolve_id(settings: &StoredOpenSettings, id: Option<String>) -> Result<String, String> {
    let Some(id) = id else { return Ok(new_id()) };
    settings
        .applications
        .iter()
        .any(|application| application.id == id)
        .then_some(id)
        .ok_or_else(|| "application not found".to_owned())
}

fn upsert(settings: &mut StoredOpenSettings, application: StoredOpenApplication) {
    if let Some(existing) = settings
        .applications
        .iter_mut()
        .find(|existing| existing.id == application.id)
    {
        *existing = application;
    } else {
        settings.applications.push(application);
    }
}

fn read_settings(base: &Path) -> Result<StoredOpenSettings, String> {
    let path = settings_path(base);
    if !path.exists() {
        return Ok(StoredOpenSettings::default());
    }
    let content = fs::read_to_string(&path).map_err(file_error)?;
    serde_json::from_str(&content).map_err(|error| format!("invalid Open settings: {error}"))
}

fn write_settings(base: &Path, settings: &StoredOpenSettings) -> Result<(), String> {
    let path = settings_path(base);
    fs::create_dir_all(path.parent().unwrap_or(base)).map_err(file_error)?;
    let content = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, content).map_err(file_error)?;
    fs::rename(temporary, path).map_err(file_error)
}

fn settings_path(base: &Path) -> PathBuf {
    base.join("open").join("settings.json")
}

fn new_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("application-{timestamp}")
}

fn file_error(error: std::io::Error) -> String {
    error.to_string()
}
