<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { getWorkItemCommits } from '../../services/projects';
import type { WorkItemCommit, WorkItemCommitHistory } from '../../types/commits';
import type { Project, WorkItem } from '../../types/projects';
import { pullRequestSyncState } from '../../utils/workItems';

const props = defineProps<{
  project: Project;
  workItem: WorkItem;
}>();

const emit = defineEmits<{
  showChanges: [];
}>();

const root = ref<HTMLElement>();
const searchInput = ref<HTMLInputElement>();
const loading = ref(false);
const error = ref<string | null>(null);
const query = ref('');
const history = ref<WorkItemCommitHistory | null>(null);
const expanded = ref(new Set<string>());
let requestVersion = 0;

const visibleCommits = computed(() => {
  const search = query.value.trim().toLocaleLowerCase();
  if (!search) return history.value?.commits ?? [];
  return (history.value?.commits ?? []).filter((commit) => [
    commit.subject,
    commit.body,
    commit.authorName,
    commit.authorEmail,
    commit.sha,
  ].some((value) => value.toLocaleLowerCase().includes(search)));
});

const sourceLabel = computed(() => {
  if (history.value?.source === 'pullRequest') {
    return `Pull request #${props.workItem.pullRequest?.number ?? ''}`.trim();
  }
  if (history.value?.source === 'local') {
    return props.workItem.branch ? `Local branch · ${props.workItem.branch}` : 'Local work';
  }
  return 'Detached worktree';
});

const summaryLabel = computed(() => {
  const total = history.value?.total ?? 0;
  return `${total} commit${total === 1 ? '' : 's'}`;
});

const syncState = computed(() => pullRequestSyncState(props.workItem));
const hasUncommittedChanges = computed(() => props.workItem.changedFiles > 0);
const emptyTitle = computed(() => hasUncommittedChanges.value
  ? 'No committed changes yet'
  : 'No commits in this work item');
const emptyDescription = computed(() => {
  if (hasUncommittedChanges.value) {
    return 'This local work only has uncommitted changes. Review the working tree in Changes.';
  }
  if (history.value?.comparisonLabel) {
    return `This work item currently has no commits ahead of ${history.value.comparisonLabel}.`;
  }
  return 'New commits will appear here after this work item is committed.';
});

function requestForWorkItem() {
  return {
    projectPath: props.project.path,
    projectId: props.project.id,
    branch: props.workItem.branch,
    worktreePath: props.workItem.worktreePath,
    headSha: props.workItem.headSha,
    defaultBranch: props.project.defaultBranch,
    pullRequestNumber: props.workItem.pullRequest?.number ?? null,
    pullRequestBaseBranch: props.workItem.pullRequest?.baseBranch ?? null,
    pullRequestHeadSha: props.workItem.pullRequest?.headSha ?? null,
  };
}

async function loadCommits() {
  const version = ++requestVersion;
  loading.value = true;
  error.value = null;
  history.value = null;
  expanded.value = new Set();
  try {
    const result = await getWorkItemCommits(requestForWorkItem());
    if (version !== requestVersion) return;
    history.value = result;
  } catch (loadError) {
    if (version === requestVersion) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    }
  } finally {
    if (version === requestVersion) loading.value = false;
  }
}

function toggleCommit(sha: string) {
  const next = new Set(expanded.value);
  if (next.has(sha)) next.delete(sha);
  else next.add(sha);
  expanded.value = next;
}

function focusCommit(index: number) {
  const commits = visibleCommits.value;
  if (commits.length === 0) return;
  const nextIndex = Math.min(commits.length - 1, Math.max(0, index));
  const button = root.value?.querySelector<HTMLButtonElement>(`[data-commit-index="${nextIndex}"]`);
  button?.focus();
}

function handleKeydown(event: KeyboardEvent) {
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
    event.preventDefault();
    searchInput.value?.focus();
    searchInput.value?.select();
    return;
  }
  if (event.key !== 'j' && event.key !== 'k' && event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
  event.preventDefault();
  const current = (event.target as HTMLElement).closest<HTMLButtonElement>('[data-commit-index]');
  const currentIndex = current ? Number(current.dataset.commitIndex) : -1;
  focusCommit(currentIndex + (event.key === 'j' || event.key === 'ArrowDown' ? 1 : -1));
}

