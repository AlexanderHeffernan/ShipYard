<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useRunner } from '../../composables/useRunner';
import { useRunScripts } from '../../composables/useRunScripts';
import { checkoutPullRequest } from '../../services/projects';
import type { Project, WorkItem } from '../../types/projects';
import type { RunScript } from '../../types/run';

const props = defineProps<{ project: Project; workItem: WorkItem }>();
const emit = defineEmits<{ settings: [section: 'run'] }>();
const runScripts = useRunScripts();
const { currentRun, error, run, cancel } = useRunner();
const root = ref<HTMLElement>();
const menuOpen = ref(false);

const scriptStore = computed(() => runScripts);
const settings = computed(() => scriptStore.value.settingsByProject.value[props.project.id]);
const defaultScript = computed(() =>
  settings.value?.scripts.find((script) => script.id === settings.value?.defaultScriptId),
);
const isActive = computed(
  () =>
    currentRun.value?.projectId === props.project.id &&
    currentRun.value?.kind === 'run' &&
    ['running', 'stopping'].includes(currentRun.value.status),
);
const anotherRunActive = computed(
  () =>
    !!currentRun.value &&
    ['running', 'stopping'].includes(currentRun.value.status) &&
    !isActive.value,
);
const label = computed(() => {
  if (isActive.value && currentRun.value?.status === 'stopping') return 'Stopping…';
  if (isActive.value) return 'Stop';
  if (!defaultScript.value) return 'Set up Run';
  return defaultScript.value.label;
});
const actionName = computed(() => 'Run');
const mainDisabled = computed(
  () =>
    anotherRunActive.value ||
    currentRun.value?.status === 'stopping' ||
    (!!defaultScript.value && !props.workItem.worktreePath),
);

async function runDefault() {
  if (isActive.value) return cancel();
  if (!defaultScript.value) return emit('settings', 'run');
  return runSelected(defaultScript.value);
}

async function runSelected(script: RunScript) {
  menuOpen.value = false;
  if (
    !props.workItem.worktreePath ||
    anotherRunActive.value
  ) return;
  const pullRequest = props.workItem.pullRequest;
  const checkout = pullRequest
    ? await checkoutPullRequest({
      projectId: props.project.id,
      projectPath: props.project.path,
      pullRequestNumber: pullRequest.number,
      headSha: pullRequest.headSha,
      headBranch: pullRequest.headBranch,
    })
    : { worktreePath: props.workItem.worktreePath };
  await run(props.project.id, script, checkout.worktreePath);
}

function closeMenu(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node)) menuOpen.value = false;
}

watch(
  () => props.project.id,
  (projectId) => void scriptStore.value.load(projectId),
  { immediate: true },
);
onMounted(() => document.addEventListener('pointerdown', closeMenu));
onBeforeUnmount(() => document.removeEventListener('pointerdown', closeMenu));
</script>

<template>
  <div ref="root" class="run-control">
    <button
      class="run-control__main"
      type="button"
      :disabled="mainDisabled"
      :title="
        !workItem.worktreePath
          ? `This branch must be checked out before it can ${actionName.toLowerCase()}`
          : defaultScript
            ? `${isActive ? 'Stop' : actionName} “${defaultScript.label}”`
            : `Configure ${actionName.toLowerCase()} scripts`
      "
      @click="runDefault"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path v-if="isActive" d="M4.5 4.5h7v7h-7z" />
        <path v-else d="m5.25 3.25 7 4.75-7 4.75v-9.5Z" />
      </svg>
      <span>{{ label }}</span>
    </button>
    <button
      class="run-control__menu-button"
      type="button"
      :aria-label="`Choose ${actionName} script`"
      :aria-expanded="menuOpen"
      :disabled="anotherRunActive"
      @click="menuOpen = !menuOpen"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4.5 6 3.5 3.5L11.5 6" /></svg>
    </button>

    <div v-if="menuOpen" class="run-menu">
      <button
        v-for="script in settings?.scripts ?? []"
        :key="script.id"
        type="button"
        :disabled="!workItem.worktreePath"
        @click="runSelected(script)"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path v-if="script.id === settings?.defaultScriptId" d="m3.5 8 3 3 6-6" />
        </svg>
        <span>{{ script.label }}</span>
      </button>
      <p v-if="settings?.scripts.length === 0">No scripts configured</p>
      <button class="run-menu__settings" type="button" @click="emit('settings', 'run')">
        Configure scripts…
      </button>
      <p v-if="error" class="run-menu__error">{{ error }}</p>
    </div>
  </div>
</template>

<style scoped>
.run-control {
  position: relative;
  display: flex;
}

.run-control > button {
  display: flex;
  align-items: center;
  gap: 5px;
  height: 24px;
  padding: 0 8px;
  font: inherit;
  font-size: 11px;
  color: var(--text-secondary);
  background: var(--surface-subtle);
  border: 1px solid var(--border-subtle);
  border-radius: 0;
}

.run-control > button:hover {
  color: var(--text-primary);
  background: var(--surface-hover);
}

.run-control > button:focus-visible,
.run-menu button:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 1px;
}

.run-control svg {
  width: 13px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.35;
}

.run-control__main {
  max-width: 150px;
  border-radius: 6px 0 0 6px !important;
}

.run-control__main span,
.run-menu button span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.run-control__menu-button {
  width: 22px;
  padding: 0 !important;
  justify-content: center;
  margin-left: -1px;
  border-radius: 0 6px 6px 0 !important;
}

.run-control__menu-button svg {
  width: 10px;
}

.run-control > button:disabled {
  opacity: 0.45;
}

.run-menu {
  position: absolute;
  z-index: 6;
  top: 29px;
  right: 0;
  width: 210px;
  padding: 5px;
  overflow: hidden;
  background: var(--surface-elevated);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  box-shadow: var(--shadow-elevated);
}

.run-menu button {
  display: grid;
  grid-template-columns: 16px minmax(0, 1fr);
  gap: 5px;
  align-items: center;
  width: 100%;
  min-height: 30px;
  padding: 5px 7px;
  font: inherit;
  font-size: 11px;
  color: var(--text-primary);
  text-align: left;
  background: transparent;
  border: 0;
  border-radius: 5px;
}

.run-menu button:hover {
  background: var(--surface-hover);
}

.run-menu button:disabled {
  color: var(--text-secondary);
}

.run-menu .run-menu__settings {
  display: block;
  margin-top: 4px;
  padding-left: 8px;
  color: var(--text-secondary);
  border-top: 1px solid var(--border-subtle);
  border-radius: 0 0 5px 5px;
}

.run-menu p {
  margin: 0;
  padding: 8px;
  font-size: 10px;
  color: var(--text-secondary);
}

.run-menu .run-menu__error {
  color: var(--danger);
}

@media (max-width: 680px) {
  .run-control > button {
    width: 26px;
    padding: 0;
    justify-content: center;
  }

  .run-control > button span {
    display: none;
  }

  .run-control__menu-button {
    width: 18px !important;
  }
}
</style>
