import type { Project, WorkItem } from '../types/projects';

export type PullRequestSyncState = 'synced' | 'localChanges' | 'localAhead' | 'remoteAhead' | 'diverged';

export function workItemTitle(item: WorkItem) {
  if (item.pullRequest && !item.branch) return item.pullRequest.title;
  if (item.branch) return item.branch;
  if (item.worktreePath) return fileName(item.worktreePath) || 'Detached worktree';
  return `Detached at ${item.headSha.slice(0, 7)}`;
}

export function workItemKind(project: Project, item: WorkItem) {
  if (item.pullRequest && !item.worktreePath) return 'Remote pull request';
  if (!item.branch) return 'Detached worktree';
  if (item.worktreePath && item.worktreePath !== project.path) return 'Worktree';
  return 'Branch';
}

export function workItemMeta(item: WorkItem) {
  const syncState = pullRequestSyncState(item);
  if (syncState === 'localChanges') return 'Local changes';
  if (syncState === 'localAhead') return localCommitLabel(item.pullRequest!.localCommits);
  if (syncState === 'remoteAhead') return 'Checkout behind';
  if (syncState === 'diverged') return 'Diverged';
  if (item.pullRequest) return `#${item.pullRequest.number}`;
  if (item.status === 'working') {
    if (item.additions > 0 || item.deletions > 0) {
      return `+${item.additions} −${item.deletions}`;
    }
    return `${item.changedFiles} file${item.changedFiles === 1 ? '' : 's'}`;
  }

  return relativeTime(item.updatedAt);
}

export function pullRequestSyncState(item: WorkItem): PullRequestSyncState | null {
  const pullRequest = item.pullRequest;
  if (!pullRequest) return null;
  if (item.changedFiles > 0) return 'localChanges';
  if (pullRequest.localCommits > 0 && pullRequest.remoteCommits > 0) return 'diverged';
  if (pullRequest.localCommits > 0) return 'localAhead';
  if (pullRequest.remoteCommits > 0 || item.headSha !== pullRequest.headSha) return 'remoteAhead';
  return 'synced';
}

export function pullRequestSyncLabel(item: WorkItem) {
  const state = pullRequestSyncState(item);
  if (state === 'localChanges') return 'Local changes not in PR';
  if (state === 'localAhead') return `${localCommitLabel(item.pullRequest!.localCommits)} not in PR`;
  if (state === 'remoteAhead') return 'Checkout behind PR';
  if (state === 'diverged') return 'Local and PR diverged';
  return null;
}

function localCommitLabel(count: number) {
  return `${count} local commit${count === 1 ? '' : 's'}`;
}

export function relativeTime(timestamp: number) {
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - timestamp);
  if (seconds < 60) return 'now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}

function fileName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop();
}
