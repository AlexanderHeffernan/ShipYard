<script setup lang="ts">
import { openUrl } from '@tauri-apps/plugin-opener';
import {
  AlertCircle,
  CheckCircle2,
  ExternalLink,
  FileCode2,
  GitPullRequest,
  LoaderCircle,
  MessageSquare,
  RefreshCw,
  Send,
  ShieldAlert,
} from '@lucide/vue';
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { getPullRequestConversation, postPullRequestComment } from '../../services/github';
import type { ConversationEntry, PullRequestConversation } from '../../types/github';
import type { Project, WorkItem } from '../../types/projects';
import {
  authorInitials,
  authorLabel,
  conversationKindLabel,
  formatTimestamp,
  relativeTimestamp,
} from '../../utils/github';
import { renderMarkdown } from '../../utils/markdown';

const MAX_COMMENT_LENGTH = 10_000;

const props = defineProps<{
  project: Project;
  workItem: WorkItem;
}>();

const loading = ref(false);
const error = ref<string | null>(null);
const conversation = ref<PullRequestConversation | null>(null);
const draft = ref('');
const composerError = ref<string | null>(null);
const posting = ref(false);
const posted = ref(false);
let requestVersion = 0;
let postedTimer: number | undefined;

const request = computed(() => {
  const pullRequest = props.workItem.pullRequest;
  if (!pullRequest || !props.project.githubRepository) return null;
  return { repository: props.project.githubRepository, number: pullRequest.number };
});

const entries = computed(() => conversation.value?.entries ?? []);
const discussionCount = computed(() => entries.value.filter((entry) => entry.kind !== 'system').length);
const viewerLogin = computed(() => conversation.value?.viewerLogin ?? null);

function message(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}

async function load() {
  const currentRequest = request.value;
  const version = ++requestVersion;
  if (!currentRequest) {
    conversation.value = null;
    error.value = 'This pull request is not connected to a GitHub repository.';
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    const result = await getPullRequestConversation(currentRequest);
    if (version === requestVersion) conversation.value = result;
  } catch (loadError) {
    if (version === requestVersion) error.value = message(loadError);
  } finally {
    if (version === requestVersion) loading.value = false;
  }
}

function validateComment() {
  const value = draft.value.trim();
  if (!value) return 'Write a comment before posting.';
  if (value.length > MAX_COMMENT_LENGTH) return `Comments are limited to ${MAX_COMMENT_LENGTH.toLocaleString()} characters.`;
  return null;
}

async function postComment() {
  composerError.value = validateComment();
  posted.value = false;
  if (composerError.value || posting.value || !request.value) return;

  posting.value = true;
  try {
    const entry = await postPullRequestComment({ ...request.value, body: draft.value.trim() });
    const current = conversation.value;
    if (current) {
      conversation.value = {
        ...current,
        entries: [...current.entries, entry].sort((left, right) => left.timestamp.localeCompare(right.timestamp)),
      };
    }
    draft.value = '';
    composerError.value = null;
    posted.value = true;
    window.clearTimeout(postedTimer);
    postedTimer = window.setTimeout(() => (posted.value = false), 2600);
  } catch (postError) {
    composerError.value = message(postError);
  } finally {
    posting.value = false;
  }
}

async function openExternal(url: string | null) {
  if (!url) return;
  try {
    await openUrl(url);
  } catch (openError) {
    error.value = message(openError);
  }
}

function handleAvatarError(event: Event) {
  (event.target as HTMLImageElement).style.display = 'none';
}

function entryIsOwn(entry: ConversationEntry) {
  return !!viewerLogin.value && entry.author?.login === viewerLogin.value;
}

function entryStateClass(entry: ConversationEntry) {
  if (entry.kind === 'system') return 'conversation-entry--system';
  if (entry.kind === 'review' && entry.state === 'APPROVED') return 'conversation-entry--approved';
  if (entry.kind === 'review' && entry.state === 'CHANGES_REQUESTED') return 'conversation-entry--changes';
  return '';
}

watch(
  () => `${props.project.id}:${props.workItem.id}:${props.workItem.pullRequest?.number ?? 'local'}`,
  () => void load(),
  { immediate: true },
);

onBeforeUnmount(() => {
  requestVersion += 1;
  window.clearTimeout(postedTimer);
});
</script>

