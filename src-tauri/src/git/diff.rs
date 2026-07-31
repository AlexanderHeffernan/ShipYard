use super::{command, diff_stats::DiffStats};
use std::path::Path;

pub(super) fn read_stats(worktree: &Path) -> Result<DiffStats, String> {
    let status = command::output(
        worktree,
        &["status", "--porcelain=v1", "-z", "--untracked-files=normal"],
    )?;
    let changed_files = count_status_entries(&status.stdout);
    let mut stats = DiffStats {
        dirty: changed_files > 0,
        changed_files,
        ..DiffStats::default()
    };
    let diff = read_numstat(worktree)?;
    add_numstat(&mut stats, &diff.stdout);
    Ok(stats)
}

fn read_numstat(worktree: &Path) -> Result<std::process::Output, String> {
    let diff = command::output_allow_failure(worktree, &["diff", "HEAD", "--numstat", "--"]);
    if diff.status.success() {
        Ok(diff)
    } else {
        command::output(worktree, &["diff", "--cached", "--numstat", "--"])
    }
}

fn add_numstat(stats: &mut DiffStats, output: &[u8]) {
    for line in command::bytes_text(output).lines() {
        let mut fields = line.splitn(3, '\t');
        stats.additions += parse_count(fields.next());
        stats.deletions += parse_count(fields.next());
    }
}

fn parse_count(value: Option<&str>) -> u64 {
    value.and_then(|value| value.parse().ok()).unwrap_or(0)
}

fn count_status_entries(output: &[u8]) -> usize {
    let fields = output.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut count = 0;
    let mut index = 0;

    while index < fields.len() && !fields[index].is_empty() {
        let status = fields[index].get(..2);
        count += 1;
        index += 1;
        if status.is_some_and(|value| value.contains(&b'R') || value.contains(&b'C')) {
            index += 1;
        }
    }

    count
}
