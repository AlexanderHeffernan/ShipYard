import type { ConversationAuthor, ConversationEntry, PullRequestCheck } from '../types/github';

export type CheckTone = 'success' | 'danger' | 'pending' | 'neutral';

export function checkTone(check: PullRequestCheck): CheckTone {
  const value = (check.conclusion || check.status).toUpperCase();
  if (['SUCCESS', 'NEUTRAL', 'SKIPPED'].includes(value)) return 'success';
  if (['FAILURE', 'ERROR', 'CANCELLED', 'TIMED_OUT', 'ACTION_REQUIRED'].includes(value)) return 'danger';
  if (['IN_PROGRESS', 'QUEUED', 'PENDING', 'EXPECTED', 'REQUESTED'].includes(value)) return 'pending';
  return 'neutral';
}

export function checkOutcomeLabel(check: PullRequestCheck) {
  const value = (check.conclusion || check.status).toUpperCase();
  return ({
    SUCCESS: 'Passed',
    FAILURE: 'Failed',
    ERROR: 'Error',
    CANCELLED: 'Cancelled',
    TIMED_OUT: 'Timed out',
    ACTION_REQUIRED: 'Action required',
    IN_PROGRESS: 'In progress',
    QUEUED: 'Queued',
    PENDING: 'Pending',
    EXPECTED: 'Waiting',
    REQUESTED: 'Waiting',
    NEUTRAL: 'Neutral',
    SKIPPED: 'Skipped',
  } as Record<string, string>)[value] ?? 'No conclusion';
}

export function checkIsComplete(check: PullRequestCheck) {
  return !!check.completedAt || !!check.conclusion;
}

export function formatCheckDuration(check: PullRequestCheck, now = Date.now()) {
  if (!check.startedAt) return 'Timing unavailable';
  const start = Date.parse(check.startedAt);
  if (Number.isNaN(start)) return 'Timing unavailable';
  const end = check.completedAt ? Date.parse(check.completedAt) : now;
  const seconds = Math.max(0, Math.round((end - start) / 1000));
  const suffix = checkIsComplete(check) ? '' : ' elapsed';
  return `${formatDuration(seconds)}${suffix}`;
}

export function formatDuration(seconds: number) {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) return `${minutes}m${remainingSeconds ? ` ${remainingSeconds}s` : ''}`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return `${hours}h${remainingMinutes ? ` ${remainingMinutes}m` : ''}`;
}

export function formatTimestamp(value: string | null | undefined) {
  if (!value) return 'Time unavailable';
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) return 'Time unavailable';
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(timestamp);
}

export function relativeTimestamp(value: string | null | undefined, now = Date.now()) {
  if (!value) return '';
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) return '';
  const seconds = Math.max(0, Math.floor((now - timestamp) / 1000));
  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  if (seconds < 604800) return `${Math.floor(seconds / 86400)}d ago`;
  return formatTimestamp(value);
}

export function authorLabel(author: ConversationAuthor | null) {
  if (!author) return 'GitHub user';
  return author.name?.trim() || author.login;
}

export function authorInitials(author: ConversationAuthor | null) {
  const value = authorLabel(author);
  const words = value.split(/\s+/).filter(Boolean);
  if (words.length > 1) return `${words[0][0]}${words[words.length - 1][0]}`.toUpperCase();
  return value.slice(0, 2).toUpperCase() || 'GH';
}

export function conversationKindLabel(entry: ConversationEntry) {
  if (entry.kind === 'reviewComment') return 'Inline review';
  if (entry.kind === 'review') {
    if (entry.state === 'APPROVED') return 'Approved';
    if (entry.state === 'CHANGES_REQUESTED') return 'Changes requested';
    if (entry.state === 'DISMISSED') return 'Review dismissed';
    return 'Review';
  }
  if (entry.kind === 'system') return 'System';
  return 'Comment';
}