<template>
  <section class="conversation" aria-labelledby="conversation-title">
    <header class="conversation__toolbar">
      <div>
        <span class="conversation__eyebrow"><MessageSquare aria-hidden="true" /> Pull request discussion</span>
        <h2 id="conversation-title">Conversation</h2>
        <p>Chronological comments, reviews, and GitHub activity for pull request #{{ workItem.pullRequest?.number }}</p>
      </div>
      <div class="conversation__toolbar-actions">
        <button
          type="button"
          class="conversation__icon-button"
          :aria-busy="loading"
          aria-label="Refresh conversation"
          title="Refresh conversation"
          @click="load"
        >
          <RefreshCw :class="{ 'is-spinning': loading }" aria-hidden="true" />
        </button>
        <button v-if="workItem.pullRequest" type="button" class="conversation__open-button" @click="openExternal(workItem.pullRequest.url)">
          <GitPullRequest aria-hidden="true" /> Open PR
        </button>
      </div>
    </header>

    <div v-if="loading" class="conversation__state" aria-live="polite">
      <span class="conversation__spinner"></span>
      <strong>Loading conversation…</strong>
      <span>Gathering comments, reviews, and activity from GitHub.</span>
      <div class="conversation__skeletons" aria-hidden="true">
        <i v-for="index in 3" :key="index"></i>
      </div>
    </div>

    <div v-else-if="error" class="conversation__state conversation__state--error" role="alert">
      <AlertCircle aria-hidden="true" />
      <strong>Couldn’t load conversation</strong>
      <span>{{ error }}</span>
      <button type="button" @click="load">Try again</button>
    </div>

    <div v-else-if="conversation" class="conversation__content">
      <div class="conversation__summary">
        <span><MessageSquare aria-hidden="true" /> {{ discussionCount }} discussion item{{ discussionCount === 1 ? '' : 's' }}</span>
        <span v-if="conversation.viewerLogin">Posting as <strong>@{{ conversation.viewerLogin }}</strong></span>
      </div>

      <div v-if="entries.length === 0" class="conversation__empty">
        <MessageSquare aria-hidden="true" />
        <strong>No conversation yet</strong>
        <p>Start the discussion with a top-level comment below.</p>
      </div>

      <ol v-else class="conversation__timeline" aria-label="Pull request conversation">
        <li v-for="entry in entries" :key="entry.id" class="conversation-entry" :class="entryStateClass(entry)">
          <div class="conversation-entry__rail" aria-hidden="true"><span></span></div>
          <div class="conversation-entry__avatar" :class="{ 'conversation-entry__avatar--system': entry.kind === 'system' }">
            <img
              v-if="entry.author?.avatarUrl"
              :src="entry.author.avatarUrl"
              :alt="`${authorLabel(entry.author)} avatar`"
              @error="handleAvatarError"
            />
            <span>{{ authorInitials(entry.author) }}</span>
          </div>
          <article class="conversation-entry__card">
            <header class="conversation-entry__header">
              <div class="conversation-entry__author">
                <strong>{{ authorLabel(entry.author) }}</strong>
                <span v-if="entryIsOwn(entry)" class="conversation-entry__you">You</span>
                <span class="conversation-entry__kind">
                  <CheckCircle2 v-if="entry.kind === 'review' && entry.state === 'APPROVED'" aria-hidden="true" />
                  <ShieldAlert v-else-if="entry.kind === 'review' && entry.state === 'CHANGES_REQUESTED'" aria-hidden="true" />
                  <FileCode2 v-else-if="entry.kind === 'reviewComment'" aria-hidden="true" />
                  <ShieldAlert v-else-if="entry.kind === 'system'" aria-hidden="true" />
                  <MessageSquare v-else aria-hidden="true" />
                  {{ conversationKindLabel(entry) }}
                </span>
              </div>
              <time :datetime="entry.timestamp" :title="formatTimestamp(entry.timestamp)">
                {{ relativeTimestamp(entry.timestamp) || formatTimestamp(entry.timestamp) }}
              </time>
            </header>
            <div v-if="entry.path" class="conversation-entry__location">
              <FileCode2 aria-hidden="true" />
              <code>{{ entry.path }}<span v-if="entry.line">:{{ entry.line }}</span></code>
              <span>Inline review comment</span>
            </div>
            <div class="conversation-entry__body" v-html="renderMarkdown(entry.body)"></div>
            <footer v-if="entry.updatedAt || entry.url" class="conversation-entry__footer">
              <span v-if="entry.updatedAt">Edited {{ formatTimestamp(entry.updatedAt) }}</span>
              <button v-if="entry.url" type="button" @click="openExternal(entry.url)">View on GitHub <ExternalLink aria-hidden="true" /></button>
            </footer>
          </article>
        </li>
      </ol>

      <form class="conversation__composer" aria-labelledby="composer-title" @submit.prevent="postComment">
        <div class="conversation__composer-heading">
          <div>
            <span class="conversation__eyebrow"><Send aria-hidden="true" /> Add to conversation</span>
            <h3 id="composer-title">Leave a comment</h3>
          </div>
          <span class="conversation__character-count" :class="{ 'conversation__character-count--near-limit': draft.length > MAX_COMMENT_LENGTH * .9 }">
            {{ draft.length.toLocaleString() }} / {{ MAX_COMMENT_LENGTH.toLocaleString() }}
          </span>
        </div>
        <textarea
          v-model="draft"
          :disabled="posting"
          aria-label="New pull request comment"
          :aria-invalid="!!composerError"
          :aria-describedby="composerError ? 'comment-error' : 'comment-help'"
          maxlength="10000"
          rows="4"
          placeholder="Share context, ask a question, or leave a note…"
          @input="composerError = null"
        ></textarea>
        <div class="conversation__composer-footer">
          <span id="comment-help">Markdown is supported. This posts a top-level GitHub comment; review actions stay on GitHub.</span>
          <button type="submit" :disabled="posting">
            <LoaderCircle v-if="posting" class="is-spinning" aria-hidden="true" />
            <CheckCircle2 v-else-if="posted" aria-hidden="true" />
            <Send v-else aria-hidden="true" />
            {{ posting ? 'Posting…' : posted ? 'Posted' : 'Post comment' }}
          </button>
        </div>
        <p v-if="composerError" id="comment-error" class="conversation__composer-error" role="alert">{{ composerError }}</p>
      </form>

      <p class="conversation__limitation"><ShieldAlert aria-hidden="true" /> Shipyard can read discussion and post top-level comments. Approvals, change requests, replies, and inline review actions remain available on GitHub.</p>
    </div>
  </section>
