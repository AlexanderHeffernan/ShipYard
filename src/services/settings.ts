import { invoke } from '@tauri-apps/api/core';
import type { AgentConfiguration, AgentSettings, GitHubStatus } from '../types/settings';

let cachedAgentConfiguration: AgentConfiguration | null = null;
let cachedGitHubStatus: GitHubStatus | null = null;

export async function getGitHubStatus(force = false) {
  if (!force && cachedGitHubStatus) return cachedGitHubStatus;
  cachedGitHubStatus = await invoke<GitHubStatus>('get_github_status');
  return cachedGitHubStatus;
}

export async function getAgentConfiguration(force = false) {
  if (!force && cachedAgentConfiguration) return cachedAgentConfiguration;
  cachedAgentConfiguration = await invoke<AgentConfiguration>('get_agent_configuration');
  return cachedAgentConfiguration;
}

export async function saveAgentSettings(settings: AgentSettings) {
  cachedAgentConfiguration = await invoke<AgentConfiguration>('save_agent_settings', { settings });
  return cachedAgentConfiguration;
}
