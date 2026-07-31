import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { ScannedProject } from '../types/projects';

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
