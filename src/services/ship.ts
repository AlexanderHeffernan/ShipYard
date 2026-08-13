import { invoke } from '@tauri-apps/api/core';
import type { Project, WorkItem } from '../types/projects';

export type ShippingAction = 'createPullRequest' | 'updatePullRequest' | 'mergePullRequest' | 'resolvePullRequest' | 'directToMain';

export function shipWork(project: Project, item: WorkItem, action: ShippingAction) {
  return invoke<{ runId: string }>('ship_work', {
    request: {
      projectId: project.id,
      workItemId: item.id,
      sourcePath: item.worktreePath ?? project.path,
      sourceBranch: item.branch ?? item.pullRequest?.headBranch ?? null,
      defaultBranch: project.defaultBranch,
      githubRepository: project.githubRepository,
      action,
      pullRequestNumber: item.pullRequest?.number ?? null,
    },
  });
}
