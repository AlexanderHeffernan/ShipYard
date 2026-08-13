<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import AppSettingsModal from './components/settings/AppSettingsModal.vue';
import WorkItemPanel from './components/review/WorkItemPanel.vue';
import ProjectSettingsModal from './components/settings/ProjectSettingsModal.vue';
import AppSidebar from './components/sidebar/AppSidebar.vue';
import ProjectSwitcher from './components/sidebar/ProjectSwitcher.vue';
import ConfirmationDialog from './components/ui/ConfirmationDialog.vue';
import { useProjects } from './composables/useProjects';
import { useNotifications } from './composables/useNotifications';
import { useUpdates } from './composables/useUpdates';
import { useWindowFullscreen } from './composables/useWindowFullscreen';
import { deleteWorkItem, inspectWorkItemDeletion } from './services/projects';
import type { DeleteWorkItemRequest, DeletionPlan, Project, WorkItem } from './types/projects';
import { titlebarControlsInset } from './utils/titlebar';

const sidebarOpen = ref(true);
const sidebarWidth = ref(288);
const selectedWorkItemId = ref<string | null>(null);
const settingsProject = ref<Project | null>(null);
const settingsSection = ref<'open' | 'run'>('run');
const appSettingsOpen = ref(false);
const notificationsReady = ref(false);
const deletion = ref<{
  project: Project;
  item: WorkItem;
  request: DeleteWorkItemRequest;
  plan: DeletionPlan | null;
  inspecting: boolean;
  deleting: boolean;
  error: string | null;
} | null>(null);
const {
  projects,
  loading,
  error,
  loadProjects,
  refreshAllProjects,
  addProject,
  rescanProject,
  removeProject,
  disposeProjects,
} = useProjects();
const notifications = useNotifications();
const { startAutomaticChecks, stopAutomaticChecks } = useUpdates();
const { isFullscreen, fullscreenStateReady } = useWindowFullscreen();
const selection = computed(() => {
  for (const project of projects.value) {
    const workItem = project.workItems.find((item) => item.id === selectedWorkItemId.value);
    if (workItem && !workItem.completed && workItem.status !== 'shipped') return { project, workItem };
  }
  return null;
});

async function initializeApp() {
  await loadProjects();
  await notifications.loadSettings();
  notificationsReady.value = true;
  await notifications.observeProjects(projects.value);
  notifications.startPolling(refreshAllProjects);
}

onMounted(() => {
  void initializeApp();
  startAutomaticChecks();
});
onBeforeUnmount(() => {
  disposeProjects();
  notifications.stopPolling();
  stopAutomaticChecks();
});
watch(projects, (value) => {
  if (notificationsReady.value) void notifications.observeProjects(value);
}, { deep: true });
watch(selection, (current) => {
  if (selectedWorkItemId.value && !current) selectedWorkItemId.value = null;
});

function openSettings(project: Project, section: 'open' | 'run' = 'run') {
  settingsProject.value = project;
  settingsSection.value = section;
}

function openProjectSettings(projectId: string) {
  const project = projects.value.find((candidate) => candidate.id === projectId);
  if (project) openSettings(project);
}

