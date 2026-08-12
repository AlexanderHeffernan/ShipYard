import { invoke } from '@tauri-apps/api/core';
import type { Project, WorkItem } from '../types/projects';

export type ShippingAction =
  | 'createPullRequest'
  | 'updatePullRequest'
  | 'mergePullRequest'
  | 'directToMain'
  | 'pushBranch'
  | 'pushDefault'
  | 'integrateToDefault';

export function shipWork(project: Project, item: WorkItem, action: ShippingAction) {
  return invoke<{ runId: string }>('ship_work', {
    request: {
      projectId: project.id,
      workItemId: item.id,
      sourcePath: item.worktreePath ?? project.path,
      sourceBranch: item.branch,
      defaultBranch: project.defaultBranch,
      remoteName: project.remoteName,
      remoteIdentity: project.remoteIdentity,
      githubRepository: project.githubRepository,
      action,
      pullRequestNumber: item.pullRequest?.number ?? null,
    },
  });
}
