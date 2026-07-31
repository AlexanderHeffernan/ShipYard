import { ref } from 'vue';
import { deleteRunScript, getRunSettings, saveRunScript } from '../services/run';
import type { RunSettings, ScriptInput } from '../types/run';

const settingsByProject = ref<Record<string, RunSettings>>({});
const loadingProjects = ref(new Set<string>());

export function useRunScripts() {
  async function load(projectId: string) {
    loadingProjects.value.add(projectId);
    try {
      const settings = await getRunSettings(projectId);
      settingsByProject.value = { ...settingsByProject.value, [projectId]: settings };
      return settings;
    } finally {
      loadingProjects.value.delete(projectId);
    }
  }

  async function save(projectId: string, input: ScriptInput) {
    const settings = await saveRunScript(projectId, input);
    settingsByProject.value = { ...settingsByProject.value, [projectId]: settings };
    return settings;
  }

  async function remove(projectId: string, scriptId: string) {
    const settings = await deleteRunScript(projectId, scriptId);
    settingsByProject.value = { ...settingsByProject.value, [projectId]: settings };
    return settings;
  }

  return { settingsByProject, loadingProjects, load, save, remove };
}
