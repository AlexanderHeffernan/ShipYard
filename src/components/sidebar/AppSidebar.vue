<script setup lang="ts">
import { Settings, Trash2 } from '@lucide/vue';
import { computed, onBeforeUnmount, ref } from 'vue';
import AppButton from '../ui/AppButton.vue';
import type { Project, WorkItem } from '../../types/projects';
import { pullRequestSyncState, workItemMeta, workItemTitle, type PullRequestSyncState } from '../../utils/workItems';

const MIN_WIDTH = 224;
const MAX_WIDTH = 420;

type SidebarWorkItem = WorkItem & {
  title: string;
  meta: string;
  color: string;
  projectName: string;
  syncState: PullRequestSyncState | null;
  deletable: boolean;
};

type WorkSection = {
  id: 'local' | 'pullRequests';
  label: string;
  items: SidebarWorkItem[];
};

const collapsedSections = ref(new Set<WorkSection['id']>());

const props = defineProps<{
  open: boolean;
  width: number;
  projects: Project[];
  selectedWorkItemId: string | null;
}>();

const emit = defineEmits<{
  'update:width': [value: number];
  select: [id: string];
  delete: [id: string];
  settings: [];
}>();

const isResizing = ref(false);

const sections = computed<WorkSection[]>(() => {
  const items = props.projects.flatMap((project) =>
    project.workItems.map((item) => ({
      ...item,
      title: workItemTitle(item),
      meta: workItemMeta(item),
      color: project.color,
      projectName: project.name,
      syncState: pullRequestSyncState(item),
      deletable: item.branch
        ? item.branch !== project.defaultBranch
        : !!item.worktreePath && item.worktreePath !== project.path,
    })),
  );

  return [
    {
      id: 'local',
      label: 'Local Work',
      items: items.filter((item) => !item.completed && item.status !== 'shipped' && !item.pullRequest),
    },
    {
      id: 'pullRequests',
      label: 'Pull Requests',
      items: items.filter((item) => !item.completed && !!item.pullRequest),
    },
  ];
});
const hasWorkItems = computed(() => sections.value.some((section) => section.items.length > 0));

function clampWidth(width: number) {
  return Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, width));
}

function resizeTo(clientX: number) {
  emit('update:width', clampWidth(clientX));
}

function stopResize() {
  isResizing.value = false;
  document.body.classList.remove('is-resizing-sidebar');
  window.removeEventListener('pointermove', onPointerMove);
  window.removeEventListener('pointerup', stopResize);
  window.removeEventListener('pointercancel', stopResize);
}

function onPointerMove(event: PointerEvent) {
  resizeTo(event.clientX);
}

function startResize(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  isResizing.value = true;
  document.body.classList.add('is-resizing-sidebar');
  window.addEventListener('pointermove', onPointerMove);
  window.addEventListener('pointerup', stopResize);
  window.addEventListener('pointercancel', stopResize);
}

function resizeWithKeyboard(event: KeyboardEvent) {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
  event.preventDefault();
  const delta = event.key === 'ArrowLeft' ? -10 : 10;
  emit('update:width', clampWidth(props.width + delta));
}

function toggleSection(id: WorkSection['id']) {
  const next = new Set(collapsedSections.value);
  next.has(id) ? next.delete(id) : next.add(id);
  collapsedSections.value = next;
}

onBeforeUnmount(stopResize);
</script>

<template>
  <aside
    class="sidebar"
    :class="{
      'sidebar--closed': !open,
      'sidebar--resizing': isResizing,
    }"
    :style="{ width: open ? `${width}px` : '0px' }"
    :aria-hidden="!open"
    :inert="!open"
  >
    <div class="sidebar__body" :style="{ width: `${width}px` }">
      <nav class="sidebar__content" aria-label="Work">
        <section v-for="section in hasWorkItems ? sections : []" :key="section.id" class="work-section">
          <button
            class="work-section__header"
            type="button"
            :aria-expanded="!collapsedSections.has(section.id)"
            @click="toggleSection(section.id)"
          >
            <span class="work-section__label">
              <span>{{ section.label }}</span>
              <span class="work-section__count">{{ section.items.length }}</span>
            </span>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="m4.5 6 3.5 3.5L11.5 6" />
            </svg>
          </button>

          <div v-if="!collapsedSections.has(section.id)" class="work-section__items">
            <div
              v-for="item in section.items"
              :key="item.id"
              class="work-item"
              :class="{ 'work-item--selected': item.id === selectedWorkItemId }"
            >
              <button
                class="work-item__select"
                type="button"
                :aria-current="item.id === selectedWorkItemId ? 'page' : undefined"
                :title="`${item.projectName} · ${item.lastCommitSubject || item.title}`"
                @click="emit('select', item.id)"
              >
                <span class="work-item__dot" :style="{ background: item.color }"></span>
                <span class="work-item__title">{{ item.title }}</span>
                <span class="work-item__meta" :class="{ 'work-item__meta--attention': item.syncState && item.syncState !== 'synced', 'work-item__meta--danger': item.syncState === 'diverged' }">{{ item.meta }}</span>
              </button>
              <button
                v-if="item.deletable"
                class="work-item__delete"
                type="button"
                :aria-label="`Delete ${item.title}`"
                :title="`Delete ${item.title}`"
                @click="emit('delete', item.id)"
              >
                <Trash2 aria-hidden="true" />
              </button>
            </div>
          </div>
        </section>
      </nav>
      <footer class="sidebar__footer">
        <AppButton
          variant="ghost"
          block
          type="button"
          title="Settings for Shipyard across all projects"
          @click="emit('settings')"
        >
          <Settings aria-hidden="true" />
          Shipyard Settings
        </AppButton>
      </footer>
    </div>

    <div
      class="sidebar__resize-handle"
      role="separator"
      aria-label="Resize sidebar"
      aria-orientation="vertical"
      :aria-valuemin="MIN_WIDTH"
      :aria-valuemax="MAX_WIDTH"
      :aria-valuenow="width"
      tabindex="0"
      @pointerdown="startResize"
      @keydown="resizeWithKeyboard"
    />
  </aside>
