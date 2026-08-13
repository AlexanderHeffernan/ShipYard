export type CommitVerification = 'verified' | 'unverified' | 'unsigned' | 'unknown';

export type CommitHistorySource = 'local' | 'pullRequest' | 'detached';

export type WorkItemCommitsRequest = {
  projectPath: string;
  projectId: string;
  branch: string | null;
  worktreePath: string | null;
  headSha: string;
  defaultBranch: string | null;
  pullRequestNumber: number | null;
  pullRequestBaseBranch: string | null;
  pullRequestHeadSha: string | null;
};

export type WorkItemCommit = {
  sha: string;
  shortSha: string;
  subject: string;
  body: string;
  authorName: string;
  authorEmail: string;
  authoredAt: number;
  committedAt: number;
  verification: CommitVerification;
  verificationSigner: string | null;
  parentCount: number;
  additions: number;
  deletions: number;
  changedFiles: number;
};

export type WorkItemCommitHistory = {
  commits: WorkItemCommit[];
  total: number;
  hasMore: boolean;
  source: CommitHistorySource;
  comparisonLabel: string | null;
  headSha: string | null;
};
