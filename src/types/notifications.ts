export type NotificationSettings = {
  newPullRequests: boolean;
  pullRequestUpdates: boolean;
};

export type NotificationPermissionState =
  | 'granted'
  | 'denied'
  | 'prompt'
  | 'unsupported'
  | 'unknown';

export type NotificationPullRequestSnapshot = {
  number: number;
  headSha: string;
  draft: boolean;
  mergeState: string;
  attentionState: string;
  baseBranch: string;
};

export type NotificationProjectSnapshot = {
  id: string;
  name: string;
  available: boolean;
  pullRequests: NotificationPullRequestSnapshot[];
};

export type NotificationEvent = {
  kind: 'newPullRequest' | 'pullRequestUpdated';
  identity: string;
  projectId: string;
  pullRequestNumber: number;
  title: string;
  body: string;
};

export const DEFAULT_NOTIFICATION_SETTINGS: NotificationSettings = {
  newPullRequests: false,
  pullRequestUpdates: false,
};
