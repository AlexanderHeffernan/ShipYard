use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[cfg(target_os = "macos")]
use std::process::Command;

const SETTINGS_VERSION: u32 = 1;
const OBSERVED_VERSION: u32 = 2;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationSettings {
    pub(crate) new_pull_requests: bool,
    pub(crate) pull_request_updates: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationProject {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) available: bool,
    pub(crate) pull_requests: Vec<NotificationPullRequest>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationPullRequest {
    pub(crate) number: u64,
    pub(crate) head_sha: String,
    pub(crate) draft: bool,
    pub(crate) merge_state: String,
    pub(crate) attention_state: String,
    pub(crate) base_branch: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationEvent {
    pub(crate) kind: NotificationEventKind,
    pub(crate) identity: String,
    pub(crate) project_id: String,
    pub(crate) pull_request_number: u64,
    pub(crate) title: String,
    pub(crate) body: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum NotificationEventKind {
    NewPullRequest,
    PullRequestUpdated,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredObservationState {
    version: u32,
    projects: BTreeMap<String, StoredProjectObservation>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredProjectObservation {
    bootstrapped: bool,
    pull_requests: BTreeMap<String, StoredPullRequestObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredPullRequestObservation {
    identity: String,
    revision: String,
    #[serde(default)]
    attention_state: String,
    event_state: String,
    present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UpdateReason {
    NewCommits,
    NeedsAttention,
    StatusChanged,
}

impl UpdateReason {
    fn event_state(self) -> &'static str {
        match self {
            Self::NewCommits => "newCommits",
            Self::NeedsAttention => "needsAttention",
            Self::StatusChanged => "statusChanged",
        }
    }
}

pub(crate) fn load_settings(base: &Path) -> Result<NotificationSettings, String> {
    let path = settings_path(base);
    if !path.exists() {
        return Ok(NotificationSettings::default());
    }

    let content = fs::read_to_string(&path).map_err(file_error)?;
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(_) => {
            let settings = NotificationSettings::default();
            write_settings(base, &settings)?;
            return Ok(settings);
        }
    };
    let (settings, migrated) = migrate_settings(&value)?;
    if migrated {
        write_settings(base, &settings)?;
    }
    Ok(settings)
}

pub(crate) fn save_settings(
    base: &Path,
    settings: NotificationSettings,
) -> Result<NotificationSettings, String> {
    write_settings(base, &settings)?;
    Ok(settings)
}

pub(crate) fn observe(
    base: &Path,
    projects: Vec<NotificationProject>,
) -> Result<Vec<NotificationEvent>, String> {
    if projects.is_empty() {
        return Ok(Vec::new());
    }

    let mut state = read_observations(base)?;
    let mut events = Vec::new();

    for project in projects {
        if !project.available {
            continue;
        }
        let project_state = state.projects.entry(project.id.clone()).or_default();
        let was_bootstrapped = project_state.bootstrapped;
        let mut seen_numbers = HashSet::new();
        for pull_request in &project.pull_requests {
            let key = pull_request.number.to_string();
            if !seen_numbers.insert(key.clone()) {
                continue;
            }

            let identity = pull_request_identity(&project.id, pull_request.number);
            let revision = material_revision(pull_request);
            let previous = project_state.pull_requests.get(&key).cloned();
            let event = if was_bootstrapped
                && previous
                    .as_ref()
                    .map(|previous| !previous.present)
                    .unwrap_or(true)
            {
                Some(new_pull_request_event(
                    &project,
                    pull_request.number,
                    &identity,
                ))
            } else {
                None
            };
            let event = event.or_else(|| {
                if !was_bootstrapped {
                    return None;
                }
                let previous = previous.as_ref()?;
                let reason = material_update(previous, pull_request)?;
                Some(updated_pull_request_event(
                    &project,
                    pull_request.number,
                    &identity,
                    reason,
                ))
            });

            let event_state = if event
                .as_ref()
                .is_some_and(|event| event.kind == NotificationEventKind::NewPullRequest)
            {
                "new"
            } else if let Some(reason) = previous
                .as_ref()
                .and_then(|previous| material_update(previous, pull_request))
            {
                reason.event_state()
            } else if was_bootstrapped {
                "unchanged"
            } else {
                "baseline"
            };

            project_state.pull_requests.insert(
                key,
                StoredPullRequestObservation {
                    identity,
                    revision,
                    attention_state: pull_request.attention_state.clone(),
                    event_state: event_state.to_owned(),
                    present: true,
                },
            );
            if let Some(event) = event {
                events.push(event);
            }
        }
        for (number, observed) in &mut project_state.pull_requests {
            observed.present = seen_numbers.contains(number);
        }
        project_state.bootstrapped = true;
    }

    state.version = OBSERVED_VERSION;
    write_observations(base, &state)?;
    Ok(events)
}

pub(crate) fn open_system_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
            .status()
            .map_err(|error| format!("could not open Notification settings: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("macOS could not open Notification settings".to_owned())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Notification settings are available only on macOS".to_owned())
    }
}

fn migrate_settings(value: &serde_json::Value) -> Result<(NotificationSettings, bool), String> {
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as u32;
    if version > SETTINGS_VERSION {
        return Err(format!(
            "notification settings version {version} is newer than Shipyard supports"
        ));
    }

    let new_pull_requests =
        bool_field(value, "newPullRequests").or_else(|| bool_field(value, "new_pull_requests"));
    let pull_request_updates = bool_field(value, "pullRequestUpdates")
        .or_else(|| bool_field(value, "pull_request_updates"));
    let legacy_enabled =
        bool_field(value, "enabled").or_else(|| bool_field(value, "notificationsEnabled"));
    let migrated = version != SETTINGS_VERSION;

    Ok((
        NotificationSettings {
            new_pull_requests: new_pull_requests.or(legacy_enabled).unwrap_or(false),
            pull_request_updates: pull_request_updates.or(legacy_enabled).unwrap_or(false),
        },
        migrated,
    ))
}

fn bool_field(value: &serde_json::Value, name: &str) -> Option<bool> {
    value.get(name).and_then(serde_json::Value::as_bool)
}

fn material_update(
    previous: &StoredPullRequestObservation,
    current: &NotificationPullRequest,
) -> Option<UpdateReason> {
    if previous.revision == material_revision(current) {
        return None;
    }

    if previous.revision.split('|').next() != Some(current.head_sha.as_str()) {
        return Some(UpdateReason::NewCommits);
    }
    if previous.attention_state != current.attention_state
        && requires_attention(&current.merge_state, &current.attention_state)
    {
        return Some(UpdateReason::NeedsAttention);
    }
    Some(UpdateReason::StatusChanged)
}

fn requires_attention(merge_state: &str, attention_state: &str) -> bool {
    matches!(
        merge_state,
        "checksFailed" | "reviewRequired" | "conflicting"
    ) || attention_state.split('|').any(|state| {
        matches!(
            state,
            "review=changesRequested" | "review=reviewRequired" | "checks=failed"
        )
    })
}

fn material_revision(pull_request: &NotificationPullRequest) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        pull_request.head_sha,
        pull_request.draft,
        pull_request.merge_state,
        pull_request.attention_state,
        pull_request.base_branch
    )
}

fn pull_request_identity(project_id: &str, number: u64) -> String {
    format!("{project_id}::pull-request::{number}")
}

fn new_pull_request_event(
    project: &NotificationProject,
    number: u64,
    identity: &str,
) -> NotificationEvent {
    let project_name = safe_project_name(&project.name);
    NotificationEvent {
        kind: NotificationEventKind::NewPullRequest,
        identity: identity.to_owned(),
        project_id: project.id.clone(),
        pull_request_number: number,
        title: format!("New pull request · {project_name}"),
        body: format!("Pull request #{number} is now open."),
    }
}

fn updated_pull_request_event(
    project: &NotificationProject,
    number: u64,
    identity: &str,
    reason: UpdateReason,
) -> NotificationEvent {
    let project_name = safe_project_name(&project.name);
    let body = match reason {
        UpdateReason::NewCommits => format!("Pull request #{number} has new commits."),
        UpdateReason::NeedsAttention => format!("Pull request #{number} needs attention."),
        UpdateReason::StatusChanged => format!("Pull request #{number} changed."),
    };
    NotificationEvent {
        kind: NotificationEventKind::PullRequestUpdated,
        identity: identity.to_owned(),
        project_id: project.id.clone(),
        pull_request_number: number,
        title: format!("Pull request update · {project_name}"),
        body,
    }
}

fn safe_project_name(name: &str) -> String {
    let normalized = name
        .chars()
        .map(|character| {
            if character.is_control() || character == '\n' || character == '\r' {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut name = normalized.chars().take(80).collect::<String>();
    if name.is_empty() {
        name = "Project".to_owned();
    }
    name
}

fn read_observations(base: &Path) -> Result<StoredObservationState, String> {
    let path = observations_path(base);
    if !path.exists() {
        return Ok(StoredObservationState {
            version: OBSERVED_VERSION,
            ..StoredObservationState::default()
        });
    }
    let content = fs::read_to_string(path).map_err(file_error)?;
    let mut state: StoredObservationState =
        serde_json::from_str(&content).unwrap_or_else(|_| StoredObservationState {
            version: OBSERVED_VERSION,
            ..StoredObservationState::default()
        });
    if state.version != OBSERVED_VERSION {
        state = StoredObservationState {
            version: OBSERVED_VERSION,
            ..StoredObservationState::default()
        };
    }
    Ok(state)
}

fn write_settings(base: &Path, settings: &NotificationSettings) -> Result<(), String> {
    let path = settings_path(base);
    let value = serde_json::json!({
        "version": SETTINGS_VERSION,
        "newPullRequests": settings.new_pull_requests,
        "pullRequestUpdates": settings.pull_request_updates,
    });
    write_json(&path, &value)
}

fn write_observations(base: &Path, state: &StoredObservationState) -> Result<(), String> {
    write_json(&observations_path(base), state)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new("."))).map_err(file_error)?;
    let content = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, content).map_err(file_error)?;
    fs::rename(temporary, path).map_err(file_error)
}

fn settings_path(base: &Path) -> PathBuf {
    base.join("notifications").join("settings.json")
}

fn observations_path(base: &Path) -> PathBuf {
    base.join("notifications").join("observed.json")
}

fn file_error(error: std::io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        load_settings, material_revision, observe, pull_request_identity, save_settings,
        NotificationEventKind, NotificationProject, NotificationPullRequest, NotificationSettings,
    };
    use serde_json::json;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn defaults_keep_every_notification_type_off() {
        assert_eq!(
            NotificationSettings::default(),
            NotificationSettings {
                new_pull_requests: false,
                pull_request_updates: false,
            }
        );
        let root = temporary_directory();
        assert_eq!(
            load_settings(&root).unwrap(),
            NotificationSettings::default()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_persistence_resets_to_a_quiet_safe_baseline() {
        let root = temporary_directory();
        fs::create_dir_all(root.join("notifications")).unwrap();
        fs::write(root.join("notifications/settings.json"), b"not-json").unwrap();
        fs::write(root.join("notifications/observed.json"), b"not-json").unwrap();

        assert_eq!(
            load_settings(&root).unwrap(),
            NotificationSettings::default()
        );
        assert!(observe(&root, vec![project("harbor", 7, "sha-1", "ready")])
            .unwrap()
            .is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migrates_a_legacy_global_notification_choice_without_losing_opt_in() {
        let root = temporary_directory();
        fs::create_dir_all(root.join("notifications")).unwrap();
        fs::write(
            root.join("notifications/settings.json"),
            serde_json::to_vec(&json!({ "enabled": true })).unwrap(),
        )
        .unwrap();

        let settings = load_settings(&root).unwrap();
        assert!(settings.new_pull_requests);
        assert!(settings.pull_request_updates);
        let migrated: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("notifications/settings.json")).unwrap())
                .unwrap();
        assert_eq!(migrated["version"], 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn upgrades_observation_schema_with_a_quiet_baseline() {
        let root = temporary_directory();
        fs::create_dir_all(root.join("notifications")).unwrap();
        fs::write(
            root.join("notifications/observed.json"),
            serde_json::to_vec(&json!({
                "version": 1,
                "projects": {
                    "/harbor/.git": {
                        "bootstrapped": true,
                        "pullRequests": {
                            "7": {
                                "identity": "/harbor/.git::pull-request::7",
                                "revision": "sha-1|false|ready|main",
                                "eventState": "unchanged",
                                "present": true
                            }
                        }
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();

        assert!(observe(&root, vec![project("harbor", 7, "sha-1", "ready")])
            .unwrap()
            .is_empty());
        let upgraded: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("notifications/observed.json")).unwrap())
                .unwrap();
        assert_eq!(upgraded["version"], 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bootstraps_existing_prs_then_detects_new_and_material_updates() {
        let root = temporary_directory();
        let baseline = project("harbor", 7, "sha-1", "ready");
        assert!(observe(&root, vec![baseline.clone()]).unwrap().is_empty());

        let mut new_pr = project("harbor", 8, "sha-8", "ready");
        new_pr.pull_requests.push(baseline.pull_requests[0].clone());
        let events = observe(&root, vec![new_pr]).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, NotificationEventKind::NewPullRequest);
        assert_eq!(events[0].title, "New pull request · harbor");
        assert_eq!(events[0].body, "Pull request #8 is now open.");

        let mut updated = project("harbor", 7, "sha-2", "ready");
        updated
            .pull_requests
            .push(project("harbor", 8, "sha-8", "ready").pull_requests[0].clone());
        let events = observe(&root, vec![updated]).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, NotificationEventKind::PullRequestUpdated);
        assert!(events[0].body.contains("new commits"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repeated_polls_of_the_same_revision_are_deduplicated_and_persisted() {
        let root = temporary_directory();
        let baseline = project("harbor", 7, "sha-1", "ready");
        observe(&root, vec![baseline.clone()]).unwrap();
        let updated = project("harbor", 7, "sha-2", "checksFailed");
        assert_eq!(observe(&root, vec![updated.clone()]).unwrap().len(), 1);
        assert!(observe(&root, vec![updated]).unwrap().is_empty());
        assert!(root.join("notifications/observed.json").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_pr_that_disappears_and_reappears_is_a_new_appearance() {
        let root = temporary_directory();
        let current = project("harbor", 7, "sha-1", "ready");
        observe(&root, vec![current.clone()]).unwrap();
        observe(&root, vec![project_without_prs("harbor")]).unwrap();
        let events = observe(&root, vec![current]).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, NotificationEventKind::NewPullRequest);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unavailable_project_scans_do_not_create_false_disappearances() {
        let root = temporary_directory();
        let current = project("harbor", 7, "sha-1", "ready");
        observe(&root, vec![current.clone()]).unwrap();
        observe(&root, vec![unavailable_project("harbor")]).unwrap();
        assert!(observe(&root, vec![current]).unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn payloads_identify_project_and_pr_without_user_content() {
        let root = temporary_directory();
        let project = project("folder", 42, "sha-1", "ready");
        observe(&root, vec![project.clone()]).unwrap();
        let events = observe(
            &root,
            vec![NotificationProject {
                name: "folder\n".to_owned(),
                ..project.clone()
            }],
        )
        .unwrap();
        assert!(events.is_empty());

        let events = observe(
            &root,
            vec![NotificationProject {
                name: "folder\n".to_owned(),
                pull_requests: vec![NotificationPullRequest {
                    number: 42,
                    head_sha: "sha-2".to_owned(),
                    draft: false,
                    merge_state: "reviewRequired".to_owned(),
                    attention_state: "review=reviewRequired|checks=passed".to_owned(),
                    base_branch: "main".to_owned(),
                }],
                ..project_without_prs("folder")
            }],
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Pull request update · folder");
        assert!(events[0].body.contains("#42"));
        assert!(!events[0].body.contains("sha-2"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn setting_round_trip_preserves_independent_rules() {
        let root = temporary_directory();
        let settings = NotificationSettings {
            new_pull_requests: true,
            pull_request_updates: false,
        };
        save_settings(&root, settings.clone()).unwrap();
        assert_eq!(load_settings(&root).unwrap(), settings);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn attention_state_changes_are_material_even_when_merge_state_is_unchanged() {
        let root = temporary_directory();
        let baseline =
            project_with_attention("harbor", 7, "sha-1", "ready", "review=none|checks=passed");
        observe(&root, vec![baseline]).unwrap();
        let updated = project_with_attention(
            "harbor",
            7,
            "sha-1",
            "ready",
            "review=changesRequested|checks=passed",
        );
        let events = observe(&root, vec![updated]).unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].body.contains("needs attention"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn multiple_projects_keep_identity_and_deduplication_independent() {
        let root = temporary_directory();
        observe(
            &root,
            vec![
                project("harbor", 7, "sha-harbor", "ready"),
                project("dock", 3, "sha-dock", "ready"),
            ],
        )
        .unwrap();

        let events = observe(
            &root,
            vec![
                project("harbor", 7, "sha-harbor-2", "ready"),
                project("dock", 3, "sha-dock", "ready"),
            ],
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].project_id, "/harbor/.git");
        assert!(observe(
            &root,
            vec![
                project("harbor", 7, "sha-harbor-2", "ready"),
                project("dock", 3, "sha-dock", "ready"),
            ],
        )
        .unwrap()
        .is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn revision_contains_only_material_pr_fields() {
        let pull_request = NotificationPullRequest {
            number: 1,
            head_sha: "sha".to_owned(),
            draft: false,
            merge_state: "ready".to_owned(),
            attention_state: "review=none|checks=passed".to_owned(),
            base_branch: "main".to_owned(),
        };
        assert_eq!(
            material_revision(&pull_request),
            "sha|false|ready|review=none|checks=passed|main"
        );
        assert_eq!(
            pull_request_identity("/repo/.git", 1),
            "/repo/.git::pull-request::1"
        );
    }

    fn project(name: &str, number: u64, head_sha: &str, merge_state: &str) -> NotificationProject {
        project_with_attention(
            name,
            number,
            head_sha,
            merge_state,
            "review=none|checks=passed",
        )
    }

    fn project_with_attention(
        name: &str,
        number: u64,
        head_sha: &str,
        merge_state: &str,
        attention_state: &str,
    ) -> NotificationProject {
        NotificationProject {
            id: format!("/{name}/.git"),
            name: name.to_owned(),
            available: true,
            pull_requests: vec![NotificationPullRequest {
                number,
                head_sha: head_sha.to_owned(),
                draft: false,
                merge_state: merge_state.to_owned(),
                attention_state: attention_state.to_owned(),
                base_branch: "main".to_owned(),
            }],
        }
    }

    fn project_without_prs(name: &str) -> NotificationProject {
        NotificationProject {
            id: format!("/{name}/.git"),
            name: name.to_owned(),
            available: true,
            pull_requests: Vec::new(),
        }
    }

    fn unavailable_project(name: &str) -> NotificationProject {
        NotificationProject {
            id: format!("/{name}/.git"),
            name: name.to_owned(),
            available: false,
            pull_requests: Vec::new(),
        }
    }

    fn temporary_directory() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("shipyard-notification-test-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
