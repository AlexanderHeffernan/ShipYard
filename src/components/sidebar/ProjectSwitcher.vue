<script setup lang="ts">
import { Plus, Settings, X } from '@lucide/vue';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import AppButton from '../ui/AppButton.vue';
import type { Project } from '../../types/projects';

const props = defineProps<{
  projects: Project[];
  loading: boolean;
  error: string | null;
}>();

const emit = defineEmits<{
  add: [];
  remove: [id: string];
  settings: [id: string];
}>();

const root = ref<HTMLElement>();
const open = ref(false);
const projectLabel = computed(
  () => `${props.projects.length} Project${props.projects.length === 1 ? '' : 's'}`,
);

function closeMenuOnOutsideClick(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node)) open.value = false;
}

function closeMenuOnEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') open.value = false;
}

function openSettings(projectId: string) {
  open.value = false;
  emit('settings', projectId);
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
      class="project-switcher__trigger"
      type="button"
      aria-controls="project-menu"
      :aria-expanded="open"
      @click="open = !open"
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
          >
            <div class="project-menu__project">
              <span class="project-dot" :style="{ background: project.color }"></span>
              <span>{{ project.name }}</span>
            </div>

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
              @click="emit('remove', project.id)"
            >
              <X aria-hidden="true" />
            </AppButton>
          </div>
        </div>

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
  overflow: hidden;
  color: var(--text-primary);
  background: var(--surface-content);
  border: 1px solid rgba(255, 255, 255, 0.13);
  border-radius: 10px;
  box-shadow: 0 14px 36px rgba(0, 0, 0, 0.42);
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
  color: #ff9b9b;
  border-bottom: 1px solid var(--border-subtle);
}

.project-menu__row {
  display: flex;
  align-items: center;
  height: 37px;
  border-radius: 6px;
}

.project-menu__project {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 11px;
  min-width: 0;
  height: 100%;
  padding: 0 9px;
  overflow: hidden;
  font-size: 13px;
  color: inherit;
}

.project-menu__project span:last-child {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-dot {
  flex: 0 0 auto;
  width: 9px;
  height: 9px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.24);
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
