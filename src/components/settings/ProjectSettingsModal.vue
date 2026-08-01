<script setup lang="ts">
import { ExternalLink, Play, Plus, TerminalSquare, Trash2 } from '@lucide/vue';
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import AppButton from '../ui/AppButton.vue';
import OpenSettingsSection from './OpenSettingsSection.vue';
import SettingsModal from './SettingsModal.vue';
import type { Project } from '../../types/projects';
import type { RunScript, ScriptInput } from '../../types/run';
import { useRunScripts } from '../../composables/useRunScripts';

const props = withDefaults(defineProps<{ project: Project; initialSection?: 'open' | 'run' }>(), {
  initialSection: 'run',
});
const emit = defineEmits<{ close: [] }>();
const runScripts = useRunScripts();
const activeSection = ref<'open' | 'run'>(props.initialSection);
const selectedId = ref<string | null>(null);
const draft = ref<ScriptInput>(newDraft());
const adding = ref(false);
const saving = ref(false);
const deleting = ref(false);
const saved = ref(false);
const error = ref<string | null>(null);
let savedFeedbackTimer: number | undefined;
const scriptStore = computed(() => runScripts);
const settings = computed(() => scriptStore.value.settingsByProject.value[props.project.id]);
const scripts = computed(() => settings.value?.scripts ?? []);
const actionName = computed(() => 'Run');

watch(
  () => settings.value,
  (value) => {
    if (!value || selectedId.value || value.scripts.length === 0) return;
    const initial = value.scripts.find((script) => script.id === value.defaultScriptId) ?? value.scripts[0];
    if (initial) selectScript(initial);
  },
  { immediate: true },
);

watch(
  () => [props.project.id, activeSection.value] as const,
  ([projectId]) => {
    selectedId.value = null;
    error.value = null;
    if (activeSection.value === 'open') return;
    void scriptStore.value.load(projectId).catch((loadError) => (error.value = String(loadError)));
  },
  { immediate: true },
);

function newDraft(): ScriptInput {
  return {
    id: null,
    label: 'Run project',
    content: '#!/bin/zsh\nset -euo pipefail\n\n# Add your commands here\n',
    makeDefault: false,
  };
}

function selectScript(script: RunScript) {
  selectedId.value = script.id;
  draft.value = {
    id: script.id,
    label: script.label,
    content: script.content,
    makeDefault: settings.value?.defaultScriptId === script.id,
  };
  saved.value = false;
  error.value = null;
}

async function addScript() {
  adding.value = true;
  error.value = null;
  const input = {
    ...newDraft(),
    label: scripts.value.length === 0 ? `${actionName.value} project` : `${actionName.value} script ${scripts.value.length + 1}`,
    makeDefault: scripts.value.length === 0,
  };
  try {
    const updated = await scriptStore.value.save(props.project.id, input);
    const created = updated.scripts[updated.scripts.length - 1];
    if (created) selectScript(created);
  } catch (saveError) {
    error.value = String(saveError);
  } finally {
    adding.value = false;
  }
}