function authorName(commit: WorkItemCommit) {
  return commit.authorName || commit.authorEmail || 'Unknown author';
}

function authorInitials(commit: WorkItemCommit) {
  const name = authorName(commit).replace(/[^\p{L}\p{N}]+/gu, ' ').trim();
  const words = name.split(/\s+/).filter(Boolean);
  if (words.length > 1) return `${words[0][0]}${words[words.length - 1][0]}`.toUpperCase();
  return (words[0] || '?').slice(0, 2).toUpperCase();
}

function authorColor(commit: WorkItemCommit) {
  let hash = 0;
  for (const character of authorName(commit)) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  const colors = ['#fb771f', '#a879ff', '#56b7ff', '#64cf8c', '#e7b950', '#f078b5'];
  return colors[hash % colors.length];
}

function authorTitle(commit: WorkItemCommit) {
  return commit.authorEmail ? `${authorName(commit)} <${commit.authorEmail}>` : authorName(commit);
}

function relativeDate(timestamp: number) {
  if (!timestamp) return 'Unknown date';
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - timestamp);
  if (seconds < 45) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  if (seconds < 2592000) return `${Math.floor(seconds / 86400)}d ago`;
  if (seconds < 31536000) return `${Math.floor(seconds / 2592000)}mo ago`;
  return `${Math.floor(seconds / 31536000)}y ago`;
}

function exactDate(timestamp: number) {
  if (!timestamp) return 'Date unavailable';
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(timestamp * 1000);
}

function isoDate(timestamp: number) {
  return timestamp ? new Date(timestamp * 1000).toISOString() : undefined;
}

function verificationLabel(commit: WorkItemCommit) {
  if (commit.verification === 'verified') return commit.verificationSigner ? `Verified by ${commit.verificationSigner}` : 'Verified commit';
  if (commit.verification === 'unverified') return 'Signature could not be verified';
  if (commit.verification === 'unsigned') return 'Unsigned commit';
  return 'Verification unavailable';
}

function statLabel(commit: WorkItemCommit) {
  if (commit.changedFiles === 0) return 'No file changes';
  return `${commit.changedFiles} file${commit.changedFiles === 1 ? '' : 's'}`;
}

function syncMessage() {
  if (hasUncommittedChanges.value) {
    return 'Uncommitted working-tree changes are not part of this history. Review them in Changes.';
  }
  if (!props.workItem.pullRequest) return null;
  if (syncState.value === 'localAhead') {
    return `${props.workItem.pullRequest.localCommits} local commit${props.workItem.pullRequest.localCommits === 1 ? '' : 's'} are not on the pull request yet. Showing the local branch history.`;
  }
  if (syncState.value === 'remoteAhead') {
    return 'The pull request has newer commits than this checkout. Refresh after checking out the latest work.';
  }
  if (syncState.value === 'diverged') {
    return 'The local branch and pull request have diverged, possibly after a rebase. Showing the local branch history for this work item.';
  }
  return null;
}

function focusSearch() {
  nextTick(() => {
    searchInput.value?.focus();
    searchInput.value?.select();
  });
}

watch(
  () => `${props.project.id}:${props.workItem.id}:${props.workItem.headSha}:${props.workItem.updatedAt}`,
  loadCommits,
  { immediate: true },
);
</script>

