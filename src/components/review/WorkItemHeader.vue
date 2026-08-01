<script setup lang="ts">
import { computed } from 'vue';
import type { Project, WorkItem } from '../../types/projects';
import { pullRequestSyncLabel, pullRequestSyncState, workItemKind, workItemMeta, workItemTitle } from '../../utils/workItems';
import OpenAction from './OpenAction.vue';
import RunAction from './RunAction.vue';
import ShipAction from './ShipAction.vue';

const props = defineProps<{
  project: Project;
  workItem: WorkItem;
  sidebarOpen: boolean;
}>();

const emit = defineEmits<{ settings: [section: 'open' | 'run']; refresh: [] }>();

const title = computed(() => workItemTitle(props.workItem));
const kind = computed(() => workItemKind(props.project, props.workItem));
const meta = computed(() => workItemMeta(props.workItem));
const syncState = computed(() => pullRequestSyncState(props.workItem));
const syncLabel = computed(() => pullRequestSyncLabel(props.workItem));
const statusLabel = computed(() => {
  if (syncLabel.value) return syncLabel.value;
  const state = props.workItem.pullRequest?.mergeState;
  if (!state) return 'Local Work';
  return ({ ready: 'Ready to merge', checksPending: 'Checks running', checksFailed: 'Checks failed', reviewRequired: 'Review required', conflicting: 'Resolving needed', draft: 'Draft PR' } as const)[state];
});
const statusClass = computed(() => {
  if (syncState.value && syncState.value !== 'synced') return `status-pill--${syncState.value}`;
  return props.workItem.pullRequest ? `status-pill--${props.workItem.pullRequest.mergeState}` : 'status-pill--local';
});
</script>

<template>
  <header class="work-header" :class="{ 'work-header--sidebar-closed': !sidebarOpen }">
    <div class="work-header__primary">
      <div class="work-header__identity">
        <span class="work-header__dot" :style="{ background: project.color }"></span>
        <h1>{{ title }}</h1>
      </div>

      <div class="work-header__actions">
        <OpenAction :project="project" :work-item="workItem" @settings="emit('settings', 'open')" />
        <RunAction :project="project" :work-item="workItem" @settings="emit('settings', $event)" />
        <ShipAction :project="project" :work-item="workItem" @refresh="emit('refresh')" />
      </div>
    </div>

    <div class="work-header__secondary">
      <span>{{ project.name }}</span>
      <span class="work-header__separator"></span>
      <span>{{ kind }}</span>
      <span class="work-header__separator"></span>
      <span v-if="workItem.pullRequest">#{{ workItem.pullRequest.number }}</span>
      <span v-else :class="{ 'work-header__changes': workItem.status === 'working' }">{{ meta }}</span>
      <span class="work-header__separator"></span>
      <span class="status-pill" :class="statusClass">{{ statusLabel }}</span>
    </div>
  </header>
</template>

<style scoped>
.work-header {
  --header-inset: 16px;

  flex: 0 0 auto;
  height: 70px;
  border-bottom: 1px solid var(--border-subtle);
}

.work-header--sidebar-closed {
  --header-inset: 116px;
}

.work-header__primary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-width: 0;
  height: 32px;
  padding: 0 12px 0 var(--header-inset);
  transition: padding-left 180ms cubic-bezier(0.2, 0.75, 0.25, 1);
}

.work-header__identity {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  pointer-events: none;
}

.work-header__dot {
  flex: 0 0 auto;
  width: 9px;
  height: 9px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.24);
}

.work-header h1 {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  font-size: 13px;
  font-weight: 550;
  line-height: 1;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-pill {
  flex: 0 0 auto;
  height: 18px;
  padding: 0 7px;
  font-size: 9px;
  font-weight: 600;
  line-height: 18px;
  text-transform: uppercase;
  letter-spacing: 0.045em;
  border: 1px solid transparent;
  border-radius: 9px;
}

.status-pill--local {
  color: var(--warning);
  background: var(--warning-subtle);
  border-color: var(--warning-border);
}

.status-pill--localChanges,
.status-pill--localAhead,
.status-pill--remoteAhead {
  color: var(--warning);
  background: var(--warning-subtle);
  border-color: var(--warning-border);
}

.status-pill--ready,
.status-pill--checksPending,
.status-pill--reviewRequired,
.status-pill--draft {
  color: var(--primary-hover);
  background: var(--primary-subtle);
  border-color: var(--primary-border);
}

.status-pill--shipped {
  color: var(--success);
  background: var(--success-subtle);
  border-color: var(--success-border);
}

.status-pill--conflicting,
.status-pill--checksFailed,
.status-pill--diverged {
  color: var(--danger);
  background: var(--danger-subtle);
  border-color: var(--danger-border);
}

.work-header__actions {
  position: relative;
  z-index: 4;
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 5px;
  margin-left: 12px;
}

.work-header__secondary {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  height: 37px;
  padding: 0 16px;
  overflow: hidden;
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.work-header__separator {
  width: 2px;
  height: 2px;
  background: rgba(255, 255, 255, 0.28);
  border-radius: 50%;
}

.work-header__changes {
  font-variant-numeric: tabular-nums;
}
</style>
