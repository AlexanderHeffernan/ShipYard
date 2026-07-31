import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { OpenApplicationInput, OpenSettings } from '../types/open';

export function getOpenSettings() {
  return invoke<OpenSettings>('get_open_settings');
}

export function saveOpenApplication(application: OpenApplicationInput) {
  return invoke<OpenSettings>('save_open_application', { application });
}

export function deleteOpenApplication(applicationId: string) {
  return invoke<OpenSettings>('delete_open_application', { applicationId });
}

export function openCheckout(applicationId: string, projectId: string, checkoutPath: string) {
  return invoke<void>('open_checkout', {
    request: { applicationId, projectId, checkoutPath },
  });
}

export async function chooseApplication() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: 'Choose an application',
    filters: [{ name: 'macOS applications', extensions: ['app'] }],
  });
  return typeof selected === 'string' ? selected : null;
}
