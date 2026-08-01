<script setup lang="ts">
import { Bot, Check, GitPullRequest, RefreshCw, X } from '@lucide/vue';
import { onMounted, ref } from 'vue';
import AppButton from '../ui/AppButton.vue';
import { getAgentConfiguration, getGitHubStatus, saveAgentSettings } from '../../services/settings';
import type { AgentConfiguration, AgentSettings, GitHubStatus } from '../../types/settings';

const emit = defineEmits<{ close: [] }>();
const section = ref<'agents' | 'github'>('agents');
const agentConfiguration = ref<AgentConfiguration | null>(null);
const agentDraft = ref<AgentSettings>({ preferredAgent: null, customName: '', customCommand: '' });
const github = ref<GitHubStatus | null>(null);
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const [agents, githubStatus] = await Promise.all([getAgentConfiguration(), getGitHubStatus()]);
    agentConfiguration.value = agents;
    agentDraft.value = { ...agents.settings };
    github.value = githubStatus;
  } catch (loadError) {
    error.value = String(loadError);
  } finally {
    loading.value = false;
  }
}

async function saveAgents() {
  saving.value = true;
  error.value = null;
  try {
    agentConfiguration.value = await saveAgentSettings(agentDraft.value);
    agentDraft.value = { ...agentConfiguration.value.settings };
  } catch (saveError) {
    error.value = String(saveError);
  } finally {
    saving.value = false;
  }
}

onMounted(refresh);
</script>

