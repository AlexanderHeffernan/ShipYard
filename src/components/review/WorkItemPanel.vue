<script setup lang="ts">
import { ref } from 'vue';
import type { Project, WorkItem } from '../../types/projects';
import WorkItemCommits from './WorkItemCommits.vue';
import RunConsole from './RunConsole.vue';
import WorkItemChanges from './WorkItemChanges.vue';
import WorkItemHeader from './WorkItemHeader.vue';

defineProps<{
  project: Project | null;
  workItem: WorkItem | null;
  sidebarOpen: boolean;
  fullscreen: boolean;
}>();

const emit = defineEmits<{
  settings: [project: Project, section: 'open' | 'run'];
  refresh: [projectId: string];
}>();

type ReviewTab = 'changes' | 'commits';

const activeTab = ref<ReviewTab>('changes');

function selectTab(tab: ReviewTab) {
  activeTab.value = tab;
}

function handleTabKey(event: KeyboardEvent) {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight' && event.key !== 'Home' && event.key !== 'End') return;
  event.preventDefault();
  const nextTab = event.key === 'Home' || (event.key === 'ArrowLeft' && activeTab.value === 'commits')
    ? 'changes'
    : event.key === 'End' || (event.key === 'ArrowRight' && activeTab.value === 'changes')
      ? 'commits'
      : activeTab.value;
  selectTab(nextTab);
  document.querySelector<HTMLButtonElement>(`[data-review-tab="${nextTab}"]`)?.focus();
}
</script>

<template>
  <section v-if="project && workItem" class="work-panel">
    <WorkItemHeader
      :project="project"
      :work-item="workItem"
      :sidebar-open="sidebarOpen"
      :fullscreen="fullscreen"
      @settings="emit('settings', project, $event)"
      @refresh="emit('refresh', project.id)"
    />

    <nav class="review-tabs" role="tablist" aria-label="Work item details" @keydown="handleTabKey">
      <button
        id="work-item-tab-changes"
        data-review-tab="changes"
        role="tab"
        type="button"
        :class="{ 'review-tabs__tab--active': activeTab === 'changes' }"
        :aria-selected="activeTab === 'changes'"
        :tabindex="activeTab === 'changes' ? 0 : -1"
        @click="selectTab('changes')"
      >
        Changes
        <span>{{ workItem.changedFiles }}</span>
      </button>
      <button
        id="work-item-tab-commits"
        data-review-tab="commits"
        role="tab"
        type="button"
        :class="{ 'review-tabs__tab--active': activeTab === 'commits' }"
        :aria-selected="activeTab === 'commits'"
        :tabindex="activeTab === 'commits' ? 0 : -1"
        @click="selectTab('commits')"
      >
        Commits
      </button>
    </nav>

    <div class="work-panel__body" role="tabpanel" :aria-labelledby="`work-item-tab-${activeTab}`" tabindex="0">
      <WorkItemChanges
        v-if="activeTab === 'changes'"
        :project="project"
        :work-item="workItem"
      />
      <WorkItemCommits
        v-else
        :project="project"
        :work-item="workItem"
        @show-changes="activeTab = 'changes'"
      />
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
  display: flex;
  min-height: 0;
  flex: 1;
  overflow: hidden;
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
