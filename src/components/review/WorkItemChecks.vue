<script setup lang="ts">
import { openUrl } from '@tauri-apps/plugin-opener';
import {
  AlertCircle,
  CheckCircle2,
  Clock3,
  ExternalLink,
  GitPullRequest,
  Info,
  LoaderCircle,
  RefreshCw,
  ShieldCheck,
  XCircle,
} from '@lucide/vue';
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { getPullRequestChecks } from '../../services/github';
import type { PullRequestCheck, PullRequestChecks } from '../../types/github';
import type { Project, WorkItem } from '../../types/projects';
import {
  checkOutcomeLabel,
  checkTone,
  formatCheckDuration,
  formatTimestamp,
} from '../../utils/github';

const props = defineProps<{
  project: Project;
  workItem: WorkItem;
}>();

const loading = ref(false);
const error = ref<string | null>(null);
const checks = ref<PullRequestChecks | null>(null);
let requestVersion = 0;

const request = computed(() => {
  const pullRequest = props.workItem.pullRequest;
  if (!pullRequest || !props.project.githubRepository) return null;
  return { repository: props.project.githubRepository, number: pullRequest.number };
});

const readiness = computed(() => {
  const pullRequest = props.workItem.pullRequest;
  if (!pullRequest) {
    return {
      tone: 'neutral',
      title: 'Checks are only available for pull requests',
      description: 'Create a pull request to see GitHub checks and merge readiness.',
    };
  }
  if (pullRequest.draft || pullRequest.mergeState === 'draft') {
    return {
      tone: 'neutral',
      title: 'Draft pull request',
      description: 'This pull request is not ready to merge until it is marked ready on GitHub.',
    };
  }
  if (pullRequest.mergeState === 'conflicting') {
    return {
      tone: 'danger',
      title: 'Resolve merge conflicts',
      description: 'GitHub reports that this pull request cannot merge cleanly yet.',
    };
  }
  if (checks.value?.overallState === 'failure' || pullRequest.mergeState === 'checksFailed') {
    return {
      tone: 'danger',
      title: 'Checks need attention',
      description: 'At least one visible check has failed or requires action before merging.',
    };
  }
  if (checks.value?.overallState === 'pending' || pullRequest.mergeState === 'checksPending') {
    return {
      tone: 'pending',
      title: 'Waiting for checks',
      description: 'GitHub is still running the checks for this pull request.',
    };
  }
  if (pullRequest.mergeState === 'reviewRequired') {
    return {
      tone: 'pending',
      title: 'Review required',
      description: 'Checks are not the blocker currently reported by GitHub; an approval or other review rule is still outstanding.',
    };
  }
  if (pullRequest.mergeState === 'ready' && checks.value?.overallState !== 'unknown') {
    return {
      tone: 'success',
      title: 'Ready to merge',
      description: 'GitHub reports no known check, review, or mergeability blocker for this pull request.',
    };
  }
  if (pullRequest.mergeState === 'ready' && checks.value?.overallState === 'unknown') {
    return {
      tone: 'neutral',
      title: 'Mergeable, but checks are not visible',
      description: 'GitHub reports this pull request can merge cleanly, but Shipyard cannot confirm required checks from the current response. Verify required checks on GitHub before merging.',
    };
  }
  return {
    tone: 'neutral',
    title: 'Merge readiness is not fully known',
    description: 'GitHub has not published enough check information for Shipyard to make a readiness claim.',
  };
});

const summaryLabel = computed(() => {
  const value = checks.value;
  if (!value) return '';
  if (value.total === 0) return 'No checks reported';
  if (value.overallState === 'success') return `${value.passed} check${value.passed === 1 ? '' : 's'} passed`;
  if (value.overallState === 'failure') return `${value.failed} check${value.failed === 1 ? '' : 's'} need attention`;
  if (value.overallState === 'pending') return `${value.pending} check${value.pending === 1 ? '' : 's'} in progress`;
  return `${value.total} check${value.total === 1 ? '' : 's'} reported`;
});

function message(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}