<template>
  <div class="settings-backdrop" @mousedown.self="emit('close')">
    <section class="settings-modal" role="dialog" aria-modal="true" aria-labelledby="app-settings-title">
      <header>
        <div>
          <h2 id="app-settings-title">ShipYard Settings</h2>
          <span>Configure the services ShipYard uses to get work shipped.</span>
        </div>
        <AppButton variant="ghost" size="icon" type="button" aria-label="Close settings" @click="emit('close')">
          <X aria-hidden="true" />
        </AppButton>
      </header>
      <div class="settings-layout">
        <nav aria-label="ShipYard settings">
          <button :class="{ active: section === 'agents' }" type="button" @click="section = 'agents'">
            <Bot aria-hidden="true" /> Agents
          </button>
          <button :class="{ active: section === 'github' }" type="button" @click="section = 'github'">
            <GitPullRequest aria-hidden="true" /> GitHub
          </button>
        </nav>
        <main>
          <div class="section-heading">
            <div>
              <h3>{{ section === 'agents' ? 'Coding agent' : 'GitHub' }}</h3>
              <p>{{ section === 'agents' ? 'ShipYard uses your preferred agent automatically while shipping.' : 'GitHub is the source of truth for pull requests and merge status.' }}</p>
            </div>
            <AppButton variant="ghost" size="small" type="button" :disabled="loading" @click="refresh">
              <RefreshCw aria-hidden="true" /> Refresh
            </AppButton>
          </div>

          <div v-if="loading && !agentConfiguration" class="loading">Detecting integrations…</div>

          <form v-else-if="section === 'agents' && agentConfiguration" class="agent-form" @submit.prevent="saveAgents">
            <label v-for="agent in agentConfiguration.agents" :key="agent.kind" class="agent-card" :class="{ selected: agentDraft.preferredAgent === agent.kind, unavailable: !agent.available && agent.kind !== 'custom' }">
              <input v-model="agentDraft.preferredAgent" type="radio" name="agent" :value="agent.kind" :disabled="!agent.available && agent.kind !== 'custom'" />
              <span class="agent-card__identity">
                <strong>{{ agent.label }}</strong>
                <small>{{ agent.available ? (agent.version || agent.executable) : 'Not detected' }}</small>
              </span>
              <Check v-if="agentDraft.preferredAgent === agent.kind" aria-hidden="true" />
            </label>

            <div v-if="agentDraft.preferredAgent === 'custom'" class="custom-fields">
              <label><span>Name</span><input v-model="agentDraft.customName" placeholder="My coding agent" /></label>
              <label><span>Command</span><input v-model="agentDraft.customCommand" required placeholder="/absolute/path/to/agent --flag" /></label>
              <small>The command receives ShipYard’s instructions on standard input and runs in the active checkout.</small>
            </div>

            <p v-if="!agentDraft.preferredAgent" class="notice">Choose an agent before shipping local work.</p>
            <p v-if="error" class="error">{{ error }}</p>
            <div class="form-actions"><AppButton variant="primary" type="submit" :disabled="saving">{{ saving ? 'Saving…' : 'Save agent' }}</AppButton></div>
          </form>

          <div v-else-if="section === 'github' && github" class="github-status" :class="{ connected: github.authenticated }">
            <span class="github-status__icon"><GitPullRequest aria-hidden="true" /></span>
            <div>
              <strong>{{ github.authenticated ? `Connected as ${github.account}` : 'GitHub is not connected' }}</strong>
              <p>{{ github.authenticated ? 'ShipYard can discover, create, and merge pull requests.' : github.error }}</p>
              <small v-if="github.version">{{ github.version }}</small>
            </div>
            <Check v-if="github.authenticated" class="github-status__check" aria-hidden="true" />
          </div>
        </main>
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-backdrop { position: fixed; z-index: 30; inset: 0; display: grid; padding: 32px; place-items: center; background: rgba(0,0,0,.55); backdrop-filter: blur(5px); }
.settings-modal { display: flex; width: min(720px, 90vw); height: min(500px, 82vh); flex-direction: column; overflow: hidden; background: #15161b; border: 1px solid rgba(255,255,255,.14); border-radius: 12px; box-shadow: 0 24px 80px rgba(0,0,0,.55); }
header { display: flex; flex: 0 0 64px; align-items: center; justify-content: space-between; padding: 0 15px 0 20px; border-bottom: 1px solid var(--border-subtle); }
h2, h3 { margin: 0; font-size: 14px; font-weight: 550; }
header span, main p { display: block; margin-top: 5px; font-size: 11px; color: var(--text-secondary); }
.settings-layout { display: flex; min-height: 0; flex: 1; }
nav { flex: 0 0 160px; padding: 12px 9px; background: rgba(255,255,255,.018); border-right: 1px solid var(--border-subtle); }
nav button { display: flex; align-items: center; gap: 9px; width: 100%; height: 34px; padding: 0 10px; font: inherit; font-size: 12px; color: var(--text-secondary); background: transparent; border: 0; border-radius: 6px; }
nav button:hover, nav button.active { color: var(--text-primary); background: var(--surface-hover); }
nav svg { width: 14px; height: 14px; }
main { min-width: 0; flex: 1; padding: 24px; overflow-y: auto; }
.section-heading { display: flex; align-items: start; justify-content: space-between; margin-bottom: 20px; }
.loading { display: grid; height: 220px; place-items: center; font-size: 11px; color: var(--text-secondary); }
.agent-form { display: flex; flex-direction: column; gap: 9px; }
.agent-card { display: flex; align-items: center; gap: 12px; min-height: 58px; padding: 0 14px; background: rgba(255,255,255,.018); border: 1px solid var(--border-subtle); border-radius: 8px; }
.agent-card:hover, .agent-card.selected { background: rgba(85,137,255,.07); border-color: rgba(85,137,255,.38); }
.agent-card.unavailable { opacity: .52; }
.agent-card__identity { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 4px; }
.agent-card strong, .github-status strong { font-size: 12px; font-weight: 550; }
.agent-card small, .github-status small, .custom-fields small { overflow: hidden; font: 9px ui-monospace, SFMono-Regular, Menlo, monospace; color: var(--text-secondary); text-overflow: ellipsis; white-space: nowrap; }
.agent-card > svg, .github-status__check { width: 15px; color: #64cf8c; }
.custom-fields { display: grid; grid-template-columns: 1fr 2fr; gap: 10px; padding: 12px 0 4px 28px; }
.custom-fields label span { display: block; margin-bottom: 5px; font-size: 9px; color: var(--text-secondary); text-transform: uppercase; }
.custom-fields input { width: 100%; height: 32px; padding: 0 9px; font: 11px ui-monospace, SFMono-Regular, Menlo, monospace; color: var(--text-primary); background: #0e0f13; border: 1px solid var(--border-subtle); border-radius: 6px; }
.custom-fields small { grid-column: 1 / -1; white-space: normal; }
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
