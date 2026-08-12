export type PullRequestRequest = {
  repository: string;
  number: number;
};

export type PullRequestCommentRequest = PullRequestRequest & {
  body: string;
};

export type PullRequestCheck = {
  id: string;
  name: string;
  workflowName: string | null;
  status: string;
  conclusion: string | null;
  startedAt: string | null;
  completedAt: string | null;
  url: string | null;
};

export type PullRequestChecks = {
  overallState: 'success' | 'failure' | 'pending' | 'neutral' | 'unknown';
  total: number;
  passed: number;
  failed: number;
  pending: number;
  neutral: number;
  lastUpdatedAt: string | null;
  checks: PullRequestCheck[];
};

export type ConversationAuthor = {
  login: string;
  avatarUrl: string | null;
  profileUrl: string | null;
  name: string | null;
};

export type ConversationEntry = {
  id: string;
  kind: 'comment' | 'review' | 'reviewComment' | 'system';
  author: ConversationAuthor | null;
  body: string;
  timestamp: string;
  updatedAt: string | null;
  state: string | null;
  url: string | null;
  path: string | null;
  line: number | null;
};

export type PullRequestConversation = {
  viewerLogin: string | null;
  entries: ConversationEntry[];
};
