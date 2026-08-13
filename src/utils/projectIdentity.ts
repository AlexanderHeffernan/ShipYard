import type { ProjectCustomization, ProjectImage } from '../types/projects';

export const PROJECT_IDENTITY_STORAGE_KEY = 'shipyard.projectIdentity.v1';
export const LEGACY_PROJECT_IDENTITY_STORAGE_KEY = 'shipyard.projectIdentity';
export const MAX_PROJECT_IMAGE_BYTES = 2 * 1024 * 1024;

export const PROJECT_COLOR_PRESETS = [
  { name: 'Sunset', value: '#fb771f' },
  { name: 'Coral', value: '#ff4d6d' },
  { name: 'Berry', value: '#d946ef' },
  { name: 'Violet', value: '#8b5cf6' },
  { name: 'Indigo', value: '#6366f1' },
  { name: 'Sky', value: '#0ea5e9' },
  { name: 'Mint', value: '#14b8a6' },
  { name: 'Leaf', value: '#22c55e' },
  { name: 'Gold', value: '#eab308' },
] as const;

type StoredProjectCustomization = ProjectCustomization & {
  path: string;
};

export type ProjectIdentityStore = {
  version: 1;
  projects: Record<string, StoredProjectCustomization>;
};

const SUPPORTED_IMAGE_TYPES = new Set([
  'image/avif',
  'image/gif',
  'image/jpeg',
  'image/png',
  'image/webp',
]);

function emptyStore(): ProjectIdentityStore {
  return { version: 1, projects: {} };
}

function storageOrNull(storage?: Storage | null) {
  if (storage !== undefined) return storage;
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

function parseJson(value: string | null) {
  if (!value) return null;
  try {
    return JSON.parse(value) as unknown;
  } catch {
    return null;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function normalizeHexColor(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const match = value.trim().match(/^#([\da-f]{3}|[\da-f]{6})$/i);
  if (!match) return null;
  const hex = match[1].toLowerCase();
  return `#${hex.length === 3 ? hex.split('').map((character) => character + character).join('') : hex}`;
}

export function projectDefaultColor(id: string) {
  let hash = 0;
  for (const character of id) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  return PROJECT_COLOR_PRESETS[hash % PROJECT_COLOR_PRESETS.length].value;
}

function isSafeImageDataUrl(value: string) {
  const match = value.match(/^data:(image\/[a-z0-9.+-]+);base64,([a-z0-9+/=\s]+)$/i);
  return !!match && SUPPORTED_IMAGE_TYPES.has(match[1].toLowerCase()) && match[2].length > 0;
}

function sanitizeImage(value: unknown): ProjectImage | null {
  if (!isRecord(value) || typeof value.dataUrl !== 'string' || !isSafeImageDataUrl(value.dataUrl)) {
    return null;
  }
  if (value.dataUrl.length > Math.ceil((MAX_PROJECT_IMAGE_BYTES * 4) / 3) + 512) return null;

  const type = typeof value.type === 'string' ? value.type.toLowerCase() : '';
  if (!SUPPORTED_IMAGE_TYPES.has(type)) return null;
  const size = typeof value.size === 'number' && Number.isFinite(value.size) ? value.size : 0;
  if (size < 0 || size > MAX_PROJECT_IMAGE_BYTES) return null;

  return {
    dataUrl: value.dataUrl,
    name: typeof value.name === 'string' && value.name.trim() ? value.name : 'Project image',
    type,
    size,
    width: typeof value.width === 'number' && Number.isFinite(value.width) ? value.width : 0,
    height: typeof value.height === 'number' && Number.isFinite(value.height) ? value.height : 0,
  };
}

function sanitizeCustomization(value: unknown): ProjectCustomization {
  if (!isRecord(value)) return { color: null, image: null };
  return {
    color: normalizeHexColor(value.color),
    image: sanitizeImage(value.image),
  };
}

function parseEntries(value: unknown): Record<string, StoredProjectCustomization> {
  if (!isRecord(value)) return {};
  const entries: Record<string, StoredProjectCustomization> = {};

  for (const [id, rawEntry] of Object.entries(value)) {
    const legacyColor = typeof rawEntry === 'string' ? rawEntry : null;
    const entry = isRecord(rawEntry) ? rawEntry : {};
    const customization = sanitizeCustomization({
      color: legacyColor ?? entry.color,
      image: entry.image,
    });
    if (!customization.color && !customization.image) continue;
    entries[id] = {
      ...customization,
      path: typeof entry.path === 'string' ? entry.path : '',
    };
  }
  return entries;
}

export function readProjectIdentityStore(storage?: Storage | null): ProjectIdentityStore {
  const source = storageOrNull(storage);
  if (!source) return emptyStore();

  let parsed = parseJson(source.getItem(PROJECT_IDENTITY_STORAGE_KEY));
  if (!isRecord(parsed)) parsed = parseJson(source.getItem(LEGACY_PROJECT_IDENTITY_STORAGE_KEY));
  if (!isRecord(parsed)) return emptyStore();

  const rawProjects = isRecord(parsed.projects) ? parsed.projects : parsed;
  return { version: 1, projects: parseEntries(rawProjects) };
}

export function saveProjectIdentityStore(store: ProjectIdentityStore, storage?: Storage | null) {
  const source = storageOrNull(storage);
  if (!source) return false;
  try {
    source.setItem(PROJECT_IDENTITY_STORAGE_KEY, JSON.stringify(store));
    return true;
  } catch {
    return false;
  }
}

export function emptyProjectCustomization(): ProjectCustomization {
  return { color: null, image: null };
}

export function resolveProjectCustomization(
  store: ProjectIdentityStore,
  projectId: string,
  projectPath: string,
) {
  const exact = store.projects[projectId];
  if (exact) {
    return { customization: sanitizeCustomization(exact), store, migrated: false };
  }

  const migratedEntry = Object.entries(store.projects).find(([, entry]) => entry.path && entry.path === projectPath);
  if (!migratedEntry) {
    return { customization: emptyProjectCustomization(), store, migrated: false };
  }

  const [, entry] = migratedEntry;
  const projects = { ...store.projects };
  delete projects[migratedEntry[0]];
  projects[projectId] = { ...entry, path: projectPath };
  return {
    customization: sanitizeCustomization(entry),
    store: { version: 1 as const, projects },
    migrated: true,
  };
}

export function setProjectCustomization(
  store: ProjectIdentityStore,
  projectId: string,
  projectPath: string,
  customization: ProjectCustomization,
) {
  const color = normalizeHexColor(customization.color);
  const image = sanitizeImage(customization.image);
  const projects = { ...store.projects };
  for (const [id, entry] of Object.entries(projects)) {
    if (id !== projectId && entry.path && entry.path === projectPath) delete projects[id];
  }

  if (!color && !image) {
    delete projects[projectId];
  } else {
    projects[projectId] = { color, image, path: projectPath };
  }
  return { version: 1 as const, projects };
}

export function removeProjectCustomization(
  store: ProjectIdentityStore,
  projectId: string,
  projectPath?: string,
) {
  const projects = { ...store.projects };
  for (const [id, entry] of Object.entries(projects)) {
    if (id === projectId || (projectPath && entry.path === projectPath)) delete projects[id];
  }
  return { version: 1 as const, projects };
}
