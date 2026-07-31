import { ref } from 'vue';
import {
  chooseProjectDirectory,
  onProjectChanged,
  scanProject,
  startProjectWatch,
  stopProjectWatch,
} from '../services/projects';
import type { Project, ScannedProject } from '../types/projects';

const STORAGE_KEY = 'shipyard.projectPaths';
const PROJECT_COLORS = ['#8b5cf6', '#3395ff', '#29c76f', '#ffbd2e', '#ff4f8b', '#9699a1'];

function projectColor(id: string) {
  let hash = 0;
  for (const character of id) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  return PROJECT_COLORS[hash % PROJECT_COLORS.length];
}

function withColor(project: ScannedProject): Project {
  return { ...project, color: projectColor(project.id) };
}

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
  let stopListening: (() => void) | null = null;

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
        const updated = withColor(await scanProject(current.path));
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
    const paths = readSavedPaths();
    if (paths.length === 0) return;

    loading.value = true;
    error.value = null;
    const results = await Promise.allSettled(paths.map(scanProject));
    projects.value = results
      .filter(
        (result): result is PromiseFulfilledResult<ScannedProject> => result.status === 'fulfilled',
      )
      .map((result) => withColor(result.value));
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
      const project = withColor(await scanProject(path));
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
      const scanned = withColor(await scanProject(projects.value[index].path));
      projects.value = projects.value.map((project) => (project.id === id ? scanned : project));
    } catch (scanError) {
      error.value = errorMessage(scanError);
    }
  }

  function removeProject(id: string) {
    projects.value = projects.value.filter((project) => project.id !== id);
    versions.set(id, (versions.get(id) ?? 0) + 1);
    queued.delete(id);
    void stopProjectWatch(id);
    savePaths(projects.value);
  }

  function disposeProjects() {
    stopListening?.();
    stopListening = null;
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
    disposeProjects,
  };
}
