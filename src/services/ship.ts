import { invoke } from '@tauri-apps/api/core';
import type { RunSettings, ScriptInput } from '../types/run';
import type { Project, WorkItem } from '../types/projects';

export function getShipSettings(projectId: string) {
  return invoke<RunSettings>('get_ship_settings', { projectId });
}

export function saveShipScript(projectId: string, script: ScriptInput) {
  return invoke<RunSettings>('save_ship_script', { projectId, script });
}

export function deleteShipScript(projectId: string, scriptId: string) {
  return invoke<RunSettings>('delete_ship_script', { projectId, scriptId });
}

export function startShip(project: Project, item: WorkItem, scriptId: string) {
  return invoke<{ runId: string }>('ship_script', {
    request: {
      projectId: project.id,
      scriptId,
      workItemId: item.id,
      sourcePath: item.worktreePath,
      sourceBranch: item.branch,
      sourceSha: item.headSha,
      defaultBranch: project.defaultBranch,
    },
  });
}
