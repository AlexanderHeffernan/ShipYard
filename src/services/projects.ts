import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type {
  DeleteWorkItemRequest,
  DeletionPlan,
  DeletionResult,
  ScannedProject,
  WorkItemDiff,
  WorkItemDiffRequest,
} from '../types/projects';

export async function chooseProjectDirectory() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: 'Add a Git project',
  });

  return typeof selected === 'string' ? selected : null;
}

export function scanProject(path: string) {
  return invoke<ScannedProject>('scan_project', { path });
}

export function getWorkItemDiff(request: WorkItemDiffRequest) {
  return invoke<WorkItemDiff>('get_work_item_diff', { request });
}

export function inspectWorkItemDeletion(request: DeleteWorkItemRequest) {
  return invoke<DeletionPlan>('inspect_work_item_deletion', { request });
}

export function deleteWorkItem(request: DeleteWorkItemRequest, confirmedPlan: DeletionPlan) {
  return invoke<DeletionResult>('delete_work_item', { request, confirmedPlan });
}

export function startProjectWatch(path: string) {
  return invoke<void>('start_project_watch', { path });
}

export function stopProjectWatch(projectId: string) {
  return invoke<void>('stop_project_watch', { projectId });
}

export function onProjectChanged(callback: (projectId: string) => void) {
  return listen<string>('project-changed', (event) => callback(event.payload));
}
