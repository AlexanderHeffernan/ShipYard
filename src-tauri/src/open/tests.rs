use super::{
    application_kind::ApplicationKind, delete_application, launcher::build_command, load_settings,
    open_application_input::OpenApplicationInput, save_application,
    store::validate_application_path,
};
use std::{fs, path::PathBuf, time::SystemTime};

#[test]
fn persists_applications_and_default_outside_projects() {
    let root = temporary_directory();
    let app = root.join("Applications").join("Example Editor.app");
    fs::create_dir_all(&app).unwrap();
    let settings = save_application(
        &root,
        OpenApplicationInput {
            id: None,
            label: "Example".to_owned(),
            kind: ApplicationKind::Editor,
            app_path: app.to_string_lossy().into_owned(),
            make_default: true,
        },
    )
    .unwrap();
    let id = settings.default_application_id.unwrap();
    assert_eq!(load_settings(&root).unwrap().applications.len(), 1);
    assert!(root.join("open/settings.json").is_file());
    assert!(delete_application(&root, &id)
        .unwrap()
        .applications
        .is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_missing_files_and_non_app_directories() {
    let root = temporary_directory();
    fs::create_dir_all(root.join("Not An App")).unwrap();
    assert!(validate_application_path(&root.join("Missing.app")).is_err());
    assert!(validate_application_path(&root.join("Not An App")).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn constructs_open_command_with_uninterpolated_arguments() {
    use std::ffi::OsStr;
    let command = build_command(
        PathBuf::from("/Applications/Visual Studio Code.app").as_path(),
        PathBuf::from("/tmp/a checkout; echo unsafe").as_path(),
    )
    .unwrap();
    assert_eq!(command.get_program(), OsStr::new("/usr/bin/open"));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        vec![
            OsStr::new("-a"),
            OsStr::new("/Applications/Visual Studio Code.app"),
            OsStr::new("/tmp/a checkout; echo unsafe")
        ]
    );
}

fn temporary_directory() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("shipyard-open-test-{suffix}"))
}
