use super::{
    run_script::RunScript, run_settings::RunSettings, script_input::ScriptInput,
    stored_run_script::StoredRunScript, stored_run_settings::StoredRunSettings,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn load_settings(base: &Path, project_id: &str) -> Result<RunSettings, String> {
    load_scoped_settings(base, project_id, "run")
}

pub(crate) fn load_scoped_settings(
    base: &Path,
    project_id: &str,
    scope: &str,
) -> Result<RunSettings, String> {
    let directory = scoped_directory(base, project_id, scope);
    let stored = read_stored_settings(&directory)?;
    let scripts = stored
        .scripts
        .iter()
        .map(|script| hydrate_script(&directory, script))
        .collect::<Result<_, _>>()?;
    Ok(RunSettings {
        default_script_id: stored.default_script_id,
        scripts,
    })
}

pub fn save_script(
    base: &Path,
    project_id: &str,
    input: ScriptInput,
) -> Result<RunSettings, String> {
    save_scoped_script(base, project_id, "run", input)
}

pub(crate) fn save_scoped_script(
    base: &Path,
    project_id: &str,
    scope: &str,
    input: ScriptInput,
) -> Result<RunSettings, String> {
    validate_input(&input)?;
    let directory = scoped_directory(base, project_id, scope);
    fs::create_dir_all(directory.join("scripts")).map_err(file_error)?;
    let mut settings = read_stored_settings(&directory)?;
    let id = resolve_script_id(&settings, input.id)?;
    let file_name = format!("{id}.sh");
    write_script(&directory.join("scripts").join(&file_name), &input.content)?;
    upsert_metadata(&mut settings, &id, &input.label, &file_name);
    if input.make_default || settings.default_script_id.is_none() {
        settings.default_script_id = Some(id);
    }
    write_settings(&directory, &settings)?;
    load_scoped_settings(base, project_id, scope)
}

pub fn delete_script(
    base: &Path,
    project_id: &str,
    script_id: &str,
) -> Result<RunSettings, String> {
    delete_scoped_script(base, project_id, "run", script_id)
}

pub(crate) fn delete_scoped_script(
    base: &Path,
    project_id: &str,
    scope: &str,
    script_id: &str,
) -> Result<RunSettings, String> {
    let directory = scoped_directory(base, project_id, scope);
    let mut settings = read_stored_settings(&directory)?;
    let script = settings
        .scripts
        .iter()
        .find(|script| script.id == script_id)
        .ok_or_else(|| "run script not found".to_owned())?;
    validate_metadata(script)?;
    let _ = fs::remove_file(directory.join("scripts").join(&script.file_name));
    settings.scripts.retain(|script| script.id != script_id);
    if settings.default_script_id.as_deref() == Some(script_id) {
        settings.default_script_id = settings.scripts.first().map(|script| script.id.clone());
    }
    write_settings(&directory, &settings)?;
    load_scoped_settings(base, project_id, scope)
}

pub(super) fn script_path(
    base: &Path,
    project_id: &str,
    script_id: &str,
) -> Result<PathBuf, String> {
    scoped_script_path(base, project_id, "run", script_id)
}

pub(crate) fn scoped_script_path(
    base: &Path,
    project_id: &str,
    scope: &str,
    script_id: &str,
) -> Result<PathBuf, String> {
    let directory = scoped_directory(base, project_id, scope);
    let settings = read_stored_settings(&directory)?;
    let script = settings
        .scripts
        .iter()
        .find(|script| script.id == script_id)
        .ok_or_else(|| "run script not found".to_owned())?;
    validate_metadata(script)?;
    Ok(directory.join("scripts").join(&script.file_name))
}

fn validate_input(input: &ScriptInput) -> Result<(), String> {
    if input.label.trim().is_empty() {
        return Err("script label is required".to_owned());
    }
    if input.content.trim().is_empty() {
        return Err("script content is required".to_owned());
    }
    Ok(())
}

fn resolve_script_id(
    settings: &StoredRunSettings,
    requested_id: Option<String>,
) -> Result<String, String> {
    let Some(id) = requested_id else {
        return Ok(new_script_id());
    };
    settings
        .scripts
        .iter()
        .any(|script| script.id == id)
        .then_some(id)
        .ok_or_else(|| "run script not found".to_owned())
}

fn upsert_metadata(settings: &mut StoredRunSettings, id: &str, label: &str, file_name: &str) {
    if let Some(script) = settings.scripts.iter_mut().find(|script| script.id == id) {
        script.label = label.trim().to_owned();
        return;
    }
    settings.scripts.push(StoredRunScript {
        id: id.to_owned(),
        label: label.trim().to_owned(),
        file_name: file_name.to_owned(),
    });
}

fn hydrate_script(directory: &Path, script: &StoredRunScript) -> Result<RunScript, String> {
    validate_metadata(script)?;
    let path = directory.join("scripts").join(&script.file_name);
    let content = fs::read_to_string(&path).map_err(file_error)?;
    Ok(RunScript {
        id: script.id.clone(),
        label: script.label.clone(),
        file_name: script.file_name.clone(),
        file_path: path.to_string_lossy().into_owned(),
        content,
    })
}

fn validate_metadata(script: &StoredRunScript) -> Result<(), String> {
    if script.file_name != format!("{}.sh", script.id)
        || !script
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("invalid run script metadata".to_owned());
    }
    Ok(())
}

fn read_stored_settings(directory: &Path) -> Result<StoredRunSettings, String> {
    let path = directory.join("settings.json");
    if !path.exists() {
        return Ok(StoredRunSettings::default());
    }
    let content = fs::read_to_string(path).map_err(file_error)?;
    serde_json::from_str(&content).map_err(|error| format!("invalid run settings: {error}"))
}

fn write_settings(directory: &Path, settings: &StoredRunSettings) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(file_error)?;
    let content = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    atomic_write(&directory.join("settings.json"), &content)
}

fn write_script(path: &Path, content: &str) -> Result<(), String> {
    atomic_write(path, content.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(file_error)?;
    }
    Ok(())
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, content).map_err(file_error)?;
    fs::rename(temporary, path).map_err(file_error)
}

fn project_directory(base: &Path, project_id: &str) -> PathBuf {
    let hash = Sha256::digest(project_id.as_bytes());
    base.join("projects").join(format!("{hash:x}"))
}

fn scoped_directory(base: &Path, project_id: &str, scope: &str) -> PathBuf {
    let directory = project_directory(base, project_id);
    if scope == "run" {
        directory
    } else {
        directory.join(scope)
    }
}

fn new_script_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("script-{timestamp}")
}

fn file_error(error: std::io::Error) -> String {
    error.to_string()
}
