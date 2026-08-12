import { ref } from 'vue';
import {
  chooseProjectDirectory,
  onProjectChanged,
  scanProject,
  startProjectWatch,
  stopProjectWatch,
} from '../services/projects';
import type { Project, ProjectCustomization, ScannedProject } from '../types/projects';
import {
  projectDefaultColor,
  readProjectIdentityStore,
  removeProjectCustomization,
  resolveProjectCustomization,
  saveProjectIdentityStore,
  setProjectCustomization,
} from '../utils/projectIdentity';

const STORAGE_KEY = 'shipyard.projectPaths';

function readSavedPaths() {
  try {
    const value = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]');
    return Array.isArray(value)
      ? value.filter((path): path is string => typeof path === 'string')
      : [];
  } catch {
    return [];
  }
}

function savePaths(projects: Project[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(projects.map((project) => project.path)));
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function useProjects() {
  const projects = ref<Project[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const refreshing = new Set<string>();
  const queued = new Set<string>();
  const versions = new Map<string, number>();
  let identityStore = readProjectIdentityStore();
  let stopListening: (() => void) | null = null;

  function withIdentity(project: ScannedProject): Project {
    const resolved = resolveProjectCustomization(identityStore, project.id, project.path);
    if (resolved.migrated) {
      identityStore = resolved.store;
      saveProjectIdentityStore(identityStore);
    }
    const color = resolved.customization.color ?? projectDefaultColor(project.id);
    return {
      ...project,
      color,
      colorOverride: resolved.customization.color,
      image: resolved.customization.image,
    };
  }

  function refreshAllProjects() {
    for (const project of projects.value) void refreshProject(project.id);
  }

  async function refreshProject(id: string) {
    if (refreshing.has(id)) {
      queued.add(id);
      return;
    }
    refreshing.add(id);
    do {
      queued.delete(id);
      const current = projects.value.find((project) => project.id === id);
      if (!current) break;
      const version = versions.get(id) ?? 0;
      try {
        const updated = withIdentity(await scanProject(current.path));
        const index = projects.value.findIndex((project) => project.id === id);
        if (index !== -1 && updated.id === id && versions.get(id) === version) {
          projects.value[index] = updated;
          versions.set(id, version + 1);
        }
      } catch {
        // Watch/scan errors are transient; retain the last good project state.
      }
    } while (queued.has(id));
    refreshing.delete(id);
  }

  async function watchProject(project: Project) {
    try {
      await startProjectWatch(project.path);
    } catch (watchError) {
      error.value = `Live refresh unavailable for ${project.name}: ${errorMessage(watchError)}`;
    }
  }

  async function loadProjects() {
    stopListening ??= await onProjectChanged((id) => void refreshProject(id));
    window.addEventListener('focus', refreshAllProjects);
    const paths = readSavedPaths();
    if (paths.length === 0) return;

    loading.value = true;
    error.value = null;
    const results = await Promise.allSettled(paths.map(scanProject));
    projects.value = results
      .filter(
        (result): result is PromiseFulfilledResult<ScannedProject> => result.status === 'fulfilled',
      )
      .map((result) => withIdentity(result.value));
    for (const project of projects.value) versions.set(project.id, 0);
    await Promise.all(projects.value.map(watchProject));

    const failures = results.filter((result) => result.status === 'rejected');
    if (failures.length > 0) {
      error.value = `${failures.length} project${failures.length === 1 ? '' : 's'} could not be loaded.`;
    }
    loading.value = false;
  }

  async function addProject() {
    const path = await chooseProjectDirectory();
    if (!path) return;

    loading.value = true;
    error.value = null;
    try {
      const project = withIdentity(await scanProject(path));
      const existingIndex = projects.value.findIndex((existing) => existing.id === project.id);
      versions.set(project.id, (versions.get(project.id) ?? 0) + 1);
      if (existingIndex === -1) {
        projects.value = [...projects.value, project];
      } else {
        projects.value[existingIndex] = project;
      }
      await watchProject(project);
      savePaths(projects.value);
    } catch (scanError) {
      error.value = errorMessage(scanError);
    } finally {
      loading.value = false;
    }
  }

  async function rescanProject(id: string) {
    const index = projects.value.findIndex((project) => project.id === id);
    if (index === -1) return;
    try {
      const scanned = withIdentity(await scanProject(projects.value[index].path));
      projects.value = projects.value.map((project) => (project.id === id ? scanned : project));
    } catch (scanError) {
      error.value = errorMessage(scanError);
    }
  }

  function removeProject(id: string) {
    const removed = projects.value.find((project) => project.id === id);
    projects.value = projects.value.filter((project) => project.id !== id);
    versions.set(id, (versions.get(id) ?? 0) + 1);
    queued.delete(id);
    void stopProjectWatch(id);
    identityStore = removeProjectCustomization(identityStore, id, removed?.path);
    saveProjectIdentityStore(identityStore);
    savePaths(projects.value);
  }

  function updateProjectIdentity(id: string, patch: Partial<ProjectCustomization>) {
    const current = projects.value.find((project) => project.id === id);
    if (!current) return false;

    const customization: ProjectCustomization = {
      color: patch.color === undefined ? current.colorOverride : patch.color,
      image: patch.image === undefined ? current.image : patch.image,
    };
    const nextStore = setProjectCustomization(identityStore, id, current.path, customization);
    if (!saveProjectIdentityStore(nextStore)) {
      error.value = 'Project identity could not be saved. Try a smaller image or check available storage.';
      return false;
    }

    identityStore = nextStore;
    projects.value = projects.value.map((project) => {
      if (project.id !== id) return project;
      const color = customization.color ?? projectDefaultColor(project.id);
      return {
        ...project,
        color,
        colorOverride: customization.color,
        image: customization.image,
      };
    });
    error.value = null;
    return true;
  }

  function disposeProjects() {
    stopListening?.();
    stopListening = null;
    window.removeEventListener('focus', refreshAllProjects);
    for (const project of projects.value) void stopProjectWatch(project.id);
  }

  return {
    projects,
    loading,
    error,
    loadProjects,
    addProject,
    rescanProject,
    removeProject,
    updateProjectIdentity,
    disposeProjects,
  };
}
