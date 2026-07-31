use super::{branch::Branch, command};
use std::path::Path;

pub(super) fn read(root: &Path) -> Result<Vec<Branch>, String> {
    let output = command::output(
        root,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(objectname)%00%(subject)%00%(committerdate:unix)%00",
            "refs/heads",
        ],
    )?;
    let fields = output.stdout.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut branches = Vec::new();

    for fields in fields.chunks_exact(4) {
        let reference = command::bytes_text(fields[0])
            .trim_start_matches('\n')
            .to_owned();
        if reference.is_empty() {
            break;
        }
        branches.push(branch_from_fields(reference, fields));
    }

    Ok(branches)
}

fn branch_from_fields(reference: String, fields: &[&[u8]]) -> Branch {
    Branch {
        name: reference
            .strip_prefix("refs/heads/")
            .unwrap_or(&reference)
            .to_owned(),
        reference,
        sha: command::bytes_text(fields[1]).to_owned(),
        subject: command::bytes_text(fields[2]).to_owned(),
        updated_at: command::bytes_text(fields[3]).parse().unwrap_or_default(),
    }
}
