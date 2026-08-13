<script setup lang="ts">
import { Plus, Settings, X } from '@lucide/vue';
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import AppButton from '../ui/AppButton.vue';
import ProjectIcon from '../ui/ProjectIcon.vue';
import ProjectIdentityPopover from './ProjectIdentityPopover.vue';
import type { Project, ProjectCustomization } from '../../types/projects';
import { projectDefaultColor } from '../../utils/projectIdentity';

const props = defineProps<{
  projects: Project[];
  loading: boolean;
  error: string | null;
}>();

const emit = defineEmits<{
  add: [];
  remove: [id: string];
  settings: [id: string];
  identity: [id: string, patch: Partial<ProjectCustomization>];
}>();

const root = ref<HTMLElement>();
const trigger = ref<HTMLButtonElement>();
const open = ref(false);
const identityProjectId = ref<string | null>(null);
const projectLabel = computed(
  () => `${props.projects.length} Project${props.projects.length === 1 ? '' : 's'}`,
);

function closeMenuOnOutsideClick(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node)) {
    open.value = false;
    identityProjectId.value = null;
  }
}

function closeMenuOnEscape(event: KeyboardEvent) {
  if (event.key !== 'Escape') return;
  if (identityProjectId.value) {
    closeIdentity(true);
    event.preventDefault();
    return;
  }
  if (open.value) {
    open.value = false;
    trigger.value?.focus();
    event.preventDefault();
  }
}

function toggleMenu() {
  open.value = !open.value;
  if (!open.value) identityProjectId.value = null;
}

function identityPopoverId(project: Project) {
  return `project-identity-${project.id.replace(/[^a-zA-Z0-9_-]/g, '-')}`;
}

function focusIdentityTrigger(projectId: string) {
  nextTick(() => {
    const buttons = root.value?.querySelectorAll<HTMLButtonElement>('[data-project-identity-trigger]');
    const button = [...(buttons ?? [])].find((candidate) => candidate.dataset.projectIdentityTrigger === projectId);
    button?.focus();
  });
}

function toggleIdentity(projectId: string) {
  identityProjectId.value = identityProjectId.value === projectId ? null : projectId;
}

function closeIdentity(returnFocus = true) {
  const projectId = identityProjectId.value;
  identityProjectId.value = null;
  if (returnFocus && projectId) focusIdentityTrigger(projectId);
}

function openSettings(projectId: string) {
  closeIdentity(false);
  open.value = false;
  emit('settings', projectId);
}

function removeProject(projectId: string) {
  if (identityProjectId.value === projectId) closeIdentity(false);
  emit('remove', projectId);
}

function updateIdentity(projectId: string, patch: Partial<ProjectCustomization>) {
  emit('identity', projectId, patch);
}

onMounted(() => {
  document.addEventListener('pointerdown', closeMenuOnOutsideClick);
  document.addEventListener('keydown', closeMenuOnEscape);
});

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', closeMenuOnOutsideClick);
  document.removeEventListener('keydown', closeMenuOnEscape);
});
</script>

<template>
  <div ref="root" class="project-switcher">
    <button
      ref="trigger"
      class="project-switcher__trigger"
      type="button"
      aria-controls="project-menu"
      :aria-expanded="open"
      @click="toggleMenu"
    >
      <span>{{ projectLabel }}</span>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="m4.5 6 3.5 3.5L11.5 6" />
      </svg>
    </button>

    <Transition name="project-menu">
      <div v-if="open" id="project-menu" class="project-menu">
        <p v-if="error" class="project-menu__error">{{ error }}</p>

        <div class="project-menu__list">
          <div
            v-for="project in projects"
            :key="project.id"
            class="project-menu__row"
            :class="{ 'project-menu__row--identity-open': identityProjectId === project.id }"
          >
            <button
              class="project-menu__identity-trigger"
              type="button"
              :data-project-identity-trigger="project.id"
              :aria-label="`Customize ${project.name} icon`"
              :aria-expanded="identityProjectId === project.id"
              :aria-controls="identityPopoverId(project)"
              @click="toggleIdentity(project.id)"
            >
              <ProjectIcon :color="project.color" :image="project.image" size="medium" />
            </button>
            <div class="project-menu__project">
              <span class="project-menu__name">{{ project.name }}</span>
            </div>

            <ProjectIdentityPopover
              v-if="identityProjectId === project.id"
              :project="project"
              :default-color="projectDefaultColor(project.id)"
              @color="updateIdentity(project.id, { color: $event })"
              @image="updateIdentity(project.id, { image: $event })"
            />

            <AppButton
              class="project-menu__settings"
              variant="ghost"
              size="icon"
              type="button"
              :aria-label="`Settings for ${project.name}`"
              :title="`Settings for ${project.name}`"
              @click="openSettings(project.id)"
            >
              <Settings aria-hidden="true" />
            </AppButton>

            <AppButton
              class="project-menu__close"
              variant="ghost"
              size="icon"
              type="button"
              :aria-label="`Close ${project.name}`"
              :title="`Close ${project.name}`"
              @click="removeProject(project.id)"
            >
              <X aria-hidden="true" />
            </AppButton>
          </div>
        </div>

        <p v-if="projects.length === 0" class="project-menu__empty">Add a Git project to start building your queue.</p>

        <AppButton
          class="project-menu__add"
          variant="ghost"
          block
          type="button"
          :loading="loading"
          loading-label="Scanning"
          @click="emit('add')"
        >
          <Plus aria-hidden="true" />
          <span>Add project</span>
        </AppButton>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.project-switcher {
  position: relative;
}

