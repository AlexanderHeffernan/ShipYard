import { invoke } from '@tauri-apps/api/core';
import type { AgentConfiguration, AgentSettings, GitHubStatus } from '../types/settings';

export function getGitHubStatus() {
  return invoke<GitHubStatus>('get_github_status');
}

export function getAgentConfiguration() {
  return invoke<AgentConfiguration>('get_agent_configuration');
}

export function saveAgentSettings(settings: AgentSettings) {
  return invoke<AgentConfiguration>('save_agent_settings', { settings });
}
