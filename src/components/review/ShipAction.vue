<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useRunner } from '../../composables/useRunner';
import type { Project, WorkItem } from '../../types/projects';
import { pullRequestSyncState } from '../../utils/workItems';

const props = defineProps<{ project: Project; workItem: WorkItem }>();
const emit = defineEmits<{ refresh: [] }>();
const { currentRun, error, shipWork, cancel } = useRunner();
const root = ref<HTMLElement>();
const menuOpen = ref(false);
const refreshedRunId = ref<string | null>(null);

const active = computed(() => currentRun.value?.kind === 'ship' && currentRun.value.workItemId === props.workItem.id && ['running', 'stopping'].includes(currentRun.value.status));
const anotherRunActive = computed(() => !!currentRun.value && ['running', 'stopping'].includes(currentRun.value.status) && !active.value);
const syncState = computed(() => pullRequestSyncState(props.workItem));
const needsUpdate = computed(() => !!syncState.value && syncState.value !== 'synced');
const blocked = computed(() => !!props.workItem.pullRequest && !needsUpdate.value && ['checksPending', 'checksFailed', 'reviewRequired', 'draft'].includes(props.workItem.pullRequest.mergeState));
const primaryLabel = computed(() => {
  if (active.value && currentRun.value?.status === 'stopping') return 'Stopping…';
  if (active.value) return 'Stop';
  const pullRequest = props.workItem.pullRequest;
  if (!pullRequest) return 'Create PR';
  if (syncState.value === 'remoteAhead') return 'Sync checkout';
  if (syncState.value === 'diverged') return 'Reconcile PR';
  if (needsUpdate.value) return 'Update PR';
  if (pullRequest.mergeState === 'conflicting') return 'Resolve & merge';
  if (pullRequest.mergeState === 'checksPending') return 'Waiting for checks';
  if (pullRequest.mergeState === 'checksFailed') return 'Checks failed';
  if (pullRequest.mergeState === 'reviewRequired') return 'Review required';
  if (pullRequest.mergeState === 'draft') return 'Draft PR';
  return 'Merge PR';
});
const unavailable = computed(() => !props.workItem.branch || !props.project.defaultBranch || !props.project.githubRepository || ((!props.workItem.pullRequest || needsUpdate.value) && !props.workItem.worktreePath));

async function primary() {
  if (active.value) return cancel();
  if (blocked.value || unavailable.value || anotherRunActive.value) return;
  const action = !props.workItem.pullRequest
    ? 'createPullRequest'
    : needsUpdate.value
      ? 'updatePullRequest'
      : 'mergePullRequest';
  await shipWork(props.project, props.workItem, action);
}

async function directToMain() {
  menuOpen.value = false;
  if (!window.confirm(`Ship “${props.workItem.branch}” directly to ${props.project.defaultBranch}? This bypasses pull-request review.`)) return;
  await shipWork(props.project, props.workItem, 'directToMain');
}

function closeMenu(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node)) menuOpen.value = false;
}

watch(() => currentRun.value, (state) => {
  if (state?.kind === 'ship' && state.workItemId === props.workItem.id && !['running', 'stopping'].includes(state.status) && refreshedRunId.value !== state.runId) {
    refreshedRunId.value = state.runId;
    emit('refresh');
  }
});
onMounted(() => document.addEventListener('pointerdown', closeMenu));
onBeforeUnmount(() => document.removeEventListener('pointerdown', closeMenu));
</script>

<template>
  <div ref="root" class="ship-control">
    <button class="ship-control__main" :class="{ 'ship-control__main--single': workItem.pullRequest }" type="button" :disabled="anotherRunActive || blocked || unavailable" :title="unavailable ? (needsUpdate ? 'Check out this branch before updating its pull request' : 'Local shipping requires a checked-out branch connected to GitHub') : primaryLabel" @click="primary">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path v-if="active" d="M4.5 4.5h7v7h-7z"/><path v-else d="M2.5 9.5h11l-2 3h-7l-2-3Zm3-1V3.5l5 2.5-5 2.5Z"/></svg>
      <span>{{ primaryLabel }}</span>
    </button>
    <button v-if="!workItem.pullRequest" class="ship-control__menu-button" type="button" aria-label="More shipping options" :aria-expanded="menuOpen" :disabled="anotherRunActive || unavailable" @click="menuOpen = !menuOpen">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4.5 6 3.5 3.5L11.5 6"/></svg>
    </button>
    <div v-if="menuOpen && !workItem.pullRequest" class="ship-menu">
      <button type="button" @click="directToMain">Ship directly to {{ project.defaultBranch }}</button>
      <p v-if="error">{{ error }}</p>
    </div>
  </div>
</template>

<style scoped>
.ship-control { position: relative; display: flex; }
.ship-control > button { display: flex; align-items: center; gap: 5px; height: 24px; padding: 0 8px; font: inherit; font-size: 11px; font-weight: 600; color: var(--primary-foreground); background: var(--primary); border: 1px solid var(--primary); }
.ship-control > button:hover:not(:disabled) { background: var(--primary-hover); border-color: var(--primary-hover); }
.ship-control > button:focus-visible, .ship-menu button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
.ship-control > button:disabled { opacity: .45; }
.ship-control svg { width: 13px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.35; }
.ship-control__main { max-width: 140px; border-radius: 6px 0 0 6px; }
.ship-control__main--single { border-radius: 6px; }
.ship-control__menu-button { width: 22px; justify-content: center; padding: 0 !important; margin-left: -1px; border-radius: 0 6px 6px 0; }
.ship-control__menu-button svg { width: 10px; }
.ship-menu { position: absolute; z-index: 7; top: 29px; right: 0; width: 210px; padding: 5px; background: var(--surface-elevated); border: 1px solid var(--border-strong); border-radius: 8px; box-shadow: var(--shadow-elevated); }
.ship-menu button { width: 100%; min-height: 30px; padding: 5px 8px; font: inherit; font-size: 11px; color: var(--text-primary); text-align: left; background: transparent; border: 0; border-radius: 5px; }
.ship-menu button:hover { background: var(--surface-hover); }
.ship-menu p { margin: 0; padding: 7px; font-size: 10px; color: var(--danger); }
</style>