.project-switcher__trigger {
  display: flex;
  align-items: center;
  gap: 5px;
  height: 24px;
  padding: 0 5px;
  font: inherit;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: 0;
  border-radius: 7px;
}

.project-switcher__trigger:hover,
.project-switcher__trigger[aria-expanded='true'] {
  color: var(--text-primary);
  background: var(--surface-hover);
}

.project-switcher__trigger:focus-visible,
.project-menu button:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 1px;
}

.project-switcher__trigger svg {
  width: 11px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.5;
}

.project-menu {
  position: absolute;
  top: 30px;
  left: 0;
  width: 244px;
  overflow: visible;
  color: var(--text-primary);
  background: var(--surface-elevated);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  box-shadow: var(--shadow-elevated);
  transform-origin: top left;
}

.project-menu__list {
  padding: 7px;
}

.project-menu__error {
  margin: 0;
  padding: 10px 14px;
  font-size: 11px;
  line-height: 1.4;
  color: var(--danger);
  border-bottom: 1px solid var(--border-subtle);
}

.project-menu__row {
  position: relative;
  display: flex;
  align-items: center;
  height: 37px;
  border-radius: 6px;
}

.project-menu__row:hover,
.project-menu__row--identity-open {
  background: var(--surface-hover);
}

.project-menu__row--identity-open {
  z-index: 2;
}

.project-menu__identity-trigger {
  display: grid;
  flex: 0 0 auto;
  width: 35px;
  height: 35px;
  padding: 0;
  place-items: center;
  color: inherit;
  background: transparent;
  border: 0;
  border-radius: 6px;
}

.project-menu__identity-trigger:hover {
  background: rgba(255, 255, 255, 0.075);
}

.project-menu__project {
  display: flex;
  flex: 1;
  align-items: center;
  min-width: 0;
  height: 100%;
  padding: 0 4px;
  overflow: hidden;
  font-size: 13px;
  color: inherit;
}

.project-menu__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-menu__empty {
  margin: 0;
  padding: 7px 15px 13px;
  font-size: 11px;
  line-height: 1.4;
  color: var(--text-secondary);
}

.project-menu__close,
.project-menu__settings {
  flex: 0 0 auto;
  width: 30px;
  height: 30px;
}

.project-menu__close:hover,
.project-menu__settings:hover {
  background: rgba(255, 255, 255, 0.07);
}

.project-menu__close svg,
.project-menu__settings svg,
.project-menu__add svg {
  width: 15px;
  height: 15px;
}

.project-menu__add {
  display: flex;
  align-items: center;
  gap: 11px;
  width: 100%;
  height: 46px;
  padding: 0 16px;
  font: inherit;
  font-size: 13px;
  color: var(--text-primary);
  background: rgba(255, 255, 255, 0.02);
  border: 0;
  border-top: 1px solid var(--border-subtle);
  border-radius: 0 0 9px 9px;
}

.project-menu__add:hover {
  background: var(--surface-hover);
}

.project-menu__add:disabled {
  color: var(--text-secondary);
}

.project-menu-enter-active,
.project-menu-leave-active {
  transition: opacity 110ms ease, transform 110ms ease;
}

.project-menu-enter-from,
.project-menu-leave-to {
  opacity: 0;
  transform: translateY(-3px) scale(0.985);
}
</style>
