use serde::{Deserialize, Deserializer, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AgentKind {
    Amp,
    Codex,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSettings {
    #[serde(default, deserialize_with = "deserialize_preferred_agent")]
    pub(crate) preferred_agent: Option<AgentKind>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentInfo {
    kind: AgentKind,
    label: String,
    available: bool,
    executable: Option<String>,
    version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentConfiguration {
    settings: AgentSettings,
    agents: Vec<AgentInfo>,
}

pub(crate) trait AgentAdapter {
    fn label(&self) -> &str;
    fn executable(&self) -> &Path;
    fn metadata_args(&self) -> Vec<String>;
    fn conflict_args(&self) -> Vec<String>;
}

struct BuiltInAdapter {
    kind: AgentKind,
    path: PathBuf,
}

impl AgentAdapter for BuiltInAdapter {
    fn label(&self) -> &str {
        match self.kind {
            AgentKind::Amp => "Amp",
            AgentKind::Codex => "Codex",
        }
    }

    fn executable(&self) -> &Path {
        &self.path
    }

    fn metadata_args(&self) -> Vec<String> {
        match self.kind {
            AgentKind::Amp => vec!["--no-ide".into(), "--no-notifications".into(), "-x".into()],
            AgentKind::Codex => vec![
                "exec".into(),
                "--ephemeral".into(),
                "-s".into(),
                "read-only".into(),
                "-a".into(),
                "never".into(),
                "-".into(),
            ],
        }
    }

    fn conflict_args(&self) -> Vec<String> {
        match self.kind {
            AgentKind::Amp => vec!["--no-ide".into(), "--no-notifications".into(), "-x".into()],
            AgentKind::Codex => vec![
                "exec".into(),
                "--ephemeral".into(),
                "-s".into(),
                "workspace-write".into(),
                "-a".into(),
                "never".into(),
                "-".into(),
            ],
        }
    }
}

pub(crate) fn configuration(base: &Path) -> Result<AgentConfiguration, String> {
    let settings = read_settings(base)?;
    Ok(AgentConfiguration {
        agents: vec![
            detected(AgentKind::Amp, "Amp", "amp"),
            detected(AgentKind::Codex, "Codex", "codex"),
        ],
        settings,
    })
}

pub(crate) fn save(base: &Path, settings: AgentSettings) -> Result<AgentConfiguration, String> {
    let path = settings_path(base);
    fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
    let content = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, content).map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())?;
    configuration(base)
}

pub(crate) fn selected(base: &Path) -> Result<Box<dyn AgentAdapter + Send>, String> {
    let settings = read_settings(base)?;
    match settings.preferred_agent {
        Some(AgentKind::Amp) => built_in(AgentKind::Amp, "amp"),
        Some(AgentKind::Codex) => built_in(AgentKind::Codex, "codex"),
        None => Err("Choose a coding agent in Shipyard Settings before shipping".to_owned()),
    }
}

fn built_in(
    kind: AgentKind,
    executable_name: &str,
) -> Result<Box<dyn AgentAdapter + Send>, String> {
    let path = executable(executable_name).ok_or_else(|| {
        format!(
            "{} is not installed",
            if kind == AgentKind::Amp {
                "Amp"
            } else {
                "Codex"
            }
        )
    })?;
    Ok(Box::new(BuiltInAdapter { kind, path }))
}

fn detected(kind: AgentKind, label: &str, name: &str) -> AgentInfo {
    let path = executable(name);
    let version = path.as_ref().and_then(|path| {
        Command::new(path)
            .arg("--version")
            .output()
            .ok()
            .and_then(|output| {
                output
                    .status
                    .success()
                    .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            })
    });
    AgentInfo {
        kind,
        label: label.to_owned(),
        available: path.is_some(),
        executable: path.map(|path| path.to_string_lossy().into_owned()),
        version,
    }
}

fn executable(name: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    executable_with_home(name, home.as_deref())
}

fn executable_with_home(name: &str, home: Option<&Path>) -> Option<PathBuf> {
    let startup_script = r#"
if [[ -r "$HOME/.zshrc" ]]; then
  source "$HOME/.zshrc" >/dev/null 2>&1
fi
resolved="$(command -v "$1" 2>/dev/null)"
if [[ -n "$resolved" ]]; then
  printf '__SHIPYARD_EXECUTABLE__%s\n' "$resolved"
  exit 0
fi
exit 1
"#;
    let mut command = Command::new("/bin/zsh");
    command.args(["-lc", startup_script, "shipyard", name]);
    if let Some(home) = home {
        command.env("HOME", home);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let prefix = "__SHIPYARD_EXECUTABLE__";
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(prefix))
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_file())
}

fn deserialize_preferred_agent<'de, D>(deserializer: D) -> Result<Option<AgentKind>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(match value.as_deref() {
        Some("amp") => Some(AgentKind::Amp),
        Some("codex") => Some(AgentKind::Codex),
        _ => None,
    })
}

fn read_settings(base: &Path) -> Result<AgentSettings, String> {
    let path = settings_path(base);
    if !path.exists() {
        return Ok(AgentSettings::default());
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| format!("invalid agent settings: {error}"))
}

fn settings_path(base: &Path) -> PathBuf {
    base.join("agents").join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::{executable_with_home, AgentSettings};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn ignores_previously_configured_custom_agent() {
        let settings: AgentSettings = serde_json::from_str(
            r#"{"preferredAgent":"custom","customName":"Old","customCommand":"/tmp/agent"}"#,
        )
        .unwrap();
        assert!(settings.preferred_agent.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn detects_an_agent_from_zshrc_without_inherited_path() {
        use std::os::unix::fs::PermissionsExt;

        let root = temporary_directory();
        let home = root.join("home");
        let bin = home.join("bin");
        let executable = bin.join("amp");
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            home.join(".zshrc"),
            "export PATH=\"$HOME/bin:/usr/bin:/bin\"\n",
        )
        .unwrap();
        fs::write(&executable, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(executable_with_home("amp", Some(&home)), Some(executable));

        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_directory() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("shipyard-agent-test-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