<template>
  <section ref="root" class="commits" tabindex="-1" @keydown="handleKeydown">
    <header class="commits__toolbar">
      <div class="commits__title-group">
        <span class="commits__title-icon" aria-hidden="true">
          <svg viewBox="0 0 20 20"><circle cx="6" cy="4" r="2" /><circle cx="14" cy="16" r="2" /><path d="M6 6v4a4 4 0 0 0 4 4h2M14 14v-2a4 4 0 0 0-4-4H8" /></svg>
        </span>
        <div>
          <strong>Commit history</strong>
          <span>{{ history ? summaryLabel : 'Inspecting this work item' }}</span>
        </div>
      </div>

      <div class="commits__toolbar-actions">
        <label class="commit-search">
          <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.25" /><path d="m10.25 10.25 3 3" /></svg>
          <input
            ref="searchInput"
            v-model="query"
            type="search"
            placeholder="Search commits"
            aria-label="Search commits"
            @keydown.escape="query = ''; ($event.target as HTMLInputElement).blur()"
          />
          <button v-if="query" type="button" aria-label="Clear commit search" @click="query = ''; focusSearch()">×</button>
          <kbd v-else>⌘F</kbd>
        </label>
        <button class="commits__refresh" type="button" :disabled="loading" title="Refresh commit history" aria-label="Refresh commit history" @click="loadCommits">
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M13 5.5A5 5 0 1 0 13.5 9" /><path d="M13 2.75v2.75h-2.75" /></svg>
        </button>
      </div>
    </header>

    <div v-if="loading" class="commits__state" aria-live="polite">
      <span class="commits__spinner"></span>
      <strong>Loading commit history…</strong>
      <span>Reading the commits that make up this work item.</span>
    </div>
    <div v-else-if="error" class="commits__state commits__state--error" role="alert">
      <span class="commits__state-icon" aria-hidden="true">
        <svg viewBox="0 0 20 20"><circle cx="10" cy="10" r="7" /><path d="M10 6.5v4.25M10 13.5v.1" /></svg>
      </span>
      <strong>Couldn’t load commits</strong>
      <span>{{ error }}</span>
      <button type="button" @click="loadCommits">Try again</button>
    </div>
    <div v-else-if="history" class="commits__workspace">
      <div class="commits__overview">
        <div class="commits__overview-left">
          <span class="commits__source">
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 3.5v9M4 5.25h6.5a2 2 0 1 1 0 4H4" /><circle cx="12" cy="11.5" r="1.25" /></svg>
            {{ sourceLabel }}
          </span>
          <span v-if="history.comparisonLabel" class="commits__comparison">{{ history.total ? 'ahead of' : 'compared with' }} {{ history.comparisonLabel }}</span>
          <span v-else class="commits__comparison">Full reachable history</span>
        </div>
        <span v-if="query" class="commits__filtered-count">{{ visibleCommits.length }} of {{ history.commits.length }} shown</span>
        <span v-else-if="history.hasMore" class="commits__filtered-count">Showing {{ history.commits.length }} of {{ history.total }}</span>
      </div>

      <div v-if="syncMessage()" class="commits__notice" :class="{ 'commits__notice--warning': syncState && syncState !== 'synced' }" role="status">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 2.25 14 13.5H2L8 2.25Z" /><path d="M8 6v3M8 11.25v.1" /></svg>
        <span>{{ syncMessage() }}</span>
        <button v-if="hasUncommittedChanges" type="button" @click="emit('showChanges')">Open Changes</button>
      </div>

      <div v-if="history.commits.length === 0" class="commits__state commits__state--empty">
        <span class="commits__empty-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24"><circle cx="7" cy="5" r="2.25" /><circle cx="17" cy="19" r="2.25" /><path d="M7 7.5v4a5 5 0 0 0 5 5h2.75M17 16.75V14a5 5 0 0 0-5-5H9.5" /></svg>
        </span>
        <strong>{{ emptyTitle }}</strong>
        <span>{{ emptyDescription }}</span>
        <button v-if="hasUncommittedChanges" type="button" @click="emit('showChanges')">Review file changes</button>
      </div>
      <div v-else-if="visibleCommits.length === 0" class="commits__state commits__state--empty">
        <span class="commits__empty-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24"><circle cx="10" cy="10" r="5.75" /><path d="m14.5 14.5 4 4" /></svg>
        </span>
        <strong>No matching commits</strong>
        <span>Try a different subject, SHA, or author.</span>
        <button type="button" @click="query = ''; focusSearch()">Clear search</button>
      </div>
      <ol v-else class="commit-list" aria-label="Commits">
        <li v-for="(commit, index) in visibleCommits" :key="commit.sha" class="commit-card" :class="{ 'commit-card--expanded': expanded.has(commit.sha) }">
          <button
            class="commit-card__toggle"
            type="button"
            :data-commit-index="index"
            :aria-expanded="expanded.has(commit.sha)"
            :aria-label="`${expanded.has(commit.sha) ? 'Collapse' : 'Expand'} commit ${commit.shortSha}: ${commit.subject || 'No subject'}`"
            @click="toggleCommit(commit.sha)"
          >
            <span class="commit-card__rail" aria-hidden="true">
              <span class="commit-card__node" :class="{ 'commit-card__node--merge': commit.parentCount > 1 }"></span>
            </span>
            <span class="commit-card__content">
              <span class="commit-card__heading">
                <span class="commit-card__subject" :title="commit.subject">{{ commit.subject || 'No subject' }}</span>
                <span v-if="commit.body" class="commit-card__body-dot" title="Has commit message body" aria-label="Has commit message body"></span>
                <code class="commit-card__sha" :title="commit.sha">{{ commit.shortSha }}</code>
                <svg class="commit-card__chevron" viewBox="0 0 12 12" aria-hidden="true"><path d="m3.25 4.5 2.75 2.75 2.75-2.75" /></svg>
              </span>
              <span class="commit-card__metadata">
                <span class="commit-card__author" :title="authorTitle(commit)">
                  <span class="commit-card__avatar" :style="{ background: authorColor(commit) }">{{ authorInitials(commit) }}</span>
                  <span>{{ authorName(commit) }}</span>
                </span>
                <span class="commit-card__separator"></span>
                <time v-if="commit.committedAt" class="commit-card__date" :datetime="isoDate(commit.committedAt)" :title="exactDate(commit.committedAt)">{{ relativeDate(commit.committedAt) }}</time>
                <span v-else class="commit-card__date">Date unavailable</span>
                <span v-if="commit.committedAt" class="commit-card__separator"></span>
                <span v-if="commit.committedAt" class="commit-card__exact-date">{{ exactDate(commit.committedAt) }}</span>
                <span v-if="commit.parentCount > 1" class="commit-card__merge" title="Merge commit">
                  <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 2.5v4a3 3 0 0 0 3 3h4M11 7.5v4" /><circle cx="3" cy="2.5" r="1.25" /><circle cx="11" cy="11.5" r="1.25" /></svg>
                  Merge
                </span>
                <span class="commit-card__verification" :class="`commit-card__verification--${commit.verification}`" :title="verificationLabel(commit)">
                  <svg v-if="commit.verification === 'verified'" viewBox="0 0 14 14" aria-hidden="true"><path d="M7 1.5 11.5 3v3.25c0 2.7-1.8 4.75-4.5 6.25-2.7-1.5-4.5-3.55-4.5-6.25V3L7 1.5Z" /><path d="m4.5 7 1.6 1.6L9.75 5" /></svg>
                  <svg v-else viewBox="0 0 14 14" aria-hidden="true"><circle cx="7" cy="7" r="5.25" /><path d="M7 4.25v3.25M7 9.75v.1" /></svg>
                  <span>{{ commit.verification === 'verified' ? 'Verified' : commit.verification === 'unsigned' ? 'Unsigned' : 'Unverified' }}</span>
                </span>
              </span>
            </span>
            <span v-if="commit.changedFiles || commit.additions || commit.deletions" class="commit-card__stats" :title="`${statLabel(commit)} · ${commit.additions} additions · ${commit.deletions} deletions`">
              <span>{{ statLabel(commit) }}</span>
              <span v-if="commit.additions" class="commit-card__additions">+{{ commit.additions }}</span>
              <span v-if="commit.deletions" class="commit-card__deletions">−{{ commit.deletions }}</span>
            </span>
          </button>

          <div v-if="expanded.has(commit.sha)" class="commit-card__details">
            <p v-if="commit.body" class="commit-card__message">{{ commit.body }}</p>
            <div class="commit-card__detail-grid">
              <div><span>Full SHA</span><code>{{ commit.sha }}</code></div>
              <div><span>Authored</span><time :datetime="isoDate(commit.authoredAt)" :title="exactDate(commit.authoredAt)">{{ exactDate(commit.authoredAt) }}</time></div>
              <div><span>Committed</span><time :datetime="isoDate(commit.committedAt)" :title="exactDate(commit.committedAt)">{{ exactDate(commit.committedAt) }}</time></div>
              <div><span>Files changed</span><strong>{{ statLabel(commit) }}</strong></div>
              <div v-if="commit.parentCount > 1"><span>Parents</span><strong>{{ commit.parentCount }} · merge commit</strong></div>
              <div v-if="commit.verificationSigner"><span>Signed by</span><strong>{{ commit.verificationSigner }}</strong></div>
            </div>
            <div v-if="commit.changedFiles || commit.additions || commit.deletions" class="commit-card__detail-stats">
              <span>Commit stats</span>
              <span class="commit-card__additions">+{{ commit.additions }} additions</span>
              <span class="commit-card__deletions">−{{ commit.deletions }} deletions</span>
              <button type="button" @click="emit('showChanges')">Review file changes</button>
            </div>
          </div>
        </li>
      </ol>
    </div>
  </section>
