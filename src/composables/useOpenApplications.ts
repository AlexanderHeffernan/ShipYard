import { ref } from 'vue';
import {
  deleteOpenApplication,
  getOpenSettings,
  openCheckout,
  saveOpenApplication,
} from '../services/open';
import type { OpenApplicationInput, OpenSettings } from '../types/open';

const settings = ref<OpenSettings | null>(null);
let loading: Promise<OpenSettings> | null = null;

export function useOpenApplications() {
  function load() {
    if (loading) return loading;
    loading = getOpenSettings()
      .then((value) => (settings.value = value))
      .finally(() => (loading = null));
    return loading;
  }

  async function save(input: OpenApplicationInput) {
    settings.value = await saveOpenApplication(input);
    return settings.value;
  }

  async function remove(applicationId: string) {
    settings.value = await deleteOpenApplication(applicationId);
    return settings.value;
  }

  async function launch(applicationId: string, projectId: string, checkoutPath: string) {
    await openCheckout(applicationId, projectId, checkoutPath);
  }

  return { settings, load, save, remove, launch };
}
