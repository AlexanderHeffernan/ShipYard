import type { Project, WorkItem } from '../types/projects';

export function workItemTitle(item: WorkItem) {
  if (item.branch) return item.branch;
  if (item.worktreePath) return fileName(item.worktreePath) || 'Detached worktree';
  return `Detached at ${item.headSha.slice(0, 7)}`;
}

export function workItemKind(project: Project, item: WorkItem) {
  if (!item.branch) return 'Detached worktree';
  if (item.worktreePath && item.worktreePath !== project.path) return 'Worktree';
  return 'Branch';
}

export function workItemMeta(item: WorkItem) {
  if (item.status === 'working') {
    if (item.additions > 0 || item.deletions > 0) {
      return `+${item.additions} −${item.deletions}`;
    }
    return `${item.changedFiles} file${item.changedFiles === 1 ? '' : 's'}`;
  }

  return relativeTime(item.updatedAt);
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