</template>

<style scoped>
.commits {
  display: flex;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  outline: none;
  background: var(--surface-content);
}

.commits__toolbar {
  display: flex;
  min-height: 56px;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 0 16px;
  border-bottom: 1px solid var(--border-subtle);
  background: rgba(16, 13, 24, 0.94);
}

.commits__title-group,
.commits__toolbar-actions,
.commits__overview,
.commits__overview-left,
.commits__source,
.commits__notice,
.commit-card__heading,
.commit-card__metadata,
.commit-card__author,
.commit-card__verification,
.commit-card__merge,
.commit-card__stats,
.commit-card__detail-stats {
  display: flex;
  align-items: center;
}

.commits__title-group { min-width: 0; gap: 9px; }
.commits__title-group > div { display: flex; min-width: 0; flex-direction: column; gap: 3px; }
.commits__title-group strong { font-size: 12px; font-weight: 600; color: var(--text-primary); }
.commits__title-group div > span { overflow: hidden; color: var(--text-muted); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.commits__title-icon { display: grid; width: 27px; height: 27px; flex: 0 0 auto; place-items: center; color: var(--primary); background: var(--primary-subtle); border: 1px solid var(--primary-border); border-radius: 8px; }
.commits__title-icon svg { width: 16px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.25; }
.commits__toolbar-actions { gap: 6px; }

.commit-search { display: flex; width: clamp(170px, 24vw, 275px); height: 29px; align-items: center; gap: 6px; padding: 0 5px 0 8px; color: var(--text-muted); background: var(--surface-input); border: 1px solid var(--border-subtle); border-radius: 7px; }
.commit-search:focus-within { border-color: var(--primary-border); box-shadow: 0 0 0 1px var(--primary-subtle); }
.commit-search > svg { width: 13px; flex: 0 0 auto; fill: none; stroke: currentColor; stroke-width: 1.25; }
.commit-search input { min-width: 0; flex: 1; padding: 0; color: var(--text-primary); background: transparent; border: 0; outline: 0; font: inherit; font-size: 11px; }
.commit-search input::placeholder { color: var(--text-muted); }
.commit-search kbd { color: var(--text-muted); font-family: inherit; font-size: 9px; }
.commit-search button { width: 19px; height: 19px; padding: 0; color: var(--text-secondary); background: transparent; border: 0; border-radius: 4px; font: 16px/17px inherit; }
.commit-search button:hover { color: var(--text-primary); background: var(--surface-hover); }
.commits__refresh { display: grid; width: 29px; height: 29px; padding: 0; place-items: center; color: var(--text-secondary); background: transparent; border: 1px solid transparent; border-radius: 7px; }
.commits__refresh:hover { color: var(--text-primary); background: var(--surface-hover); border-color: var(--border-subtle); }
.commits__refresh:disabled { opacity: .45; }
.commits__refresh:focus-visible,
.commit-search button:focus-visible,
.commit-card__detail-stats button:focus-visible,
.commits__state button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
.commits__refresh svg { width: 15px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.25; }
.commits__refresh:disabled svg { animation: spin .8s linear infinite; }

.commits__workspace { display: flex; min-height: 0; flex: 1; flex-direction: column; overflow: hidden; }
.commits__overview { min-height: 42px; flex: 0 0 auto; justify-content: space-between; gap: 10px; padding: 0 20px; border-bottom: 1px solid var(--border-subtle); background: rgba(25, 20, 31, 0.42); }
.commits__overview-left { min-width: 0; gap: 12px; }
.commits__source { min-width: 0; gap: 6px; overflow: hidden; color: var(--text-primary); font-size: 11px; font-weight: 550; text-overflow: ellipsis; white-space: nowrap; }
.commits__source svg { width: 14px; flex: 0 0 auto; fill: none; stroke: var(--primary); stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.25; }
.commits__comparison,
.commits__filtered-count { color: var(--text-muted); font-size: 10px; white-space: nowrap; }
.commits__filtered-count { font-variant-numeric: tabular-nums; }

.commits__notice { min-height: 38px; flex: 0 0 auto; gap: 8px; margin: 12px 20px 2px; padding: 8px 10px; color: var(--text-secondary); background: var(--primary-subtle); border: 1px solid var(--primary-border); border-radius: 7px; font-size: 10px; line-height: 1.4; }
.commits__notice--warning { color: var(--warning); background: var(--warning-subtle); border-color: var(--warning-border); }
.commits__notice svg { width: 14px; flex: 0 0 auto; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.25; }
.commits__notice span { min-width: 0; flex: 1; }
.commits__notice button,
.commits__state button,
.commit-card__detail-stats button { flex: 0 0 auto; padding: 4px 7px; color: var(--primary); background: transparent; border: 1px solid var(--primary-border); border-radius: 5px; font: inherit; font-size: 10px; }
.commits__notice button:hover,
.commits__state button:hover,
.commit-card__detail-stats button:hover { color: var(--primary-hover); background: var(--primary-subtle); }

.commit-list { min-height: 0; flex: 1; overflow-y: auto; overscroll-behavior: contain; margin: 0; padding: 14px 20px 34px; list-style: none; scrollbar-color: rgba(255, 255, 255, .18) transparent; scrollbar-width: thin; }
.commit-list::-webkit-scrollbar { width: 7px; }
.commit-list::-webkit-scrollbar-track { background: transparent; }
.commit-list::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, .18); border-radius: 999px; }
.commit-list::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, .28); }
.commit-card { position: relative; content-visibility: auto; contain-intrinsic-size: 72px; }
.commit-card::before { position: absolute; top: 0; bottom: 0; left: 20px; width: 1px; content: ''; background: var(--border-strong); }
.commit-card:first-child::before { top: 20px; }
.commit-card:last-child::before { bottom: calc(100% - 20px); }
.commit-card__toggle { position: relative; display: grid; width: 100%; min-height: 68px; align-items: center; grid-template-columns: 42px minmax(0, 1fr) auto; gap: 0; padding: 10px 10px 10px 0; color: var(--text-secondary); text-align: left; background: transparent; border: 1px solid transparent; border-radius: 8px; font: inherit; }
.commit-card__toggle:hover { color: var(--text-primary); background: rgba(255, 255, 255, .025); border-color: var(--border-subtle); }
.commit-card__toggle:focus-visible { z-index: 1; outline: 2px solid var(--focus-ring); outline-offset: -2px; }
.commit-card__rail { position: relative; z-index: 1; display: grid; width: 42px; height: 100%; place-items: center; }
.commit-card__node { display: block; width: 11px; height: 11px; background: var(--surface-content); border: 2px solid var(--border-strong); border-radius: 50%; transition: border-color 120ms ease, box-shadow 120ms ease; }
.commit-card__toggle:hover .commit-card__node,
.commit-card--expanded .commit-card__node { border-color: var(--primary); box-shadow: 0 0 0 3px var(--primary-subtle); }
.commit-card__node--merge { border-color: var(--primary); border-radius: 3px; transform: rotate(45deg); }
.commit-card__content { min-width: 0; }
.commit-card__heading { min-width: 0; gap: 7px; }
.commit-card__subject { min-width: 0; overflow: hidden; color: var(--text-primary); font-size: 12px; font-weight: 550; line-height: 1.35; text-overflow: ellipsis; white-space: nowrap; }
.commit-card__body-dot { width: 4px; height: 4px; flex: 0 0 auto; background: var(--primary); border-radius: 50%; }
.commit-card__sha { flex: 0 0 auto; padding: 2px 5px; color: var(--primary-hover); background: var(--primary-subtle); border: 1px solid var(--primary-border); border-radius: 4px; font: 10px/1.2 ui-monospace, SFMono-Regular, Consolas, monospace; }
.commit-card__chevron { width: 12px; flex: 0 0 auto; margin-left: auto; fill: none; stroke: var(--text-muted); stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.4; transition: transform 120ms ease; }
.commit-card--expanded .commit-card__chevron { transform: rotate(180deg); }
.commit-card__metadata { min-width: 0; gap: 8px; margin-top: 6px; overflow: hidden; font-size: 10px; }
.commit-card__author { min-width: 0; gap: 5px; overflow: hidden; color: var(--text-secondary); text-overflow: ellipsis; white-space: nowrap; }
.commit-card__author > span:last-child { overflow: hidden; text-overflow: ellipsis; }
.commit-card__avatar { display: grid; width: 17px; height: 17px; flex: 0 0 auto; place-items: center; color: var(--primary-foreground); border-radius: 50%; font-size: 7px; font-weight: 700; letter-spacing: -.02em; }
.commit-card__separator { width: 2px; height: 2px; flex: 0 0 auto; background: var(--text-muted); border-radius: 50%; }
.commit-card__date { flex: 0 0 auto; color: var(--text-muted); white-space: nowrap; }
.commit-card__exact-date { flex: 0 0 auto; color: var(--text-muted); opacity: .72; white-space: nowrap; }
.commit-card__merge,
.commit-card__verification { gap: 3px; flex: 0 0 auto; white-space: nowrap; }
.commit-card__merge { color: var(--text-muted); }
.commit-card__merge svg { width: 13px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.2; }
.commit-card__verification svg { width: 13px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.2; }
.commit-card__verification--verified { color: var(--success); }
.commit-card__verification--unverified { color: var(--danger); }
.commit-card__verification--unsigned,
.commit-card__verification--unknown { color: var(--text-muted); }
.commit-card__stats { gap: 7px; margin-left: 12px; color: var(--text-muted); font-size: 10px; white-space: nowrap; }
.commit-card__additions { color: var(--success); }
.commit-card__deletions { color: var(--danger); }

