<script setup lang="ts">
import { ref } from 'vue';
import type { Project, WorkItem } from '../../types/projects';
import RunConsole from './RunConsole.vue';
import WorkItemHeader from './WorkItemHeader.vue';

defineProps<{
  project: Project | null;
  workItem: WorkItem | null;
  sidebarOpen: boolean;
}>();

const emit = defineEmits<{
  settings: [project: Project, section: 'open' | 'run'];
  refresh: [projectId: string];
}>();

type ReviewTab = 'changes' | 'commits';

const activeTab = ref<ReviewTab>('changes');
</script>

<template>
  <section v-if="project && workItem" class="work-panel">
    <WorkItemHeader
      :project="project"
      :work-item="workItem"
      :sidebar-open="sidebarOpen"
      @settings="emit('settings', project, $event)"
      @refresh="emit('refresh', project.id)"
    />

    <nav class="review-tabs" aria-label="Work item details">
      <button
        type="button"
        :class="{ 'review-tabs__tab--active': activeTab === 'changes' }"
        :aria-current="activeTab === 'changes' ? 'page' : undefined"
        @click="activeTab = 'changes'"
      >
        Changes
        <span>{{ workItem.changedFiles }}</span>
      </button>
      <button
        type="button"
        :class="{ 'review-tabs__tab--active': activeTab === 'commits' }"
        :aria-current="activeTab === 'commits' ? 'page' : undefined"
        @click="activeTab = 'commits'"
      >
        Commits
      </button>
    </nav>

    <div class="work-panel__body">
      <div class="work-panel__placeholder">
        <svg v-if="activeTab === 'changes'" viewBox="0 0 20 20" aria-hidden="true">
          <path d="M5.25 3.25h6l3.5 3.5v10H5.25v-13Zm6 0v3.5h3.5M7.75 10h4.5m-4.5 3h4.5" />
        </svg>
        <svg v-else viewBox="0 0 20 20" aria-hidden="true">
          <path d="M10 5.25v4.75l3 1.75M3.75 5.5v-2.25M3.75 3.25H6m-2.1 2.4A7 7 0 1 1 3 8.75" />
        </svg>
        <strong>{{ activeTab === 'changes' ? 'Changes' : 'Commits' }}</strong>
        <span>
          {{
            activeTab === 'changes'
              ? 'The file list and diff viewer will appear here.'
              : 'The commit history will appear here.'
          }}
        </span>
      </div>
    </div>
    <RunConsole :project-id="project.id" />
  </section>

  <section v-else class="work-panel work-panel--empty">
    <div class="work-panel__placeholder">
      <svg viewBox="0 0 20 20" aria-hidden="true">
        <path d="M4.25 4.25h11.5v11.5H4.25zM7 7h6m-6 3h6m-6 3h3" />
      </svg>
      <strong>Select work to review</strong>
      <span>Choose a working, ready, or shipped item from the sidebar.</span>
    </div>
  </section>
</template>

<style scoped>
.work-panel {
  display: flex;
  width: 100%;
  height: 100%;
  min-width: 0;
  flex-direction: column;
  overflow: hidden;
}

.review-tabs {
  display: flex;
  flex: 0 0 auto;
  align-items: end;
  gap: 20px;
  height: 39px;
  padding: 0 16px;
  border-bottom: 1px solid var(--border-subtle);
}

.review-tabs button {
  position: relative;
  display: flex;
  align-items: center;
  gap: 6px;
  height: 39px;
  padding: 0 1px;
  font: inherit;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: 0;
}

.review-tabs button:hover,
.review-tabs__tab--active {
  color: var(--text-primary) !important;
}

.review-tabs__tab--active::after {
  position: absolute;
  right: 0;
  bottom: -1px;
  left: 0;
  height: 2px;
  content: '';
  background: var(--primary);
  border-radius: 1px 1px 0 0;
}

.review-tabs button:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: -2px;
}

.review-tabs button span {
  display: grid;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  place-items: center;
  font-size: 10px;
  color: var(--text-secondary);
  background: var(--surface-subtle);
  border-radius: 8px;
}

.work-panel__body {
  min-height: 0;
  flex: 1;
}

.work-panel__placeholder {
  display: flex;
  height: 100%;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 7px;
  color: var(--text-secondary);
  text-align: center;
}

.work-panel__placeholder svg {
  width: 24px;
  margin-bottom: 3px;
  fill: none;
  stroke: var(--text-muted);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.1;
}

.work-panel__placeholder strong {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

.work-panel__placeholder span {
  max-width: 300px;
  font-size: 12px;
  line-height: 1.45;
}

.work-panel--empty {
  padding-top: var(--titlebar-height);
}
</style>
