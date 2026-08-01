<script setup lang="ts">
import { Bot, Check, GitPullRequest, RefreshCw } from '@lucide/vue';
import { onBeforeUnmount, onMounted, ref } from 'vue';
import AppButton from '../ui/AppButton.vue';
import SettingsModal from './SettingsModal.vue';
import { getAgentConfiguration, getGitHubStatus, saveAgentSettings } from '../../services/settings';
import type { AgentConfiguration, AgentSettings, GitHubStatus } from '../../types/settings';

const emit = defineEmits<{ close: [] }>();
const section = ref<'agents' | 'github'>('agents');
const agentConfiguration = ref<AgentConfiguration | null>(null);
const agentDraft = ref<AgentSettings>({ preferredAgent: null });
const github = ref<GitHubStatus | null>(null);
const loading = ref(false);
const saving = ref(false);
const refreshed = ref(false);
const saved = ref(false);
const error = ref<string | null>(null);
let refreshFeedbackTimer: number | undefined;
let saveFeedbackTimer: number | undefined;

async function refresh(force = false) {
  loading.value = true;
  refreshed.value = false;
  error.value = null;
  try {
    const [agents, githubStatus] = await Promise.all([
      getAgentConfiguration(force),
      getGitHubStatus(force),
    ]);
    agentConfiguration.value = agents;
    agentDraft.value = { ...agents.settings };
    github.value = githubStatus;
    if (force) {
      refreshed.value = true;
      window.clearTimeout(refreshFeedbackTimer);
      refreshFeedbackTimer = window.setTimeout(() => (refreshed.value = false), 1600);
    }
  } catch (loadError) {
    error.value = String(loadError);
  } finally {
    loading.value = false;
  }
}

async function saveAgents() {
  saving.value = true;
  saved.value = false;
  error.value = null;
  try {
    agentConfiguration.value = await saveAgentSettings(agentDraft.value);
    agentDraft.value = { ...agentConfiguration.value.settings };
    saved.value = true;
    window.clearTimeout(saveFeedbackTimer);
    saveFeedbackTimer = window.setTimeout(() => (saved.value = false), 1600);
  } catch (saveError) {
    error.value = String(saveError);
  } finally {
    saving.value = false;
  }
}

onMounted(() => void refresh());
onBeforeUnmount(() => {
  window.clearTimeout(refreshFeedbackTimer);
  window.clearTimeout(saveFeedbackTimer);
});
</script>

