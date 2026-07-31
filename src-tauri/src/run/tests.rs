use super::{delete_script, load_settings, manager::spawn_terminal, save_script, ScriptInput};
use portable_pty::PtySize;
use std::{
    fs,
    io::Read,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn saves_updates_and_deletes_script_files() {
    let root = temporary_directory();
    let settings = save_script(
        &root,
        "project-one",
        ScriptInput {
            id: None,
            label: "Development".to_owned(),
            content: "echo first\n".to_owned(),
            make_default: true,
        },
    )
    .unwrap();
    assert_eq!(settings.scripts.len(), 1);
    let id = settings.scripts[0].id.clone();
    assert_eq!(settings.default_script_id.as_deref(), Some(id.as_str()));
    assert_eq!(settings.scripts[0].content, "echo first\n");

    let updated = save_script(
        &root,
        "project-one",
        ScriptInput {
            id: Some(id.clone()),
            label: "Dev server".to_owned(),
            content: "echo second\n".to_owned(),
            make_default: true,
        },
    )
    .unwrap();
    assert_eq!(updated.scripts[0].label, "Dev server");
    assert_eq!(updated.scripts[0].content, "echo second\n");
    assert!(delete_script(&root, "project-one", &id)
        .unwrap()
        .scripts
        .is_empty());
    assert!(load_settings(&root, "project-one")
        .unwrap()
        .scripts
        .is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn terminal_is_interactive_and_accepts_input() {
    let root = temporary_directory();
    let script = root.join("interactive.sh");
    fs::write(
        &script,
        "if [[ -t 0 && -t 1 ]]; then\n  echo tty-ready\nelse\n  exit 2\nfi\nread -t 2 input\necho received:$input\n",
    )
    .unwrap();
    let mut terminal = spawn_terminal(&script, root.to_str().unwrap()).unwrap();
    let mut reader = terminal.reader;
    let output_thread = std::thread::spawn(move || {
        let mut output = String::new();
        reader.read_to_string(&mut output).unwrap();
        output
    });
    terminal
        .session
        .resize(PtySize {
            rows: 40,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    terminal.session.write("hello\r").unwrap();
    let status = terminal.child.wait().unwrap();
    let output = output_thread.join().unwrap();
    assert!(status.success(), "{output}");
    assert!(output.contains("tty-ready"), "{output}");
    assert!(output.contains("received:hello"), "{output}");
    fs::remove_dir_all(root).unwrap();
}

fn temporary_directory() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("shipyard-run-test-{suffix}"));
    fs::create_dir_all(&path).unwrap();
    path
}