</template>

<style scoped>
.sidebar {
  position: relative;
  z-index: 2;
  flex: 0 0 auto;
  height: 100%;
  overflow: hidden;
  user-select: none;
  border-right: 1px solid var(--border-subtle);
  background: var(--surface-sidebar);
  box-shadow: 1px 0 0 rgba(0, 0, 0, 0.12);
  transition:
    width 180ms cubic-bezier(0.2, 0.75, 0.25, 1),
    border-color 180ms ease,
    box-shadow 180ms ease;
}

.sidebar--closed {
  border-color: transparent;
  box-shadow: none;
}

.sidebar--resizing {
  transition: none;
}

.sidebar__body {
  display: flex;
  height: 100%;
  padding-top: var(--titlebar-height);
  flex-direction: column;
  overflow: hidden;
}

.sidebar__content {
  min-height: 0;
  flex: 1;
  padding: 0 12px 12px;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.18) transparent;
}

.sidebar__footer {
  flex: 0 0 auto;
  padding: 9px 12px 12px;
  border-top: 1px solid var(--border-subtle);
}

.sidebar__footer :deep(.app-button) {
  justify-content: flex-start;
  height: 30px;
  padding: 0 8px;
}

.work-section + .work-section {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border-subtle);
}

.work-section__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 32px;
  padding: 0 7px;
  font: inherit;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  text-align: left;
  text-transform: uppercase;
  letter-spacing: 0.035em;
  background: transparent;
  border: 0;
  border-radius: 6px;
}

.work-section__label {
  display: flex;
  align-items: center;
  gap: 7px;
}

.work-section__count {
  display: grid;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  place-items: center;
  font-size: 10px;
  line-height: 1;
  color: var(--text-secondary);
  background: rgba(255, 255, 255, 0.055);
  border-radius: 8px;
}

.work-section__header:hover {
  color: var(--text-primary);
  background: rgba(255, 255, 255, 0.035);
}

.work-section__header:focus-visible,
.work-item__select:focus-visible,
.work-item__delete:focus-visible,
.sidebar__resize-handle:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 1px;
}

.work-section__header svg {
  width: 14px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.35;
  transition: transform 120ms ease;
}

.work-section__header[aria-expanded='false'] svg {
  transform: rotate(-90deg);
}

.work-section__items {
  padding: 1px 0 5px;
}

.work-item {
  position: relative;
  height: 39px;
  border-radius: 6px;
}

.work-item__select {
  display: grid;
  grid-template-columns: 10px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  width: 100%;
  height: 39px;
  padding: 0 7px;
  font: inherit;
  font-size: 12px;
  color: var(--text-primary);
  text-align: left;
  background: transparent;
  border: 0;
  border-radius: 6px;
}

.work-item:hover,
.work-item:focus-within {
  background: var(--surface-hover);
}

.work-item--selected {
  background: var(--surface-hover);
}

.work-item__dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.24);
}

.work-item__title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.work-item__meta {
  transition: opacity 100ms ease;
  color: var(--text-secondary);
  white-space: nowrap;
}

.work-item__delete {
  position: absolute;
  top: 5px;
  right: 3px;
  display: grid;
  width: 29px;
  height: 29px;
  padding: 0;
  place-items: center;
  color: var(--danger);
  visibility: hidden;
  background: var(--surface-elevated);
  border: 0;
  border-radius: 5px;
  opacity: 0;
  transition: color 100ms ease, background 100ms ease, opacity 100ms ease;
}

.work-item__delete svg {
  width: 14px;
  height: 14px;
  stroke-width: 1.7;
}

.work-item:hover .work-item__meta,
.work-item:focus-within .work-item__meta {
  opacity: 0;
}

.work-item:hover .work-item__delete,
.work-item:focus-within .work-item__delete,
.work-item__delete:focus {
  visibility: visible;
  opacity: 1;
}

.work-item__delete:hover {
  color: var(--danger);
  background: var(--danger-subtle);
}

.work-item__meta--attention {
  color: var(--warning);
}

.work-item__meta--danger {
  color: var(--danger);
}

.sidebar__resize-handle {
  position: absolute;
  z-index: 3;
  top: 0;
  right: -4px;
  bottom: 0;
  width: 8px;
  cursor: col-resize;
  touch-action: none;
}

.sidebar--closed .sidebar__resize-handle {
  display: none;
}

.sidebar__resize-handle::after {
  position: absolute;
  top: 0;
  right: 3px;
  bottom: 0;
  width: 1px;
  content: '';
  background: transparent;
  transition: background 120ms ease;
}

.sidebar__resize-handle:hover::after,
.sidebar--resizing .sidebar__resize-handle::after {
  background: var(--resize-indicator);
}
</style>
