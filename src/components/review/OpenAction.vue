<script setup lang="ts">
import { openUrl } from '@tauri-apps/plugin-opener';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useOpenApplications } from '../../composables/useOpenApplications';
import type { Project, WorkItem } from '../../types/projects';
import type { OpenApplication } from '../../types/open';

const props = defineProps<{ project: Project; workItem: WorkItem }>();
const emit = defineEmits<{ settings: [] }>();
const { settings, load, launch } = useOpenApplications();
const root = ref<HTMLElement>();
const menuOpen = ref(false);
const error = ref<string | null>(null);
const defaultApplication = computed(() =>
  settings.value?.applications.find(
    (application) => application.id === settings.value?.defaultApplicationId,
  ),
);
const checkoutPath = computed(() => props.workItem.worktreePath);
const label = computed(() =>
  defaultApplication.value ? defaultApplication.value.label : 'Set up Open',
);
const disabledReason = computed(() => {
  if (!checkoutPath.value) return 'This branch needs to be checked out before it can be opened';
  if (defaultApplication.value && !defaultApplication.value.available) {
    return `${defaultApplication.value.label} is no longer available; update it in Settings → Open`;
  }
  return null;
});

async function openDefault() {
  if (!defaultApplication.value) return emit('settings');
  await openWith(defaultApplication.value);
}

async function openWith(application: OpenApplication) {
  if (!checkoutPath.value || !application.available) return;
  error.value = null;
  try {
    await launch(application.id, props.project.id, checkoutPath.value);
    menuOpen.value = false;
  } catch (launchError) {
    error.value = String(launchError);
    menuOpen.value = true;
  }
}

async function openPullRequest() {
  const url = props.workItem.pullRequest?.url;
  if (!url) return;
  error.value = null;
  try {
    await openUrl(url);
    menuOpen.value = false;
  } catch (openError) {
    error.value = String(openError);
  }
}

function closeMenu(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node)) menuOpen.value = false;
}

void load().catch((loadError) => (error.value = String(loadError)));
onMounted(() => document.addEventListener('pointerdown', closeMenu));
onBeforeUnmount(() => document.removeEventListener('pointerdown', closeMenu));
</script>

<template>
  <div ref="root" class="open-control">
    <button
      class="open-control__main"
      type="button"
      :disabled="!!disabledReason"
      :title="disabledReason ?? label"
      @click="openDefault"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M8.75 3.25h4v4m-.25-3.5-6 6M11.5 9v3.5h-8v-8H7" />
      </svg>
      <span>{{ label }}</span>
    </button>
    <button
      class="open-control__menu-button"
      type="button"
      aria-label="Choose application"
      :aria-expanded="menuOpen"
      @click="menuOpen = !menuOpen"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4.5 6 3.5 3.5L11.5 6" /></svg>
    </button>

    <div v-if="menuOpen" class="open-menu">
      <button v-if="workItem.pullRequest" type="button" @click="openPullRequest">
        <span>Open PR on GitHub</span>
        <small>#{{ workItem.pullRequest.number }}</small>
      </button>
      <p v-if="!checkoutPath" class="open-menu__notice">
        This branch needs to be checked out before it can be opened.
      </p>
      <button
        v-for="application in settings?.applications ?? []"
        :key="application.id"
        type="button"
        :disabled="!checkoutPath || !application.available"
        @click="openWith(application)"
      >
        <span>{{ application.label }}</span>
        <small>{{ application.available ? application.kind : 'Unavailable' }}</small>
      </button>
      <p v-if="settings?.applications.length === 0">No applications configured</p>
      <button class="open-menu__settings" type="button" @click="emit('settings')">
        Configure applications…
      </button>
      <p v-if="error" class="open-menu__error">{{ error }}</p>
    </div>
  </div>
</template>

<style scoped>
.open-control { position: relative; display: flex; }
.open-control > button { display: flex; align-items: center; gap: 5px; height: 24px; padding: 0 8px; font: inherit; font-size: 11px; color: var(--text-secondary); background: var(--surface-subtle); border: 1px solid var(--border-subtle); border-radius: 0; }
.open-control > button:hover:not(:disabled) { color: var(--text-primary); background: var(--surface-hover); }
.open-control > button:focus-visible, .open-menu button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
.open-control svg { width: 13px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.35; }
.open-control__main { max-width: 145px; border-radius: 6px 0 0 6px !important; }
.open-control__main span, .open-menu button span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.open-control__menu-button { width: 22px; padding: 0 !important; justify-content: center; margin-left: -1px; border-radius: 0 6px 6px 0 !important; }
.open-control__menu-button svg { width: 10px; }
.open-control > button:disabled { opacity: .45; }
.open-menu { position: absolute; z-index: 6; top: 29px; left: 0; width: 230px; padding: 5px; overflow: hidden; background: var(--surface-elevated); border: 1px solid var(--border-strong); border-radius: 8px; box-shadow: var(--shadow-elevated); }
.open-menu button { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; min-height: 30px; padding: 5px 7px; font: inherit; font-size: 11px; color: var(--text-primary); text-align: left; background: transparent; border: 0; border-radius: 5px; }
.open-menu button:hover:not(:disabled) { background: var(--surface-hover); }
.open-menu button:disabled { color: var(--text-secondary); }
.open-menu small { flex: 0 0 auto; font-size: 9px; color: var(--text-secondary); text-transform: capitalize; }
.open-menu .open-menu__settings { display: block; margin-top: 4px; padding-left: 8px; color: var(--text-secondary); border-top: 1px solid var(--border-subtle); border-radius: 0 0 5px 5px; }
.open-menu p { margin: 0; padding: 8px; font-size: 10px; color: var(--text-secondary); }
.open-menu .open-menu__notice { line-height: 1.4; color: var(--warning); }
.open-menu .open-menu__error { line-height: 1.4; color: var(--danger); }
@media (max-width: 680px) { .open-control > button { width: 26px; padding: 0; justify-content: center; } .open-control > button span { display: none; } .open-control__menu-button { width: 18px !important; } }
</style>
