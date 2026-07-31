<script setup lang="ts">
import { ExternalLink, Play, Plus, Ship, TerminalSquare, Trash2, X } from '@lucide/vue';
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import AppButton from '../ui/AppButton.vue';
import OpenSettingsSection from './OpenSettingsSection.vue';
import type { Project } from '../../types/projects';
import type { RunScript, ScriptInput } from '../../types/run';
import { useRunScripts } from '../../composables/useRunScripts';
import { useShipScripts } from '../../composables/useShipScripts';

const props = withDefaults(defineProps<{ project: Project; initialSection?: 'open' | 'run' | 'ship' }>(), {
  initialSection: 'run',
});
const emit = defineEmits<{ close: [] }>();
const runScripts = useRunScripts();
const shipScripts = useShipScripts();
const activeSection = ref<'open' | 'run' | 'ship'>(props.initialSection);
const selectedId = ref<string | null>(null);
const draft = ref<ScriptInput>(newDraft());
const saving = ref(false);
const error = ref<string | null>(null);
const scriptStore = computed(() => (activeSection.value === 'ship' ? shipScripts : runScripts));
const settings = computed(() => scriptStore.value.settingsByProject.value[props.project.id]);
const scripts = computed(() => settings.value?.scripts ?? []);
const actionName = computed(() => (activeSection.value === 'ship' ? 'Ship' : 'Run'));

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
  const ship = activeSection.value === 'ship';
  return {
    id: null,
    label: ship ? 'Ship work' : 'Run project',
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
  error.value = null;
}

async function addScript() {
  saving.value = true;
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
    saving.value = false;
  }
}

async function saveDraft() {
  saving.value = true;
  error.value = null;
  try {
    const updated = await scriptStore.value.save(props.project.id, draft.value);
    const selected = updated.scripts.find((script) => script.id === draft.value.id);
    if (selected) selectScript(selected);
  } catch (saveError) {
    error.value = String(saveError);
  } finally {
    saving.value = false;
  }
}

async function deleteSelected() {
  if (!selectedId.value || !window.confirm(`Delete “${draft.value.label}”?`)) return;
  try {
    const updated = await scriptStore.value.remove(props.project.id, selectedId.value);
    selectedId.value = null;
    const next = updated.scripts.find((script) => script.id === updated.defaultScriptId) ?? updated.scripts[0];
    if (next) selectScript(next);
  } catch (deleteError) {
    error.value = String(deleteError);
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close');
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown);
});
onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown));
</script>

