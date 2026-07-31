export type WorkStatus = 'working' | 'ready' | 'shipped' | 'mergeConflict';

export type WorkItem = {
  id: string;
  projectId: string;
  branch: string | null;
  worktreePath: string | null;
  resolutionPath: string | null;
  headSha: string;
  lastCommitSubject: string;
  status: WorkStatus;
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
};

export type Project = ScannedProject & {
  color: string;
};