function message(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

async function openDeletion(workItemId: string) {
  const project = projects.value.find((candidate) => candidate.workItems.some((item) => item.id === workItemId));
  const item = project?.workItems.find((candidate) => candidate.id === workItemId);
  if (!project || !item) return;
  const request: DeleteWorkItemRequest = {
    projectPath: project.path,
    projectId: project.id,
    workItemId: item.id,
    branch: item.branch,
    worktreePath: item.worktreePath,
    headSha: item.headSha,
  };
  deletion.value = { project, item, request, plan: null, inspecting: true, deleting: false, error: null };
  const current = deletion.value;
  try {
    const plan = await inspectWorkItemDeletion(request);
    if (deletion.value === current) current.plan = plan;
  } catch (inspectError) {
    if (deletion.value === current) current.error = message(inspectError);
  } finally {
    if (deletion.value === current) current.inspecting = false;
  }
}

async function confirmDeletion() {
  const current = deletion.value;
  if (!current?.plan || current.inspecting || current.deleting) return;
  current.deleting = true;
  current.error = null;
  try {
    const result = await deleteWorkItem(current.request, current.plan);
    if (selectedWorkItemId.value === result.workItemId) selectedWorkItemId.value = null;
    await rescanProject(result.projectId);
    deletion.value = null;
  } catch (deleteError) {
    current.error = message(deleteError);
    current.deleting = false;
  }
}

const deletionTitle = computed(() => deletion.value?.item.branch
  ? `Delete ${deletion.value.item.branch}?`
  : 'Delete detached worktree?');
const deletionDescription = computed(() => {
  const plan = deletion.value?.plan;
  if (!plan) return 'ShipYard is validating this work item against the current repository state.';
  if (plan.switchesPrimaryCheckout) {
    return `ShipYard will switch the primary checkout to ${plan.defaultBranch} and permanently delete the local branch ${plan.branch}.`;
  }
  if (plan.removesWorktree && plan.deletesBranch) {
    return `ShipYard will permanently remove this linked worktree from disk and delete the local branch ${plan.branch}.`;
  }
  if (plan.removesWorktree) return 'ShipYard will permanently remove this detached linked worktree from disk.';
  return `ShipYard will permanently delete the local branch ${plan.branch}.`;
});
const deletionConfirmLabel = computed(() => {
  const plan = deletion.value?.plan;
  if (!plan) return 'Delete work item';
  if (plan.switchesPrimaryCheckout) return 'Switch and delete branch';
  if (plan.removesWorktree && plan.deletesBranch) return 'Delete branch and worktree';
  if (plan.removesWorktree) return 'Delete worktree';
  return 'Delete local branch';
});
</script>

<template>
  <div
    class="app-shell"
    :style="{ '--titlebar-controls-leading-inset': `${titlebarControlsInset(isFullscreen)}px` }"
    :data-window-fullscreen="isFullscreen ? 'true' : 'false'"
  >
    <header class="window-drag-region" data-tauri-drag-region></header>

    <div v-if="fullscreenStateReady" class="titlebar-controls">
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
      @delete="openDeletion"
      @settings="appSettingsOpen = true"
    />

    <main class="app-content">
      <WorkItemPanel
        :project="selection?.project ?? null"
        :work-item="selection?.workItem ?? null"
        :sidebar-open="sidebarOpen"
        :fullscreen="isFullscreen"
        @settings="openSettings"
        @refresh="rescanProject"
      />
    </main>

    <ProjectSettingsModal
      v-if="settingsProject"
      :project="settingsProject"
      :initial-section="settingsSection"
      @close="settingsProject = null"
    />
    <AppSettingsModal v-if="appSettingsOpen" @close="appSettingsOpen = false" />
    <ConfirmationDialog
      v-if="deletion"
      :title="deletionTitle"
      :description="deletionDescription"
      :confirm-label="deletionConfirmLabel"
      :confirm-disabled="deletion.inspecting || !deletion.plan"
      :loading="deletion.deleting"
      loading-label="Deleting"
      :error="deletion.error"
      @cancel="deletion = null"
      @confirm="confirmDeletion"
    >
      <template v-if="deletion.plan">
        <p v-if="deletion.plan.worktreePath">
          Worktree: <code>{{ deletion.plan.worktreePath }}</code>
        </p>
        <ul v-if="deletion.plan.hasUncommittedChanges || deletion.plan.unpushedCommits > 0">
          <li v-if="deletion.plan.hasUncommittedChanges">
            Uncommitted files in this worktree will be permanently lost.
          </li>
          <li v-if="deletion.plan.unpushedCommits > 0">
            {{ deletion.plan.unpushedCommits }} unpushed local commit{{ deletion.plan.unpushedCommits === 1 ? '' : 's' }} unique to this work item will become unreachable.
          </li>
        </ul>
        <p class="deletion-remote-note">
          {{ deletion.item.pullRequest ? 'The GitHub pull request and its remote branch will remain untouched.' : 'Remote branches will remain untouched.' }}
        </p>
      </template>
    </ConfirmationDialog>
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

.deletion-remote-note {
  margin-top: 12px !important;
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
  left: var(--titlebar-controls-leading-inset);
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
