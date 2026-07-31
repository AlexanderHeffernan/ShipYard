use super::{context::ShipContext, request::ShipRequest};
use crate::git;

pub(crate) fn validate(request: &ShipRequest) -> Result<ShipContext, String> {
    let target_path = git::validate_ship(
        &request.project_id,
        &request.source_path,
        request.source_branch.as_deref(),
        &request.source_sha,
        &request.default_branch,
    )?;
    Ok(ShipContext {
        project_id: request.project_id.clone(),
        work_item_id: request.work_item_id.clone(),
        source_path: request.source_path.clone().into(),
        source_branch: request.source_branch.clone(),
        source_sha: request.source_sha.clone(),
        default_branch: request.default_branch.clone(),
        target_path,
    })
}
