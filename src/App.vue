<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import WorkItemPanel from './components/review/WorkItemPanel.vue';
import ProjectSettingsModal from './components/settings/ProjectSettingsModal.vue';
import AppSidebar from './components/sidebar/AppSidebar.vue';
import ProjectSwitcher from './components/sidebar/ProjectSwitcher.vue';
import { useProjects } from './composables/useProjects';
import type { Project } from './types/projects';

const sidebarOpen = ref(true);
const sidebarWidth = ref(288);
const selectedWorkItemId = ref<string | null>(null);
const settingsTarget = ref<{ project: Project; section: 'run' | 'open' } | null>(null);
const { projects, loading, error, loadProjects, addProject, removeProject } = useProjects();
const selection = computed(() => {
  for (const project of projects.value) {
    const workItem = project.workItems.find((item) => item.id === selectedWorkItemId.value);
    if (workItem) return { project, workItem };
  }
  return null;
});

function openSettings(project: Project, section: 'run' | 'open' = 'run') {
  settingsTarget.value = { project, section };
}

function openProjectSettings(projectId: string) {
  const project = projects.value.find((candidate) => candidate.id === projectId);
  if (project) openSettings(project);
}

onMounted(loadProjects);
</script>

<template>
  <div class="app-shell">
    <header class="window-drag-region" data-tauri-drag-region></header>

    <div class="titlebar-controls">
      <button
        class="sidebar-toggle"
        type="button"
        :aria-label="sidebarOpen ? 'Hide sidebar' : 'Show sidebar'"
        :title="sidebarOpen ? 'Hide sidebar' : 'Show sidebar'"
        @click="sidebarOpen = !sidebarOpen"
      >
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <rect x="2.75" y="3.25" width="14.5" height="13.5" rx="2.25" />
          <path d="M7.25 3.75v12.5" />
        </svg>
      </button>

      <ProjectSwitcher
        v-if="sidebarOpen"
        :projects="projects"
        :loading="loading"
        :error="error"
        @add="addProject"
        @remove="removeProject"
        @settings="openProjectSettings"
      />
    </div>

    <AppSidebar
      :open="sidebarOpen"
      v-model:width="sidebarWidth"
      :projects="projects"
      :selected-work-item-id="selectedWorkItemId"
      @select="selectedWorkItemId = $event"
    />

    <main class="app-content">
      <WorkItemPanel
        :project="selection?.project ?? null"
        :work-item="selection?.workItem ?? null"
        :sidebar-open="sidebarOpen"
        @settings="openSettings"
      />
    </main>

    <ProjectSettingsModal
      v-if="settingsTarget"
      :project="settingsTarget.project"
      :initial-section="settingsTarget.section"
      @close="settingsTarget = null"
    />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
}

.app-content {
  position: relative;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  background: var(--surface-content);
}

.window-drag-region {
  position: fixed;
  z-index: 3;
  inset: 0 0 auto;
  height: 36px;
}

.titlebar-controls {
  position: fixed;
  z-index: 4;
  top: 4px;
  left: 82px;
  display: flex;
  align-items: center;
  gap: 4px;
}

.sidebar-toggle {
  display: grid;
  width: 24px;
  height: 24px;
  padding: 0;
  place-items: center;
  color: var(--text-secondary);
  background: transparent;
  border: 0;
  border-radius: 7px;
}

.sidebar-toggle:hover {
  color: var(--text-primary);
  background: var(--surface-hover);
}

.sidebar-toggle:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 1px;
}

.sidebar-toggle svg {
  width: 15px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.35;
}
</style>
