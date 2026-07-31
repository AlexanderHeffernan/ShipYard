import { ref } from 'vue';
import { deleteShipScript, getShipSettings, saveShipScript } from '../services/ship';
import type { RunSettings, ScriptInput } from '../types/run';

const settingsByProject = ref<Record<string, RunSettings>>({});

export function useShipScripts() {
  async function load(projectId: string) {
    const settings = await getShipSettings(projectId);
    settingsByProject.value = { ...settingsByProject.value, [projectId]: settings };
    return settings;
  }

  async function save(projectId: string, input: ScriptInput) {
    const settings = await saveShipScript(projectId, input);
    settingsByProject.value = { ...settingsByProject.value, [projectId]: settings };
    return settings;
  }

  async function remove(projectId: string, scriptId: string) {
    const settings = await deleteShipScript(projectId, scriptId);
    settingsByProject.value = { ...settingsByProject.value, [projectId]: settings };
    return settings;
  }

  return { settingsByProject, load, save, remove };
}
