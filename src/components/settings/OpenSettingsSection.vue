<script setup lang="ts">
import { AppWindow, Plus, Trash2 } from '@lucide/vue';
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useOpenApplications } from '../../composables/useOpenApplications';
import { chooseApplication } from '../../services/open';
import type { OpenApplication, OpenApplicationInput } from '../../types/open';
import AppButton from '../ui/AppButton.vue';

const { settings, load, save, remove } = useOpenApplications();
const selectedId = ref<string | null>(null);
const draft = ref<OpenApplicationInput>(newDraft());
const saving = ref(false);
const deleting = ref(false);
const saved = ref(false);
const error = ref<string | null>(null);
let savedFeedbackTimer: number | undefined;
const applications = computed(() => settings.value?.applications ?? []);

watch(
  settings,
  (value) => {
    if (!value || selectedId.value || value.applications.length === 0) return;
    const initial = value.applications.find((app) => app.id === value.defaultApplicationId);
    selectApplication(initial ?? value.applications[0]!);
  },
  { immediate: true },
);

function newDraft(): OpenApplicationInput {
  return { id: null, label: '', kind: 'editor', appPath: '', makeDefault: false };
}

function selectApplication(application: OpenApplication) {
  selectedId.value = application.id;
  draft.value = {
    id: application.id,
    label: application.label,
    kind: application.kind,
    appPath: application.appPath,
    makeDefault: settings.value?.defaultApplicationId === application.id,
  };
  saved.value = false;
  error.value = null;
}

function addApplication() {
  selectedId.value = null;
  draft.value = { ...newDraft(), makeDefault: applications.value.length === 0 };
  error.value = null;
}

async function browse() {
  const appPath = await chooseApplication();
  if (!appPath) return;
  draft.value.appPath = appPath;
  if (!draft.value.label) {
    draft.value.label = appPath.split('/').pop()?.replace(/\.app$/, '') ?? '';
  }
}

async function saveDraft() {
  saving.value = true;
  saved.value = false;
  error.value = null;
  try {
    const updated = await save(draft.value);
    const selected = updated.applications.find((app) => app.id === draft.value.id);
    selectApplication(selected ?? updated.applications[updated.applications.length - 1]!);
    saved.value = true;
    window.clearTimeout(savedFeedbackTimer);
    savedFeedbackTimer = window.setTimeout(() => (saved.value = false), 1600);
  } catch (saveError) {
    error.value = String(saveError);
  } finally {
    saving.value = false;
  }
}

async function deleteSelected() {
  if (!selectedId.value || !window.confirm(`Remove “${draft.value.label}”?`)) return;
  deleting.value = true;
  try {
    const updated = await remove(selectedId.value);
    selectedId.value = null;
    draft.value = newDraft();
    const next = updated.applications.find((app) => app.id === updated.defaultApplicationId);
    if (next) selectApplication(next);
  } catch (deleteError) {
    error.value = String(deleteError);
  } finally {
    deleting.value = false;
  }
}

void load().catch((loadError) => (error.value = String(loadError)));
onBeforeUnmount(() => window.clearTimeout(savedFeedbackTimer));
</script>