async function saveDraft() {
  saving.value = true;
  saved.value = false;
  error.value = null;
  try {
    const updated = await scriptStore.value.save(props.project.id, draft.value);
    const selected = updated.scripts.find((script) => script.id === draft.value.id);
    if (selected) selectScript(selected);
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
  if (!selectedId.value || !window.confirm(`Delete “${draft.value.label}”?`)) return;
  deleting.value = true;
  try {
    const updated = await scriptStore.value.remove(props.project.id, selectedId.value);
    selectedId.value = null;
    const next = updated.scripts.find((script) => script.id === updated.defaultScriptId) ?? updated.scripts[0];
    if (next) selectScript(next);
  } catch (deleteError) {
    error.value = String(deleteError);
  } finally {
    deleting.value = false;
  }
}

onBeforeUnmount(() => window.clearTimeout(savedFeedbackTimer));
</script>

<template>
  <SettingsModal
    title="Project Settings"
    :subtitle="`${project.name} · Only applies to this project.`"
    size="large"
    navigation-label="Project settings"
    @close="emit('close')"
  >
    <template #navigation>
      <button :aria-current="activeSection === 'open' ? 'page' : undefined" type="button" @click="activeSection = 'open'">
        <ExternalLink aria-hidden="true" />
        Open
      </button>
      <button :aria-current="activeSection === 'run' ? 'page' : undefined" type="button" @click="activeSection = 'run'">
        <Play aria-hidden="true" />
        Run
      </button>
    </template>

    <main v-if="activeSection !== 'open'" class="run-settings">
          <div class="run-settings__heading">
            <div>
              <h3>{{ actionName }} scripts</h3>
              <p>Commands available from this project’s {{ actionName }} button.</p>
            </div>
            <AppButton v-if="scripts.length > 0" type="button" :loading="adding" loading-label="Adding" @click="addScript">
              <Plus aria-hidden="true" />
              Add script
            </AppButton>
          </div>

          <div v-if="!settings" class="run-settings__loading">Loading scripts…</div>

          <div v-else-if="scripts.length === 0" class="run-settings__empty">
            <span class="run-settings__empty-icon"><TerminalSquare aria-hidden="true" /></span>
            <h4>No {{ actionName.toLowerCase() }} scripts yet</h4>
            <p>Add a script to {{ actionName.toLowerCase() }} this project without leaving Shipyard.</p>
            <AppButton variant="primary" type="button" :loading="adding" loading-label="Adding" @click="addScript">
              <Plus aria-hidden="true" />
              Add script
            </AppButton>
            <p v-if="error" class="script-editor__error">{{ error }}</p>
          </div>

          <div v-else class="run-settings__content">
            <aside class="script-list">
              <button
                v-for="script in scripts"
                :key="script.id"
                type="button"
                :class="{ 'script-list__active': selectedId === script.id }"
                @click="selectScript(script)"
              >
                <span>{{ script.label }}</span>
                <span v-if="settings?.defaultScriptId === script.id" class="script-list__default">Default</span>
              </button>
            </aside>

            <form class="script-editor" @submit.prevent="saveDraft">
              <label>
                <span>Label</span>
                <input v-model="draft.label" required placeholder="Development" />
              </label>
              <label class="script-editor__body">
                <span>Script</span>
                <textarea v-model="draft.content" required spellcheck="false"></textarea>
              </label>
              <label class="script-editor__default">
                <input v-model="draft.makeDefault" type="checkbox" />
                Use as the default {{ actionName }} script
              </label>
              <p v-if="selectedId" class="script-editor__path">
                {{ settings?.scripts.find((script) => script.id === selectedId)?.filePath }}
              </p>
              <p v-if="error" class="script-editor__error">{{ error }}</p>
              <div class="script-editor__actions">
                <AppButton v-if="selectedId" variant="danger" type="button" :loading="deleting" loading-label="Deleting" @click="deleteSelected">
                  <Trash2 aria-hidden="true" />
                  Delete
                </AppButton>
                <span></span>
                <AppButton variant="primary" type="submit" :loading="saving" loading-label="Saving" :success="saved" success-label="Saved">
                  Save script
                </AppButton>
              </div>
            </form>
          </div>
    </main>
    <OpenSettingsSection v-else />
  </SettingsModal>
</template>

<style scoped>
.run-settings h3 { margin: 0; font-size: 14px; font-weight: 550; }
.run-settings { display: flex; min-width: 0; flex: 1; flex-direction: column; padding: 22px; }
.run-settings__heading { display: flex; flex: 0 0 auto; align-items: start; justify-content: space-between; margin-bottom: 18px; }
.run-settings__heading p { margin: 5px 0 0; font-size: 11px; color: var(--text-secondary); }
.run-settings__loading { display: grid; min-height: 0; flex: 1; place-items: center; font-size: 11px; color: var(--text-secondary); }
.run-settings__empty { display: flex; min-height: 0; flex: 1; align-items: center; justify-content: center; flex-direction: column; padding-bottom: 22px; text-align: center; border: 1px solid var(--border-subtle); border-radius: 8px; }
.run-settings__empty-icon { display: grid; width: 46px; height: 46px; margin-bottom: 13px; place-items: center; color: var(--text-muted); background: var(--surface-subtle); border: 1px solid var(--border-subtle); border-radius: 12px; }
.run-settings__empty-icon svg { width: 22px; height: 22px; stroke-width: 1.4; }
.run-settings__empty h4 { margin: 0; font-size: 13px; font-weight: 550; }
.run-settings__empty p { max-width: 280px; margin: 7px 0 16px; font-size: 11px; line-height: 1.45; color: var(--text-secondary); }
.run-settings__empty .script-editor__error { margin-top: 12px; margin-bottom: 0; color: var(--danger); }
.run-settings__content { display: flex; min-height: 0; flex: 1; overflow: hidden; border: 1px solid var(--border-subtle); border-radius: 8px; }
.script-list { flex: 0 0 170px; padding: 7px; overflow-y: auto; background: var(--surface-subtle); border-right: 1px solid var(--border-subtle); }
.script-list button { display: flex; justify-content: space-between; align-items: center; width: 100%; min-height: 34px; padding: 7px 8px; overflow: hidden; font: inherit; font-size: 11px; color: var(--text-secondary); text-align: left; background: transparent; border: 0; border-radius: 5px; }
.script-list button:hover, .script-list__active { color: var(--text-primary) !important; background: var(--surface-hover) !important; }
.script-list button > span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.script-list__default { margin-left: 5px; font-size: 9px; color: var(--primary-hover); }
.script-editor { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 13px; padding: 15px; }
.script-editor label > span { display: block; margin-bottom: 6px; font-size: 10px; font-weight: 550; color: var(--text-secondary); text-transform: uppercase; letter-spacing: .04em; }
.script-editor input:not([type='checkbox']), .script-editor textarea { width: 100%; color: var(--text-primary); background: var(--surface-input); border: 1px solid var(--border-subtle); border-radius: 6px; outline: none; }
.script-editor input:not([type='checkbox']):focus, .script-editor textarea:focus { border-color: var(--focus-ring); }
.script-editor input:not([type='checkbox']) { height: 32px; padding: 0 9px; font: inherit; font-size: 12px; }
.script-editor__body { display: flex; min-height: 0; flex: 1; flex-direction: column; }
.script-editor textarea { min-height: 130px; flex: 1; padding: 10px; resize: none; font: 11px/1.55 ui-monospace, SFMono-Regular, Menlo, monospace; tab-size: 2; }
.script-editor__default { display: flex; align-items: center; gap: 7px; font-size: 11px; color: var(--text-secondary); }
.script-editor__path { margin: -5px 0 0; overflow: hidden; font: 9px ui-monospace, SFMono-Regular, Menlo, monospace; color: var(--text-muted); text-overflow: ellipsis; white-space: nowrap; }
.script-editor__error { margin: 0; font-size: 11px; color: var(--danger); }
.script-editor__actions { display: flex; align-items: center; justify-content: space-between; }
</style>