<template>
  <div class="settings-backdrop" @mousedown.self="emit('close')">
    <section class="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <header class="settings-header">
        <div>
          <h2 id="settings-title">Project Settings</h2>
          <span>{{ project.name }}</span>
        </div>
        <AppButton variant="ghost" size="icon" type="button" aria-label="Close settings" @click="emit('close')">
          <X aria-hidden="true" />
        </AppButton>
      </header>

      <div class="settings-layout">
        <nav class="settings-nav" aria-label="Project settings">
          <button :class="{ 'settings-nav__active': activeSection === 'open' }" type="button" @click="activeSection = 'open'">
            <ExternalLink aria-hidden="true" />
            Open
          </button>
          <button :class="{ 'settings-nav__active': activeSection === 'run' }" type="button" @click="activeSection = 'run'">
            <Play aria-hidden="true" />
            Run
          </button>
          <button :class="{ 'settings-nav__active': activeSection === 'ship' }" type="button" @click="activeSection = 'ship'">
            <Ship aria-hidden="true" />
            Ship
          </button>
        </nav>

        <main v-if="activeSection !== 'open'" class="run-settings">
          <div class="run-settings__heading">
            <div>
              <h3>{{ actionName }} scripts</h3>
              <p>Commands available from this project’s {{ actionName }} button.</p>
            </div>
            <AppButton v-if="scripts.length > 0" type="button" :disabled="saving" @click="addScript">
              <Plus aria-hidden="true" />
              Add script
            </AppButton>
          </div>

          <div v-if="!settings" class="run-settings__loading">Loading scripts…</div>

          <div v-else-if="scripts.length === 0" class="run-settings__empty">
            <span class="run-settings__empty-icon"><TerminalSquare aria-hidden="true" /></span>
            <h4>No {{ actionName.toLowerCase() }} scripts yet</h4>
            <p>Add a script to {{ actionName.toLowerCase() }} this project without leaving Shipyard.</p>
            <AppButton variant="primary" type="button" :disabled="saving" @click="addScript">
              <Plus aria-hidden="true" />
              {{ saving ? 'Adding…' : 'Add script' }}
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
                <AppButton v-if="selectedId" variant="danger" type="button" @click="deleteSelected">
                  <Trash2 aria-hidden="true" />
                  Delete
                </AppButton>
                <span></span>
                <AppButton variant="primary" type="submit" :disabled="saving">
                  {{ saving ? 'Saving…' : 'Save script' }}
                </AppButton>
              </div>
            </form>
          </div>
        </main>
        <OpenSettingsSection v-else />
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-backdrop { position: fixed; z-index: 20; inset: 0; display: grid; padding: 32px; place-items: center; background: rgba(0, 0, 0, 0.52); backdrop-filter: blur(5px); }
.settings-modal { display: flex; width: min(820px, 90vw); height: min(570px, 84vh); flex-direction: column; overflow: hidden; background: #15161b; border: 1px solid rgba(255,255,255,.14); border-radius: 12px; box-shadow: 0 24px 80px rgba(0,0,0,.55); }
.settings-header { display: flex; flex: 0 0 58px; align-items: center; justify-content: space-between; padding: 0 15px 0 20px; border-bottom: 1px solid var(--border-subtle); }
.settings-header h2, .run-settings h3 { margin: 0; font-size: 14px; font-weight: 550; }
.settings-header span { display: block; margin-top: 3px; font-size: 11px; color: var(--text-secondary); }
.settings-layout { display: flex; min-height: 0; flex: 1; }
.settings-nav { flex: 0 0 160px; padding: 12px 9px; background: rgba(255,255,255,.018); border-right: 1px solid var(--border-subtle); }
.settings-nav button { display: flex; align-items: center; gap: 9px; width: 100%; height: 34px; padding: 0 10px; font: inherit; font-size: 12px; color: var(--text-secondary); background: transparent; border: 0; border-radius: 6px; }
.settings-nav button:hover, .settings-nav__active { color: var(--text-primary) !important; background: var(--surface-hover) !important; }
.settings-nav svg { width: 13px; height: 13px; stroke-width: 1.7; }
.run-settings { display: flex; min-width: 0; flex: 1; flex-direction: column; padding: 22px; }
.run-settings__heading { display: flex; flex: 0 0 auto; align-items: start; justify-content: space-between; margin-bottom: 18px; }
.run-settings__heading p { margin: 5px 0 0; font-size: 11px; color: var(--text-secondary); }
.run-settings__loading { display: grid; min-height: 0; flex: 1; place-items: center; font-size: 11px; color: var(--text-secondary); }
.run-settings__empty { display: flex; min-height: 0; flex: 1; align-items: center; justify-content: center; flex-direction: column; padding-bottom: 22px; text-align: center; border: 1px solid var(--border-subtle); border-radius: 8px; }
.run-settings__empty-icon { display: grid; width: 46px; height: 46px; margin-bottom: 13px; place-items: center; color: rgba(255,255,255,.46); background: rgba(255,255,255,.045); border: 1px solid var(--border-subtle); border-radius: 12px; }
.run-settings__empty-icon svg { width: 22px; height: 22px; stroke-width: 1.4; }
.run-settings__empty h4 { margin: 0; font-size: 13px; font-weight: 550; }
.run-settings__empty p { max-width: 280px; margin: 7px 0 16px; font-size: 11px; line-height: 1.45; color: var(--text-secondary); }
.run-settings__empty .script-editor__error { margin-top: 12px; margin-bottom: 0; color: #ff8f8f; }
.run-settings__content { display: flex; min-height: 0; flex: 1; overflow: hidden; border: 1px solid var(--border-subtle); border-radius: 8px; }
.script-list { flex: 0 0 170px; padding: 7px; overflow-y: auto; background: rgba(255,255,255,.018); border-right: 1px solid var(--border-subtle); }
.script-list button { display: flex; justify-content: space-between; align-items: center; width: 100%; min-height: 34px; padding: 7px 8px; overflow: hidden; font: inherit; font-size: 11px; color: var(--text-secondary); text-align: left; background: transparent; border: 0; border-radius: 5px; }
.script-list button:hover, .script-list__active { color: var(--text-primary) !important; background: var(--surface-hover) !important; }
.script-list button > span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.script-list__default { margin-left: 5px; font-size: 9px; color: #79aeff; }
.script-editor { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 13px; padding: 15px; }
.script-editor label > span { display: block; margin-bottom: 6px; font-size: 10px; font-weight: 550; color: var(--text-secondary); text-transform: uppercase; letter-spacing: .04em; }
.script-editor input:not([type='checkbox']), .script-editor textarea { width: 100%; color: var(--text-primary); background: #0e0f13; border: 1px solid rgba(255,255,255,.11); border-radius: 6px; outline: none; }
.script-editor input:not([type='checkbox']):focus, .script-editor textarea:focus { border-color: var(--focus-ring); }
.script-editor input:not([type='checkbox']) { height: 32px; padding: 0 9px; font: inherit; font-size: 12px; }
.script-editor__body { display: flex; min-height: 0; flex: 1; flex-direction: column; }
.script-editor textarea { min-height: 130px; flex: 1; padding: 10px; resize: none; font: 11px/1.55 ui-monospace, SFMono-Regular, Menlo, monospace; tab-size: 2; }
.script-editor__default { display: flex; align-items: center; gap: 7px; font-size: 11px; color: var(--text-secondary); }
.script-editor__path { margin: -5px 0 0; overflow: hidden; font: 9px ui-monospace, SFMono-Regular, Menlo, monospace; color: rgba(255,255,255,.32); text-overflow: ellipsis; white-space: nowrap; }
.script-editor__error { margin: 0; font-size: 11px; color: #ff8f8f; }
.script-editor__actions { display: flex; align-items: center; justify-content: space-between; }
</style>