.commit-card__details { margin: -1px 10px 9px 42px; padding: 12px 14px; background: rgba(25, 20, 31, .68); border: 1px solid var(--border-subtle); border-radius: 7px; }
.commit-card__message { margin: 0 0 12px; color: var(--text-secondary); font-size: 11px; line-height: 1.55; white-space: pre-wrap; overflow-wrap: anywhere; }
.commit-card__detail-grid { display: grid; gap: 9px 18px; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); }
.commit-card__detail-grid > div { display: flex; min-width: 0; flex-direction: column; gap: 3px; }
.commit-card__detail-grid span { color: var(--text-muted); font-size: 9px; text-transform: uppercase; letter-spacing: .045em; }
.commit-card__detail-grid code,
.commit-card__detail-grid time,
.commit-card__detail-grid strong { min-width: 0; overflow: hidden; color: var(--text-secondary); font: 10px/1.4 inherit; text-overflow: ellipsis; white-space: nowrap; }
.commit-card__detail-grid code { color: var(--primary-hover); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; }
.commit-card__detail-stats { flex-wrap: wrap; gap: 9px; margin-top: 13px; padding-top: 10px; border-top: 1px solid var(--border-subtle); color: var(--text-muted); font-size: 10px; }
.commit-card__detail-stats > span:first-child { margin-right: auto; color: var(--text-secondary); font-weight: 550; }