<template>
  <main class="open-settings">
    <div class="open-settings__heading">
      <div>
        <h3>Open applications</h3>
        <p>Editors and terminals available for every project’s Open button.</p>
      </div>
      <AppButton v-if="applications.length > 0" type="button" @click="addApplication">
        <Plus aria-hidden="true" /> Add application
      </AppButton>
    </div>

    <div v-if="!settings" class="open-settings__loading">Loading applications…</div>
    <div v-else-if="applications.length === 0 && !draft.appPath" class="open-settings__empty">
      <span><AppWindow aria-hidden="true" /></span>
      <h4>No applications yet</h4>
      <p>Add any compatible macOS editor or terminal application.</p>
      <AppButton variant="primary" type="button" @click="browse">
        <Plus aria-hidden="true" /> Choose application
      </AppButton>
      <p v-if="error" class="application-editor__error">{{ error }}</p>
    </div>
    <div v-else class="open-settings__content">
      <aside class="application-list">
        <button
          v-for="application in applications"
          :key="application.id"
          type="button"
          :class="{ 'application-list__active': selectedId === application.id }"
          @click="selectApplication(application)"
        >
          <span>{{ application.label }}</span>
          <small>{{ application.available ? application.kind : 'Missing' }}</small>
        </button>
        <button v-if="!selectedId" class="application-list__active" type="button">New application</button>
      </aside>
      <form class="application-editor" @submit.prevent="saveDraft">
        <label><span>Label</span><input v-model="draft.label" required placeholder="VS Code" /></label>
        <label><span>Type</span><select v-model="draft.kind"><option value="editor">Editor</option><option value="terminal">Terminal</option></select></label>
        <label>
          <span>Application</span>
          <div class="application-editor__path-row">
            <input v-model="draft.appPath" required readonly placeholder="Choose a .app bundle" />
            <AppButton type="button" @click="browse">Choose…</AppButton>
          </div>
        </label>
        <label class="application-editor__default">
          <input v-model="draft.makeDefault" type="checkbox" /> Use as the default Open application
        </label>
        <p v-if="error" class="application-editor__error">{{ error }}</p>
        <div class="application-editor__actions">
          <AppButton v-if="selectedId" variant="danger" type="button" :loading="deleting" loading-label="Removing" @click="deleteSelected">
            <Trash2 aria-hidden="true" /> Remove
          </AppButton>
          <span></span>
          <AppButton variant="primary" type="submit" :loading="saving" loading-label="Saving" :success="saved" success-label="Saved">
            Save application
          </AppButton>
        </div>
      </form>
    </div>
  </main>
</template>

<style scoped>
.open-settings { display: flex; min-width: 0; flex: 1; flex-direction: column; padding: 22px; }
.open-settings h3 { margin: 0; font-size: 14px; font-weight: 550; }
.open-settings__heading { display: flex; align-items: start; justify-content: space-between; margin-bottom: 18px; }
.open-settings__heading p { margin: 5px 0 0; font-size: 11px; color: var(--text-secondary); }
.open-settings__loading { display: grid; flex: 1; place-items: center; font-size: 11px; color: var(--text-secondary); }
.open-settings__empty { display: flex; flex: 1; align-items: center; justify-content: center; flex-direction: column; text-align: center; border: 1px solid var(--border-subtle); border-radius: 8px; }
.open-settings__empty > span { display: grid; width: 46px; height: 46px; margin-bottom: 13px; place-items: center; color: var(--text-muted); background: var(--surface-subtle); border: 1px solid var(--border-subtle); border-radius: 12px; }
.open-settings__empty svg { width: 22px; }
.open-settings__empty h4 { margin: 0; font-size: 13px; font-weight: 550; }
.open-settings__empty p { margin: 7px 0 16px; font-size: 11px; color: var(--text-secondary); }
.open-settings__content { display: flex; min-height: 0; flex: 1; overflow: hidden; border: 1px solid var(--border-subtle); border-radius: 8px; }
.application-list { flex: 0 0 180px; padding: 7px; overflow-y: auto; background: var(--surface-subtle); border-right: 1px solid var(--border-subtle); }
.application-list button { display: flex; align-items: center; justify-content: space-between; gap: 5px; width: 100%; min-height: 34px; padding: 7px 8px; font: inherit; font-size: 11px; color: var(--text-secondary); background: transparent; border: 0; border-radius: 5px; }
.application-list button:hover, .application-list__active { color: var(--text-primary) !important; background: var(--surface-hover) !important; }
.application-list span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.application-list small { font-size: 9px; color: var(--text-secondary); text-transform: capitalize; }
.application-editor { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 15px; padding: 15px; }
.application-editor label > span { display: block; margin-bottom: 6px; font-size: 10px; font-weight: 550; color: var(--text-secondary); text-transform: uppercase; letter-spacing: .04em; }
.application-editor input:not([type='checkbox']), .application-editor select { width: 100%; height: 32px; padding: 0 9px; font: inherit; font-size: 12px; color: var(--text-primary); background: var(--surface-input); border: 1px solid var(--border-subtle); border-radius: 6px; outline: none; }
.application-editor input:focus, .application-editor select:focus { border-color: var(--focus-ring); }
.application-editor__path-row { display: flex; gap: 7px; }
.application-editor__path-row input { min-width: 0; flex: 1; font-size: 10px !important; }
.application-editor__default { display: flex; align-items: center; gap: 7px; font-size: 11px; color: var(--text-secondary); }
.application-editor__error { margin: 0; font-size: 11px; line-height: 1.4; color: var(--danger); }
.application-editor__actions { display: flex; align-items: center; justify-content: space-between; margin-top: auto; }
</style>