async function load() {
  const currentRequest = request.value;
  const version = ++requestVersion;
  if (!currentRequest) {
    checks.value = null;
    error.value = 'This pull request is not connected to a GitHub repository.';
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    const result = await getPullRequestChecks(currentRequest);
    if (version === requestVersion) checks.value = result;
  } catch (loadError) {
    if (version === requestVersion) error.value = message(loadError);
  } finally {
    if (version === requestVersion) loading.value = false;
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

function checkAriaLabel(check: PullRequestCheck) {
  return `${check.name}: ${checkOutcomeLabel(check)}`;
}

watch(
  () => `${props.project.id}:${props.workItem.id}:${props.workItem.pullRequest?.number ?? 'local'}`,
  () => void load(),
  { immediate: true },
);

onBeforeUnmount(() => {
  requestVersion += 1;
});
</script>

<template>
  <section class="checks" aria-labelledby="checks-title">
    <header class="checks__toolbar">
      <div>
        <span class="checks__eyebrow"><ShieldCheck aria-hidden="true" /> GitHub integration</span>
        <h2 id="checks-title">Checks</h2>
        <p>Pre-merge signals for pull request #{{ workItem.pullRequest?.number }}</p>
      </div>
      <div class="checks__toolbar-actions">
        <button
          type="button"
          class="checks__icon-button"
          :aria-busy="loading"
          aria-label="Refresh checks"
          title="Refresh checks"
          @click="load"
        >
          <RefreshCw :class="{ 'is-spinning': loading }" aria-hidden="true" />
        </button>
        <button
          v-if="workItem.pullRequest"
          type="button"
          class="checks__open-button"
          @click="openExternal(workItem.pullRequest.url)"
        >
          <GitPullRequest aria-hidden="true" /> Open PR
        </button>
      </div>
    </header>

    <div v-if="loading" class="checks__state" aria-live="polite">
      <span class="checks__spinner"></span>
      <strong>Loading GitHub checks…</strong>
      <span>Fetching check runs and workflow status for this commit.</span>
      <div class="checks__skeletons" aria-hidden="true">
        <i v-for="index in 3" :key="index"></i>
      </div>
    </div>

    <div v-else-if="error" class="checks__state checks__state--error" role="alert">
      <AlertCircle aria-hidden="true" />
      <strong>Couldn’t load checks</strong>
      <span>{{ error }}</span>
      <button type="button" @click="load">Try again</button>
    </div>

    <div v-else-if="checks" class="checks__content">
      <section class="checks__readiness" :class="`checks__readiness--${readiness.tone}`" aria-live="polite">
        <div class="checks__readiness-icon">
          <CheckCircle2 v-if="readiness.tone === 'success'" aria-hidden="true" />
          <XCircle v-else-if="readiness.tone === 'danger'" aria-hidden="true" />
          <LoaderCircle v-else-if="readiness.tone === 'pending'" aria-hidden="true" />
          <Info v-else aria-hidden="true" />
        </div>
        <div class="checks__readiness-copy">
          <span>Merge readiness</span>
          <strong>{{ readiness.title }}</strong>
          <p>{{ readiness.description }}</p>
        </div>
        <span class="checks__summary-pill">{{ summaryLabel }}</span>
      </section>

      <div class="checks__stats" aria-label="Checks summary">
        <div>
          <strong>{{ checks.total }}</strong>
          <span>total</span>
        </div>
        <div class="checks__stats--success">
          <strong>{{ checks.passed }}</strong>
          <span>passed</span>
        </div>
        <div class="checks__stats--pending">
          <strong>{{ checks.pending }}</strong>
          <span>pending</span>
        </div>
        <div class="checks__stats--danger">
          <strong>{{ checks.failed }}</strong>
          <span>attention</span>
        </div>
        <time v-if="checks.lastUpdatedAt" :datetime="checks.lastUpdatedAt" class="checks__updated">
          Updated {{ formatTimestamp(checks.lastUpdatedAt) }}
        </time>
      </div>

      <div v-if="checks.checks.length === 0" class="checks__empty">
        <Clock3 aria-hidden="true" />
        <strong>No checks reported</strong>
        <p>GitHub has not published a check suite for this pull request yet. Branch protection may still require checks that are not visible to this token.</p>
        <button type="button" @click="openExternal(workItem.pullRequest?.url ?? null)">View pull request on GitHub <ExternalLink aria-hidden="true" /></button>
      </div>

      <section v-else class="checks__list" aria-label="Individual checks">
        <div class="checks__list-heading">
          <span>Check runs</span>
          <span>{{ checks.checks.length }} reported</span>
        </div>
        <article
          v-for="check in checks.checks"
          :key="check.id"
          class="check-row"
          :class="`check-row--${checkTone(check)}`"
          :aria-label="checkAriaLabel(check)"
        >
          <div class="check-row__status">
            <CheckCircle2 v-if="checkTone(check) === 'success'" aria-hidden="true" />
            <XCircle v-else-if="checkTone(check) === 'danger'" aria-hidden="true" />
            <LoaderCircle v-else-if="checkTone(check) === 'pending'" class="is-spinning" aria-hidden="true" />
            <Info v-else aria-hidden="true" />
          </div>
          <div class="check-row__identity">
            <strong :title="check.name">{{ check.name }}</strong>
            <span v-if="check.workflowName">{{ check.workflowName }}</span>
          </div>
          <div class="check-row__result">
            <strong>{{ checkOutcomeLabel(check) }}</strong>
            <span>{{ check.status.replace(/_/g, ' ').toLowerCase() }}</span>
          </div>
          <div class="check-row__timing">
            <span><Clock3 aria-hidden="true" /> {{ formatCheckDuration(check) }}</span>
            <time v-if="check.completedAt" :datetime="check.completedAt">{{ formatTimestamp(check.completedAt) }}</time>
          </div>
          <button v-if="check.url" type="button" class="check-row__details" @click="openExternal(check.url)">
            Details <ExternalLink aria-hidden="true" />
          </button>
        </article>
      </section>
    </div>
  </section>
</template>

<style scoped>
.checks { display: flex; width: 100%; min-width: 0; min-height: 0; flex: 1; flex-direction: column; overflow: hidden; background: var(--surface-content); }
.checks__toolbar { display: flex; flex: 0 0 auto; align-items: center; justify-content: space-between; gap: 18px; min-height: 78px; padding: 17px 18px 15px; border-bottom: 1px solid var(--border-subtle); background: rgba(16, 13, 24, .82); }
.checks__toolbar h2 { margin: 3px 0 0; font-size: 17px; font-weight: 560; letter-spacing: -.01em; }
.checks__toolbar p { margin: 4px 0 0; color: var(--text-secondary); font-size: 11px; }
.checks__eyebrow { display: flex; align-items: center; gap: 5px; color: var(--primary-hover); font-size: 9px; font-weight: 650; letter-spacing: .08em; text-transform: uppercase; }
.checks__eyebrow svg { width: 12px; height: 12px; }
.checks__toolbar-actions { display: flex; align-items: center; gap: 6px; }
.checks__icon-button, .checks__open-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 29px; color: var(--text-secondary); background: var(--surface-subtle); border: 1px solid var(--border-subtle); border-radius: 7px; font: inherit; font-size: 11px; }
.checks__icon-button { width: 29px; padding: 0; }
.checks__open-button { padding: 0 9px; }
.checks__icon-button:hover, .checks__open-button:hover { color: var(--text-primary); background: var(--surface-hover); border-color: var(--border-strong); }
.checks__icon-button:focus-visible, .checks__open-button:focus-visible, .checks__state button:focus-visible, .checks__empty button:focus-visible, .check-row button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
.checks__icon-button svg, .checks__open-button svg { width: 14px; height: 14px; }
.checks__content { min-height: 0; flex: 1; overflow-y: auto; padding: 18px; scrollbar-width: thin; }
.checks__readiness { display: flex; align-items: center; gap: 12px; min-height: 78px; padding: 13px 14px; border: 1px solid var(--border-subtle); border-radius: 10px; }
.checks__readiness--success { border-color: var(--success-border); background: var(--success-subtle); }
.checks__readiness--danger { border-color: var(--danger-border); background: var(--danger-subtle); }
.checks__readiness--pending { border-color: var(--warning-border); background: var(--warning-subtle); }
.checks__readiness--neutral { background: var(--surface-subtle); }
.checks__readiness-icon { display: grid; width: 34px; height: 34px; flex: 0 0 auto; place-items: center; border-radius: 9px; background: rgba(255,255,255,.05); }
.checks__readiness-icon svg { width: 20px; height: 20px; }
.checks__readiness--success .checks__readiness-icon { color: var(--success); }
.checks__readiness--danger .checks__readiness-icon { color: var(--danger); }
.checks__readiness--pending .checks__readiness-icon { color: var(--warning); }
.checks__readiness--neutral .checks__readiness-icon { color: var(--text-secondary); }
.checks__readiness-copy { min-width: 0; flex: 1; }
.checks__readiness-copy > span { display: block; margin-bottom: 3px; color: var(--text-secondary); font-size: 10px; text-transform: uppercase; letter-spacing: .05em; }
.checks__readiness-copy strong { display: block; font-size: 13px; font-weight: 600; }
.checks__readiness-copy p { margin: 4px 0 0; color: var(--text-secondary); font-size: 11px; line-height: 1.45; }
.checks__summary-pill { flex: 0 0 auto; padding: 5px 8px; color: var(--text-secondary); background: rgba(255,255,255,.055); border-radius: 999px; font-size: 10px; white-space: nowrap; }
.checks__stats { display: flex; align-items: center; gap: 22px; min-height: 63px; padding: 11px 3px 10px; border-bottom: 1px solid var(--border-subtle); }
.checks__stats > div { display: flex; flex-direction: column; gap: 2px; }
.checks__stats strong { font-size: 16px; font-weight: 550; font-variant-numeric: tabular-nums; }
.checks__stats span { color: var(--text-muted); font-size: 9px; text-transform: uppercase; letter-spacing: .05em; }
.checks__stats--success strong { color: var(--success); }
.checks__stats--pending strong { color: var(--warning); }
.checks__stats--danger strong { color: var(--danger); }
.checks__updated { margin-left: auto; color: var(--text-muted); font-size: 10px; white-space: nowrap; }
.checks__list { margin-top: 17px; border: 1px solid var(--border-subtle); border-radius: 9px; overflow: hidden; }
.checks__list-heading { display: flex; align-items: center; justify-content: space-between; min-height: 34px; padding: 0 12px; color: var(--text-muted); background: var(--surface-subtle); border-bottom: 1px solid var(--border-subtle); font-size: 10px; text-transform: uppercase; letter-spacing: .05em; }
.check-row { display: grid; grid-template-columns: 24px minmax(150px, 1fr) minmax(92px, .65fr) minmax(120px, .8fr) auto; gap: 10px; align-items: center; min-height: 58px; padding: 8px 11px; border-bottom: 1px solid var(--border-subtle); }
.check-row:last-child { border-bottom: 0; }
.check-row:hover { background: var(--surface-hover); }
.check-row__status { display: grid; place-items: center; }
.check-row__status svg { width: 17px; height: 17px; }
.check-row--success .check-row__status { color: var(--success); }
.check-row--danger .check-row__status { color: var(--danger); }
.check-row--pending .check-row__status { color: var(--warning); }
.check-row--neutral .check-row__status { color: var(--text-muted); }
.check-row__identity, .check-row__result, .check-row__timing { display: flex; min-width: 0; flex-direction: column; gap: 3px; }
.check-row__identity strong, .check-row__result strong { overflow: hidden; font-size: 11px; font-weight: 550; text-overflow: ellipsis; white-space: nowrap; }
.check-row__identity span, .check-row__result span, .check-row__timing time { overflow: hidden; color: var(--text-muted); font-size: 9px; text-overflow: ellipsis; white-space: nowrap; }
.check-row__result strong { color: var(--text-secondary); }
.check-row--success .check-row__result strong { color: var(--success); }
.check-row--danger .check-row__result strong { color: var(--danger); }
.check-row--pending .check-row__result strong { color: var(--warning); }
.check-row__timing span { display: flex; align-items: center; gap: 4px; color: var(--text-secondary); font-size: 10px; white-space: nowrap; }
.check-row__timing svg { width: 12px; height: 12px; color: var(--text-muted); }
.check-row__details { display: inline-flex; align-items: center; gap: 4px; height: 25px; padding: 0 7px; color: var(--text-secondary); background: transparent; border: 1px solid var(--border-subtle); border-radius: 5px; font: inherit; font-size: 10px; white-space: nowrap; }
.check-row__details:hover { color: var(--text-primary); border-color: var(--primary-border); background: var(--primary-subtle); }
.check-row__details svg { width: 11px; height: 11px; }
.checks__empty, .checks__state { display: flex; align-items: center; justify-content: center; min-height: 250px; flex: 1; flex-direction: column; gap: 8px; color: var(--text-muted); text-align: center; }
.checks__empty { min-height: 230px; padding: 32px 20px; }
.checks__empty > svg { width: 24px; height: 24px; margin-bottom: 2px; color: var(--text-secondary); }
.checks__empty strong, .checks__state strong { color: var(--text-primary); font-size: 13px; font-weight: 550; }
.checks__empty p, .checks__state > span:not(.checks__spinner) { max-width: 430px; margin: 0; color: var(--text-secondary); font-size: 11px; line-height: 1.5; }
.checks__empty button, .checks__state button { display: inline-flex; align-items: center; gap: 5px; margin-top: 5px; padding: 6px 9px; color: var(--primary); background: var(--primary-subtle); border: 1px solid var(--primary-border); border-radius: 6px; font: inherit; font-size: 10px; }
.checks__empty button svg { width: 12px; height: 12px; }
.checks__state--error > svg { width: 25px; height: 25px; color: var(--danger); }
.checks__state--error strong { color: var(--danger); }
.checks__spinner { width: 18px; height: 18px; border: 2px solid var(--border-strong); border-top-color: var(--primary); border-radius: 50%; animation: spin .8s linear infinite; }
.checks__skeletons { display: flex; width: min(360px, 80%); flex-direction: column; gap: 6px; margin-top: 8px; }
.checks__skeletons i { display: block; height: 34px; background: linear-gradient(90deg, var(--surface-subtle), rgba(255,255,255,.08), var(--surface-subtle)); background-size: 200% 100%; border-radius: 6px; animation: shimmer 1.4s ease-in-out infinite; }
.is-spinning { animation: spin .85s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }
@media (max-width: 820px) { .check-row { grid-template-columns: 24px minmax(0, 1fr) auto; } .check-row__result, .check-row__timing { display: none; } .checks__updated { display: none; } }
@media (prefers-reduced-motion: reduce) { .is-spinning, .checks__spinner, .checks__skeletons i { animation: none; } }
</style>
