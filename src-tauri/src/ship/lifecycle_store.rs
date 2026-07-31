use super::{context::ShipContext, ship_record::ShipRecord, ship_states::ShipStates};
use crate::git;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn record_conflict(base: &Path, context: &ShipContext) -> Result<(), String> {
    let mut records = read_records(base, &context.project_id)?;
    records.retain(|record| record.work_item_id != context.work_item_id);
    records.push(ShipRecord {
        work_item_id: context.work_item_id.clone(),
        source_sha: context.source_sha.clone(),
        default_branch: context.default_branch.clone(),
        source_path: context.source_path.to_string_lossy().into_owned(),
        target_path: context.target_path.to_string_lossy().into_owned(),
        successful: false,
    });
    write_records(base, &context.project_id, &records)
}

pub(crate) fn record_success(base: &Path, context: &ShipContext) -> Result<(), String> {
    let mut records = read_records(base, &context.project_id)?;
    records.retain(|record| record.work_item_id != context.work_item_id);
    records.push(ShipRecord {
        work_item_id: context.work_item_id.clone(),
        source_sha: context.source_sha.clone(),
        default_branch: context.default_branch.clone(),
        source_path: context.source_path.to_string_lossy().into_owned(),
        target_path: context.target_path.to_string_lossy().into_owned(),
        successful: true,
    });
    write_records(base, &context.project_id, &records)
}

pub(crate) fn active_states(base: &Path, project_id: &str) -> Result<ShipStates, String> {
    let records = read_records(base, project_id)?;
    let (active, resolved): (Vec<_>, Vec<_>) = records.into_iter().partition(|record| {
        record.successful
            || !git::is_merged(
                &record.target_path,
                &record.source_sha,
                &record.default_branch,
            )
            .unwrap_or(false)
    });
    if !resolved.is_empty() {
        write_records(base, project_id, &active)?;
    }
    let (shipped, conflicts): (Vec<_>, Vec<_>) =
        active.into_iter().partition(|record| record.successful);
    Ok(ShipStates {
        conflicts: conflicts
            .into_iter()
            .map(|record| (record.work_item_id, record.target_path))
            .collect(),
        shipped: shipped
            .into_iter()
            .map(|record| (record.work_item_id, record.source_sha))
            .collect(),
    })
}

fn records_path(base: &Path, project_id: &str) -> PathBuf {
    let hash = Sha256::digest(project_id.as_bytes());
    base.join("projects")
        .join(format!("{hash:x}"))
        .join("ship-state.json")
}

fn read_records(base: &Path, project_id: &str) -> Result<Vec<ShipRecord>, String> {
    let path = records_path(base, project_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| format!("invalid Ship lifecycle state: {error}"))
}

fn write_records(base: &Path, project_id: &str, records: &[ShipRecord]) -> Result<(), String> {
    let path = records_path(base, project_id);
    if records.is_empty() {
        let _ = fs::remove_file(path);
        return Ok(());
    }
    fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
    let content = serde_json::to_vec_pretty(records).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}