.commits__state { display: flex; min-height: 0; flex: 1; align-items: center; justify-content: center; flex-direction: column; gap: 7px; padding: 26px; color: var(--text-muted); text-align: center; }
.commits__state strong { color: var(--text-primary); font-size: 13px; font-weight: 550; }
.commits__state > span:not(.commits__spinner):not(.commits__state-icon):not(.commits__empty-icon) { max-width: 390px; font-size: 11px; line-height: 1.5; }
.commits__state button { margin-top: 5px; }
.commits__state--error strong { color: var(--danger); }
.commits__state-icon,
.commits__empty-icon { display: grid; place-items: center; color: var(--danger); }
.commits__state-icon svg { width: 24px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.15; }
.commits__empty-icon { margin-bottom: 3px; color: var(--primary); }
.commits__empty-icon svg { width: 30px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.1; }
.commits__spinner { width: 18px; height: 18px; margin-bottom: 3px; border: 2px solid var(--border-strong); border-top-color: var(--primary); border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

@media (max-width: 760px) {
  .commits__toolbar { padding-inline: 12px; }
  .commit-list { padding-inline: 12px; }
  .commits__overview { padding-inline: 14px; }
  .commit-card__stats { display: none; }
  .commit-card__metadata { gap: 6px; }
  .commit-card__exact-date { display: none; }
  .commit-card__verification span { display: none; }
}
</style>
