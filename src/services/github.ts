import { invoke } from '@tauri-apps/api/core';
import type {
  PullRequestChecks,
  PullRequestCommentRequest,
  PullRequestConversation,
  ConversationEntry,
  PullRequestRequest,
} from '../types/github';

export function getPullRequestChecks(request: PullRequestRequest) {
  return invoke<PullRequestChecks>('get_pull_request_checks', { request });
}

export function getPullRequestConversation(request: PullRequestRequest) {
  return invoke<PullRequestConversation>('get_pull_request_conversation', { request });
}

export function postPullRequestComment(request: PullRequestCommentRequest) {
  return invoke<ConversationEntry>('post_pull_request_comment', { request });
}
