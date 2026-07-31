<script setup lang="ts">
import { ExternalLink, Settings } from '@lucide/vue';
import { computed } from 'vue';
import { openPath } from '@tauri-apps/plugin-opener';
import AppButton from '../ui/AppButton.vue';
import type { Project, WorkItem } from '../../types/projects';
import { workItemKind, workItemMeta, workItemTitle } from '../../utils/workItems';
import RunAction from './RunAction.vue';

const props = defineProps<{
  project: Project;
  workItem: WorkItem;
  sidebarOpen: boolean;
}>();

const emit = defineEmits<{ settings: [section?: 'run' | 'ship']; refresh: [] }>();

const title = computed(() => workItemTitle(props.workItem));
const kind = computed(() => workItemKind(props.project, props.workItem));
const meta = computed(() => workItemMeta(props.workItem));
const statusLabel = computed(() =>
  props.workItem.status === 'mergeConflict' ? 'Merge Conflict' : props.workItem.status,
);

function openWorkItem() {
  const path = props.workItem.resolutionPath ?? props.workItem.worktreePath;
  if (path) void openPath(path);
}
</script>

<template>
  <header class="work-header" :class="{ 'work-header--sidebar-closed': !sidebarOpen }">
    <div class="work-header__primary">
      <div class="work-header__identity">
        <span class="work-header__dot" :style="{ background: project.color }"></span>
        <h1>{{ title }}</h1>
        <span class="status-pill" :class="`status-pill--${workItem.status}`">
          {{ statusLabel }}
        </span>
      </div>

      <div class="work-header__actions">
        <AppButton
          size="small"
          type="button"
          :disabled="!workItem.worktreePath && !workItem.resolutionPath"
          :title="workItem.resolutionPath ? 'Open the default-branch worktree to resolve the merge' : 'Open worktree'"
          @click="openWorkItem"
        >
          <ExternalLink aria-hidden="true" />
          <span>Open</span>
        </AppButton>
        <RunAction :project="project" :work-item="workItem" @settings="emit('settings', $event)" />
        <RunAction
          mode="ship"
          :project="project"
          :work-item="workItem"
          @settings="emit('settings', $event)"
          @refresh="emit('refresh')"
        />
        <AppButton
          class="work-header__settings"
          variant="ghost"
          size="icon"
          type="button"
          :aria-label="`Project settings for ${project.name}`"
          :title="`Project settings — ${project.name}`"
          @click="emit('settings')"
        >
          <Settings aria-hidden="true" />
        </AppButton>
      </div>
    </div>

    <div class="work-header__secondary">
      <span>{{ project.name }}</span>
      <span class="work-header__separator"></span>
      <span>{{ kind }}</span>
      <span class="work-header__separator"></span>
      <span :class="{ 'work-header__changes': workItem.status === 'working' }">{{ meta }}</span>
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

.status-pill--working {
  color: #e7b950;
  background: rgba(231, 185, 80, 0.1);
  border-color: rgba(231, 185, 80, 0.18);
}

.status-pill--ready {
  color: #79aeff;
  background: rgba(80, 145, 255, 0.11);
  border-color: rgba(80, 145, 255, 0.2);
}

.status-pill--shipped {
  color: #64cf8c;
  background: rgba(62, 190, 111, 0.1);
  border-color: rgba(62, 190, 111, 0.19);
}

.status-pill--mergeConflict {
  color: #ff8f8f;
  background: rgba(255, 85, 85, 0.1);
  border-color: rgba(255, 85, 85, 0.22);
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

.work-header__settings {
  width: 24px;
  height: 24px;
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

@media (max-width: 680px) {
  .work-header__actions > button {
    width: 26px;
    padding: 0;
    justify-content: center;
  }

  .work-header__actions > button span {
    display: none;
  }
}
</style>
