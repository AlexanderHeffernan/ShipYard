import { invoke } from '@tauri-apps/api/core';
import type { RunSettings, ScriptInput } from '../types/run';

export function getRunSettings(projectId: string) {
  return invoke<RunSettings>('get_run_settings', { projectId });
}

export function saveRunScript(projectId: string, script: ScriptInput) {
  return invoke<RunSettings>('save_run_script', { projectId, script });
}

export function deleteRunScript(projectId: string, scriptId: string) {
  return invoke<RunSettings>('delete_run_script', { projectId, scriptId });
}

export function startRun(projectId: string, scriptId: string, workingDirectory: string) {
  return invoke<{ runId: string }>('run_script', {
    request: { projectId, scriptId, workingDirectory },
  });
}

export function cancelRun(runId: string) {
  return invoke<void>('cancel_run', { runId });
}

export function writeRunInput(runId: string, input: string) {
  return invoke<void>('write_run_input', { runId, input });
}

export function resizeRunTerminal(runId: string, columns: number, rows: number) {
  return invoke<void>('resize_run_terminal', {
    runId,
    size: { columns, rows },
  });
}