<template>
  <SettingsModal
    title="ShipYard Settings"
    subtitle="App-wide integrations used across every project."
    navigation-label="ShipYard settings"
    @close="emit('close')"
  >
    <template #navigation>
      <button :aria-current="section === 'agents' ? 'page' : undefined" type="button" @click="section = 'agents'">
        <Bot aria-hidden="true" /> Agents
      </button>
      <button :aria-current="section === 'github' ? 'page' : undefined" type="button" @click="section = 'github'">
        <GitPullRequest aria-hidden="true" /> GitHub
      </button>
    </template>

    <main>
      <div class="section-heading">
        <div>
          <h3>{{ section === 'agents' ? 'Coding agent' : 'GitHub' }}</h3>
          <p>{{ section === 'agents' ? 'ShipYard uses your preferred agent automatically while shipping.' : 'GitHub is the source of truth for pull requests and merge status.' }}</p>
        </div>
        <AppButton
          variant="ghost"
          size="small"
          type="button"
          :loading="loading"
          loading-label="Refreshing"
          :success="refreshed"
          success-label="Refreshed"
          @click="refresh(true)"
        >
          <RefreshCw aria-hidden="true" /> Refresh
        </AppButton>
      </div>

      <div v-if="section === 'agents' && !agentConfiguration" class="loading">Detecting coding agents…</div>

      <form v-else-if="section === 'agents' && agentConfiguration" class="agent-form" @submit.prevent="saveAgents">
        <label v-for="agent in agentConfiguration.agents" :key="agent.kind" class="agent-card" :class="{ selected: agentDraft.preferredAgent === agent.kind, unavailable: !agent.available }">
          <input v-model="agentDraft.preferredAgent" type="radio" name="agent" :value="agent.kind" :disabled="!agent.available" @change="saved = false" />
          <span class="agent-card__identity">
            <strong>{{ agent.label }}</strong>
            <small>{{ agent.available ? (agent.version || agent.executable) : 'Not detected' }}</small>
          </span>
          <Check v-if="agentDraft.preferredAgent === agent.kind" aria-hidden="true" />
        </label>

        <p v-if="!agentDraft.preferredAgent" class="notice">Choose an agent before shipping local work.</p>
        <div class="form-actions">
          <AppButton
            variant="primary"
            type="submit"
            :disabled="!agentDraft.preferredAgent"
            :loading="saving"
            loading-label="Saving"
            :success="saved"
            success-label="Saved"
          >
            Save agent
          </AppButton>
        </div>
      </form>

      <div v-else-if="section === 'github' && !github" class="loading">Checking GitHub…</div>
      <div v-else-if="section === 'github' && github" class="github-status" :class="{ connected: github.authenticated }">
        <span class="github-status__icon"><GitPullRequest aria-hidden="true" /></span>
        <div>
          <strong>{{ github.authenticated ? `Connected as ${github.account}` : 'GitHub is not connected' }}</strong>
          <p>{{ github.authenticated ? 'ShipYard can discover, create, and merge pull requests.' : github.error }}</p>
          <small v-if="github.version">{{ github.version }}</small>
        </div>
        <Check v-if="github.authenticated" class="github-status__check" aria-hidden="true" />
      </div>

      <p v-if="error" class="error">{{ error }}</p>
    </main>
  </SettingsModal>
</template>

<style scoped>
h2, h3 { margin: 0; font-size: 14px; font-weight: 550; }
main p { display: block; margin-top: 5px; font-size: 11px; color: var(--text-secondary); }
main { min-width: 0; flex: 1; padding: 24px; overflow-y: auto; }
.section-heading { display: flex; align-items: start; justify-content: space-between; margin-bottom: 20px; }
.loading { display: grid; height: 220px; place-items: center; font-size: 11px; color: var(--text-secondary); }
.agent-form { display: flex; flex-direction: column; gap: 9px; }
.agent-card { display: flex; align-items: center; gap: 12px; min-height: 58px; padding: 0 14px; background: rgba(255,255,255,.018); border: 1px solid var(--border-subtle); border-radius: 8px; }
.agent-card:hover, .agent-card.selected { background: rgba(85,137,255,.07); border-color: rgba(85,137,255,.38); }
.agent-card.unavailable { opacity: .52; }
.agent-card__identity { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 4px; }
.agent-card strong, .github-status strong { font-size: 12px; font-weight: 550; }
.agent-card small, .github-status small { overflow: hidden; font: 9px ui-monospace, SFMono-Regular, Menlo, monospace; color: var(--text-secondary); text-overflow: ellipsis; white-space: nowrap; }
.agent-card > svg, .github-status__check { width: 15px; color: #64cf8c; }
.notice, .error { margin: 4px 0 0; font-size: 10px; color: var(--text-secondary); }
.error { color: #ff8f8f; }
.form-actions { display: flex; justify-content: end; margin-top: 5px; }
.github-status { display: flex; align-items: center; gap: 14px; min-height: 84px; padding: 14px; border: 1px solid var(--border-subtle); border-radius: 9px; }
.github-status.connected { border-color: rgba(100,207,140,.25); background: rgba(100,207,140,.04); }
.github-status__icon { display: grid; width: 42px; height: 42px; flex: 0 0 auto; place-items: center; background: rgba(255,255,255,.05); border-radius: 10px; }
.github-status__icon svg { width: 21px; }
.github-status > div { min-width: 0; flex: 1; }
.github-status p { margin-bottom: 5px; }
</style>
