<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useRunner } from '../../composables/useRunner';
import type { Project, WorkItem } from '../../types/projects';
import type { ShippingAction } from '../../services/ship';
import { pullRequestSyncState } from '../../utils/workItems';
import ConfirmationDialog from '../ui/ConfirmationDialog.vue';

const props = defineProps<{ project: Project; workItem: WorkItem }>();
const emit = defineEmits<{ refresh: [] }>();
const { currentRun, error, shipWork, cancel } = useRunner();
const root = ref<HTMLElement>();
const menuOpen = ref(false);
const pendingAction = ref<ShippingAction | null>(null);
const refreshedRunId = ref<string | null>(null);

const active = computed(() => currentRun.value?.kind === 'ship' && currentRun.value.workItemId === props.workItem.id && ['running', 'stopping'].includes(currentRun.value.status));
const anotherRunActive = computed(() => !!currentRun.value && ['running', 'stopping'].includes(currentRun.value.status) && !active.value);
const syncState = computed(() => pullRequestSyncState(props.workItem));
const needsUpdate = computed(() => !!syncState.value && syncState.value !== 'synced');
const blocked = computed(() => !!props.workItem.pullRequest && !needsUpdate.value && ['checksPending', 'checksFailed', 'reviewRequired', 'draft'].includes(props.workItem.pullRequest.mergeState));
const defaultBranch = computed(() => props.project.defaultBranch ?? 'default branch');
const remoteName = computed(() => props.project.remoteName ?? 'remote');
const remoteHostDescription = computed(() => props.project.remoteHost ? ` on ${props.project.remoteHost}` : '');
const remoteContext = computed(() => props.project.remoteHost ? `${remoteName.value} · ${props.project.remoteHost}` : remoteName.value);
const isDefaultBranch = computed(() => props.workItem.branch === props.project.defaultBranch);
const genericAvailable = computed(() => !!props.project.remoteName && !!props.project.defaultBranch && !!props.workItem.branch);
const githubPullRequestAvailable = computed(() => !!props.project.githubRepository && !!props.workItem.branch && !isDefaultBranch.value && !!props.workItem.worktreePath);
const primaryLabel = computed(() => {
  if (active.value && currentRun.value?.status === 'stopping') return 'Stopping…';
  if (active.value) return 'Stop';
  const pullRequest = props.workItem.pullRequest;
  if (!pullRequest) {
    if (isDefaultBranch.value) return `Push ${defaultBranch.value}`;
    return githubPullRequestAvailable.value ? 'Create PR' : 'Push branch';
  }
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
const unavailable = computed(() => {
  if (!props.project.defaultBranch || !props.project.remoteName) return true;
  if (!props.workItem.pullRequest) {
    if (isDefaultBranch.value) return false;
    if (githubPullRequestAvailable.value) return false;
    return !genericAvailable.value;
  }
  if (!props.project.githubRepository) return true;
  if (!needsUpdate.value) return false;
  return !props.workItem.branch || !props.workItem.worktreePath;
});

const menuOptions = computed(() => {
  if (props.workItem.pullRequest || !genericAvailable.value || isDefaultBranch.value) return [];
  if (props.project.githubRepository) {
    if (!props.workItem.worktreePath) return [];
    return [{
      action: 'directToMain' as const,
      label: `Ship directly to ${defaultBranch.value}`,
      description: 'Pushes the resolved work directly to the default branch, bypassing pull-request review.',
    }];
  }
  const options: { action: ShippingAction; label: string; description: string }[] = [
    {
      action: 'pushBranch',
      label: `Push ${props.workItem.branch} to ${remoteName.value}`,
      description: 'Pushes this branch and sets its upstream without changing the default branch.',
    },
    {
      action: 'integrateToDefault',
      label: `Integrate and push to ${defaultBranch.value}`,
      description: `Brings ${defaultBranch.value} into this work, resolves conflicts when needed, then pushes ${defaultBranch.value}.`,
    },
  ];
  return options;
});

const confirmation = computed(() => {
  const action = pendingAction.value;
  if (!action) return null;
  const branch = props.workItem.branch ?? 'this work';
  if (action === 'pushDefault') {
    return {
      title: `Push ${defaultBranch.value}?`,
      description: `ShipYard will commit any local changes in ${defaultBranch.value} with your selected coding agent, then push ${defaultBranch.value} to ${remoteName.value}${remoteHostDescription.value}. It will never force-push or alter another branch.`,
      label: `Push ${defaultBranch.value}`,
    };
  }
  if (action === 'pushBranch') {
    return {
      title: `Push ${branch}?`,
      description: `ShipYard will commit local changes in ${branch} when needed, push it to ${remoteName.value}/${branch}${remoteHostDescription.value}, and set its upstream. The default branch is not changed.`,
      label: `Push ${branch}`,
    };
  }
  if (action === 'integrateToDefault') {
    return {
      title: `Integrate ${branch}?`,
      description: `ShipYard will combine ${branch} with the latest ${defaultBranch.value}, use your selected coding agent for conflicts when needed, and push the result to ${remoteName.value}/${defaultBranch.value}${remoteHostDescription.value}. No force-push or silent discard is used.`,
      label: `Integrate and push ${defaultBranch.value}`,
    };
  }
  return {
    title: `Ship directly to ${defaultBranch.value}?`,
    description: `ShipYard will bypass pull-request review and push the resolved ${branch} work to ${remoteName.value}/${defaultBranch.value}${remoteHostDescription.value}. Conflicts are isolated and surfaced before anything is pushed.`,
    label: `Ship to ${defaultBranch.value}`,
  };
});

async function primary() {
  if (active.value) return cancel();
  if (blocked.value || unavailable.value || anotherRunActive.value) return;
  if (!props.workItem.pullRequest) {
    if (isDefaultBranch.value) return requestConfirmation('pushDefault');
    if (githubPullRequestAvailable.value) return shipWork(props.project, props.workItem, 'createPullRequest');
    return requestConfirmation('pushBranch');
  }
  const action = needsUpdate.value ? 'updatePullRequest' : 'mergePullRequest';
  await shipWork(props.project, props.workItem, action);
}

function requestConfirmation(action: ShippingAction) {
  menuOpen.value = false;
  pendingAction.value = action;
}

async function confirmShipping() {
  const action = pendingAction.value;
  if (!action) return;
  pendingAction.value = null;
  await shipWork(props.project, props.workItem, action);
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
    <button class="ship-control__main" :class="{ 'ship-control__main--single': !menuOptions.length }" type="button" :disabled="anotherRunActive || blocked || unavailable" :title="unavailable ? (needsUpdate ? 'A checked-out local branch is required to update this pull request' : 'A configured Git remote and default branch are required to ship this work') : primaryLabel" @click="primary">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path v-if="active" d="M4.5 4.5h7v7h-7z"/><path v-else d="M2.5 9.5h11l-2 3h-7l-2-3Zm3-1V3.5l5 2.5-5 2.5Z"/></svg>
      <span>{{ primaryLabel }}</span>
    </button>
    <button v-if="menuOptions.length" class="ship-control__menu-button" type="button" aria-label="More shipping options" :aria-expanded="menuOpen" :disabled="anotherRunActive || unavailable" @click="menuOpen = !menuOpen">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4.5 6 3.5 3.5L11.5 6"/></svg>
    </button>
    <div v-if="menuOpen && menuOptions.length" class="ship-menu">
      <div class="ship-menu__context">
        <strong>ShipYard · remote workflow</strong>
        <small>{{ remoteContext }} · default {{ defaultBranch }}</small>
      </div>
      <button v-for="option in menuOptions" :key="option.action" type="button" @click="requestConfirmation(option.action)">
        <strong>{{ option.label }}</strong>
        <small>{{ option.description }}</small>
      </button>
      <p v-if="error">{{ error }}</p>
    </div>
    <p v-if="error && !menuOpen" class="ship-error" role="alert">{{ error }}</p>
  </div>
  <ConfirmationDialog
    v-if="confirmation"
    :title="confirmation.title"
    :description="confirmation.description"
    :confirm-label="confirmation.label"
    @cancel="pendingAction = null"
    @confirm="confirmShipping"
  />
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
.ship-menu button { display: flex; width: 100%; min-height: 30px; padding: 6px 8px; flex-direction: column; align-items: flex-start; gap: 2px; font: inherit; font-size: 11px; color: var(--text-primary); text-align: left; background: transparent; border: 0; border-radius: 5px; }
.ship-menu button:hover { background: var(--surface-hover); }
.ship-menu button strong { font-weight: 550; }
.ship-menu button small { font-size: 10px; line-height: 1.35; color: var(--text-secondary); }
.ship-menu__context { padding: 6px 8px 7px; border-bottom: 1px solid var(--border-subtle); }
.ship-menu__context strong { display: block; font-size: 10px; font-weight: 650; color: var(--primary-hover); }
.ship-menu__context small { display: block; margin-top: 2px; overflow: hidden; font-size: 10px; line-height: 1.35; color: var(--text-secondary); text-overflow: ellipsis; white-space: nowrap; }
.ship-menu p { margin: 0; padding: 7px; font-size: 10px; color: var(--danger); }
.ship-error { position: absolute; z-index: 7; top: 29px; right: 0; width: 220px; margin: 0; padding: 7px 8px; font-size: 10px; line-height: 1.35; color: var(--danger); background: var(--danger-subtle); border: 1px solid var(--danger-border); border-radius: 6px; box-shadow: var(--shadow-elevated); }

@media (max-width: 680px) {
  .ship-control > button {
    width: 26px;
    padding: 0;
    justify-content: center;
  }

  .ship-control > button span {
    display: none;
  }

  .ship-control__menu-button {
    width: 18px !important;
  }
}
</style>