</template>

<style scoped>
.conversation { display: flex; width: 100%; min-width: 0; min-height: 0; flex: 1; flex-direction: column; overflow: hidden; background: var(--surface-content); }
.conversation__toolbar { display: flex; flex: 0 0 auto; align-items: center; justify-content: space-between; gap: 18px; min-height: 78px; padding: 17px 18px 15px; border-bottom: 1px solid var(--border-subtle); background: rgba(16, 13, 24, .82); }
.conversation__toolbar h2 { margin: 3px 0 0; font-size: 17px; font-weight: 560; letter-spacing: -.01em; }
.conversation__toolbar p { margin: 4px 0 0; color: var(--text-secondary); font-size: 11px; }
.conversation__eyebrow { display: flex; align-items: center; gap: 5px; color: var(--primary-hover); font-size: 9px; font-weight: 650; letter-spacing: .08em; text-transform: uppercase; }
.conversation__eyebrow svg { width: 12px; height: 12px; }
.conversation__toolbar-actions { display: flex; align-items: center; gap: 6px; }
.conversation__icon-button, .conversation__open-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 29px; color: var(--text-secondary); background: var(--surface-subtle); border: 1px solid var(--border-subtle); border-radius: 7px; font: inherit; font-size: 11px; }
.conversation__icon-button { width: 29px; padding: 0; }
.conversation__open-button { padding: 0 9px; }
.conversation__icon-button:hover, .conversation__open-button:hover { color: var(--text-primary); background: var(--surface-hover); border-color: var(--border-strong); }
.conversation__icon-button:focus-visible, .conversation__open-button:focus-visible, .conversation__state button:focus-visible, .conversation-entry button:focus-visible, .conversation__composer button:focus-visible, .conversation__composer textarea:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
.conversation__icon-button svg, .conversation__open-button svg { width: 14px; height: 14px; }
.conversation__content { min-height: 0; flex: 1; overflow-y: auto; padding: 15px max(18px, calc((100% - 850px) / 2)); scrollbar-width: thin; }
.conversation__summary { display: flex; align-items: center; justify-content: space-between; min-height: 31px; color: var(--text-muted); font-size: 10px; }
.conversation__summary span { display: inline-flex; align-items: center; gap: 5px; }
.conversation__summary svg { width: 12px; height: 12px; }
.conversation__summary strong { color: var(--text-secondary); font-weight: 550; }
.conversation__timeline { position: relative; display: flex; margin: 3px 0 21px; padding: 0; flex-direction: column; gap: 12px; list-style: none; }
.conversation-entry { position: relative; display: grid; grid-template-columns: 30px minmax(0, 1fr); gap: 10px; }
.conversation-entry__rail { position: absolute; top: 30px; bottom: -13px; left: 14px; width: 1px; background: var(--border-subtle); }
.conversation-entry:last-child .conversation-entry__rail { display: none; }
.conversation-entry__avatar { position: relative; z-index: 1; display: grid; width: 30px; height: 30px; place-items: center; overflow: hidden; color: var(--primary-hover); background: var(--primary-subtle); border: 1px solid var(--primary-border); border-radius: 50%; font-size: 9px; font-weight: 650; }
.conversation-entry__avatar img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
.conversation-entry__avatar--system { color: var(--text-secondary); background: var(--surface-subtle); border-color: var(--border-strong); }
.conversation-entry__card { min-width: 0; padding: 11px 13px 10px; background: var(--surface-elevated); border: 1px solid var(--border-subtle); border-radius: 9px; }
.conversation-entry--approved .conversation-entry__card { border-color: var(--success-border); }
.conversation-entry--changes .conversation-entry__card { border-color: var(--danger-border); }
.conversation-entry--system .conversation-entry__card { background: var(--surface-subtle); border-style: dashed; }
.conversation-entry__header { display: flex; align-items: center; justify-content: space-between; gap: 10px; min-height: 18px; }
.conversation-entry__author { display: flex; min-width: 0; align-items: center; flex-wrap: wrap; gap: 6px; }
.conversation-entry__author > strong { overflow: hidden; max-width: 220px; font-size: 11px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
.conversation-entry__header > time { flex: 0 0 auto; color: var(--text-muted); font-size: 9px; }
.conversation-entry__you { padding: 2px 5px; color: var(--primary); background: var(--primary-subtle); border-radius: 4px; font-size: 8px; font-weight: 650; text-transform: uppercase; letter-spacing: .04em; }
.conversation-entry__kind { display: inline-flex; align-items: center; gap: 3px; padding: 2px 5px; color: var(--text-secondary); background: rgba(255,255,255,.055); border-radius: 4px; font-size: 8px; font-weight: 600; text-transform: uppercase; letter-spacing: .035em; }
.conversation-entry__kind svg { width: 10px; height: 10px; }
.conversation-entry--approved .conversation-entry__kind { color: var(--success); background: var(--success-subtle); }
.conversation-entry--changes .conversation-entry__kind { color: var(--danger); background: var(--danger-subtle); }
.conversation-entry__location { display: flex; align-items: center; gap: 5px; margin-top: 9px; padding: 5px 7px; color: var(--text-secondary); background: rgba(0,0,0,.15); border-radius: 5px; font-size: 9px; }
.conversation-entry__location svg { width: 12px; height: 12px; color: var(--primary); }
.conversation-entry__location code { color: var(--text-primary); font: 10px ui-monospace, SFMono-Regular, Menlo, monospace; }
.conversation-entry__location > span { margin-left: auto; color: var(--text-muted); }
.conversation-entry__body { margin-top: 9px; color: var(--text-primary); font-size: 11px; line-height: 1.55; overflow-wrap: anywhere; }
.conversation-entry__body :deep(p) { margin: 0 0 8px; }
.conversation-entry__body :deep(p:last-child) { margin-bottom: 0; }
.conversation-entry__body :deep(h1), .conversation-entry__body :deep(h2), .conversation-entry__body :deep(h3) { margin: 10px 0 5px; font-size: 12px; font-weight: 600; }
.conversation-entry__body :deep(ul), .conversation-entry__body :deep(ol) { margin: 5px 0 8px; padding-left: 19px; }
.conversation-entry__body :deep(li + li) { margin-top: 3px; }
.conversation-entry__body :deep(blockquote) { margin: 7px 0; padding-left: 10px; color: var(--text-secondary); border-left: 2px solid var(--border-strong); }
.conversation-entry__body :deep(code) { padding: 1px 4px; color: var(--primary-hover); background: rgba(0,0,0,.22); border-radius: 3px; font: 10px ui-monospace, SFMono-Regular, Menlo, monospace; }
.conversation-entry__body :deep(pre) { margin: 7px 0; padding: 9px; overflow-x: auto; background: var(--surface-input); border: 1px solid var(--border-subtle); border-radius: 5px; }
.conversation-entry__body :deep(pre code) { padding: 0; color: var(--text-primary); background: transparent; }
.conversation-entry__body :deep(a) { color: var(--primary-hover); text-decoration: underline; text-underline-offset: 2px; }
.conversation-entry__footer { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 9px; color: var(--text-muted); font-size: 9px; }
.conversation-entry__footer button { display: inline-flex; align-items: center; gap: 4px; padding: 0; color: var(--text-secondary); background: transparent; border: 0; font: inherit; font-size: 9px; }
.conversation-entry__footer button:hover { color: var(--primary-hover); }
.conversation-entry__footer svg { width: 11px; height: 11px; }
.conversation__composer { padding: 13px; background: var(--surface-elevated); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 7px 24px rgba(5,3,8,.18); }
.conversation__composer-heading { display: flex; align-items: start; justify-content: space-between; gap: 10px; }
.conversation__composer-heading h3 { margin: 3px 0 0; font-size: 12px; font-weight: 600; }
.conversation__character-count { color: var(--text-muted); font: 9px ui-monospace, SFMono-Regular, Menlo, monospace; }
.conversation__character-count--near-limit { color: var(--warning); }
.conversation__composer textarea { display: block; width: 100%; min-height: 82px; margin-top: 11px; resize: vertical; padding: 9px 10px; color: var(--text-primary); background: var(--surface-input); border: 1px solid var(--border-subtle); border-radius: 6px; font: inherit; font-size: 11px; line-height: 1.5; }
.conversation__composer textarea::placeholder { color: var(--text-muted); }
.conversation__composer textarea:focus { border-color: var(--primary-border); box-shadow: 0 0 0 2px var(--primary-subtle); outline: none; }
.conversation__composer textarea:disabled { opacity: .65; }
.conversation__composer-footer { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-top: 8px; }
.conversation__composer-footer > span { color: var(--text-muted); font-size: 9px; line-height: 1.4; }
.conversation__composer-footer button { display: inline-flex; align-items: center; gap: 5px; flex: 0 0 auto; height: 28px; padding: 0 9px; color: var(--primary-foreground); background: var(--primary); border: 1px solid var(--primary); border-radius: 6px; font: inherit; font-size: 10px; font-weight: 600; }
.conversation__composer-footer button:hover:not(:disabled) { background: var(--primary-hover); border-color: var(--primary-hover); }
.conversation__composer-footer button:disabled { opacity: .45; }
.conversation__composer-footer svg { width: 13px; height: 13px; }
.conversation__composer-error { margin: 8px 0 0; color: var(--danger); font-size: 10px; line-height: 1.4; }
.conversation__limitation { display: flex; align-items: start; gap: 6px; margin: 10px 1px 4px; color: var(--text-muted); font-size: 9px; line-height: 1.45; }
.conversation__limitation svg { width: 12px; height: 12px; flex: 0 0 auto; margin-top: 1px; }
.conversation__empty, .conversation__state { display: flex; align-items: center; justify-content: center; min-height: 230px; flex-direction: column; gap: 8px; color: var(--text-muted); text-align: center; }
.conversation__empty { padding: 32px 20px; }
.conversation__empty > svg { width: 25px; height: 25px; color: var(--text-secondary); }
.conversation__empty strong, .conversation__state strong { color: var(--text-primary); font-size: 13px; font-weight: 550; }
.conversation__empty p, .conversation__state > span:not(.conversation__spinner) { margin: 0; color: var(--text-secondary); font-size: 11px; }
.conversation__state--error > svg { width: 25px; height: 25px; color: var(--danger); }
.conversation__state--error strong { color: var(--danger); }
.conversation__state button { margin-top: 4px; padding: 6px 9px; color: var(--primary); background: var(--primary-subtle); border: 1px solid var(--primary-border); border-radius: 6px; font: inherit; font-size: 10px; }
.conversation__spinner { width: 18px; height: 18px; border: 2px solid var(--border-strong); border-top-color: var(--primary); border-radius: 50%; animation: spin .8s linear infinite; }
.conversation__skeletons { display: flex; width: min(420px, 80%); flex-direction: column; gap: 7px; margin-top: 8px; }
.conversation__skeletons i { display: block; height: 40px; background: linear-gradient(90deg, var(--surface-subtle), rgba(255,255,255,.08), var(--surface-subtle)); background-size: 200% 100%; border-radius: 6px; animation: shimmer 1.4s ease-in-out infinite; }
.is-spinning { animation: spin .85s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }
@media (max-width: 680px) { .conversation__content { padding-inline: 11px; } .conversation__toolbar p { max-width: 270px; } .conversation__composer-footer { align-items: end; flex-direction: column; } .conversation__composer-footer button { align-self: end; } .conversation-entry__header { align-items: start; flex-direction: column; gap: 4px; } .conversation-entry__location > span { display: none; } }
@media (prefers-reduced-motion: reduce) { .is-spinning, .conversation__spinner, .conversation__skeletons i { animation: none; } }
</style>
