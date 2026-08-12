export type WorkStatus = 'working' | 'ready' | 'shipped';

export type PullRequest = {
  number: number;
  title: string;
  url: string;
  draft: boolean;
  mergeable: boolean | null;
  mergeState: 'ready' | 'checksPending' | 'checksFailed' | 'reviewRequired' | 'conflicting' | 'draft';
  headBranch: string;
  headSha: string;
  localCommits: number;
  remoteCommits: number;
};

export type WorkItem = {
  id: string;
  projectId: string;
  branch: string | null;
  worktreePath: string | null;
  headSha: string;
  lastCommitSubject: string;
  status: WorkStatus;
  pullRequest: PullRequest | null;
  completed: boolean;
  additions: number;
  deletions: number;
  changedFiles: number;
  ahead: number;
  behind: number;
  updatedAt: number;
};

export type ScannedProject = {
  id: string;
  name: string;
  path: string;
  defaultBranch: string | null;
  workItems: WorkItem[];
  githubRepository: string | null;
  githubError: string | null;
};

export type Project = ScannedProject & {
  color: string;
};

export type WorkItemDiffRequest = {
  projectPath: string;
  projectId: string;
  branch: string | null;
  worktreePath: string | null;
  headSha: string;
  defaultBranch: string | null;
};

export type WorkItemDiff = {
  patch: string;
  comparisonLabel: string;
};

export type DeleteWorkItemRequest = {
  projectPath: string;
  projectId: string;
  workItemId: string;
  branch: string | null;
  worktreePath: string | null;
  headSha: string;
};

export type DeletionPlan = {
  projectId: string;
  workItemId: string;
  branch: string | null;
  worktreePath: string | null;
  defaultBranch: string | null;
  removesWorktree: boolean;
  deletesBranch: boolean;
  switchesPrimaryCheckout: boolean;
  hasUncommittedChanges: boolean;
  unpushedCommits: number;
};

export type DeletionResult = {
  projectId: string;
  workItemId: string;
  worktreeRemoved: boolean;
  branchDeleted: boolean;
  switchedPrimaryToDefault: boolean;
};
