import { describe, expect, it } from 'vitest';
import type { ProjectImage } from '../types/projects';
import {
  LEGACY_PROJECT_IDENTITY_STORAGE_KEY,
  PROJECT_COLOR_PRESETS,
  PROJECT_IDENTITY_STORAGE_KEY,
  projectDefaultColor,
  readProjectIdentityStore,
  removeProjectCustomization,
  resolveProjectCustomization,
  saveProjectIdentityStore,
  setProjectCustomization,
} from './projectIdentity';

function storage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
    clear: () => values.clear(),
    key: (index: number) => [...values.keys()][index] ?? null,
    get length() {
      return values.size;
    },
  } as Storage;
}

const image: ProjectImage = {
  dataUrl: 'data:image/png;base64,AA==',
  name: 'mark.png',
  type: 'image/png',
  size: 1,
  width: 1,
  height: 1,
};

describe('project identity persistence', () => {
  it('generates vibrant defaults from the branded palette', () => {
    const colors = ['project-a', 'project-b', 'project-c', 'project-d'].map(projectDefaultColor);

    expect(colors.every((color) => PROJECT_COLOR_PRESETS.some((preset) => preset.value === color))).toBe(true);
    expect(colors).not.toContain('#9699a1');
  });

  it('round-trips a project color and image through storage', () => {
    const target = storage();
    const store = setProjectCustomization(
      readProjectIdentityStore(target),
      'project-1',
      '/Users/alex/Shipyard',
      { color: '#ABC', image },
    );

    expect(saveProjectIdentityStore(store, target)).toBe(true);
    const loaded = readProjectIdentityStore(target);
    expect(loaded.projects['project-1'].color).toBe('#aabbcc');
    expect(loaded.projects['project-1'].image).toEqual(image);
    expect(target.getItem(PROJECT_IDENTITY_STORAGE_KEY)).toContain('project-1');
  });

  it('migrates a legacy path entry when a project id changes', () => {
    const target = storage();
    target.setItem(
      LEGACY_PROJECT_IDENTITY_STORAGE_KEY,
      JSON.stringify({ 'old-project-id': { path: '/Users/alex/Shipyard', color: '#ff4d6d', image: null } }),
    );

    const resolved = resolveProjectCustomization(
      readProjectIdentityStore(target),
      'new-project-id',
      '/Users/alex/Shipyard',
    );

    expect(resolved.migrated).toBe(true);
    expect(resolved.customization.color).toBe('#ff4d6d');
    expect(resolved.store.projects['old-project-id']).toBeUndefined();
    expect(resolved.store.projects['new-project-id'].path).toBe('/Users/alex/Shipyard');
  });

  it('removes customization aliases when a project is deleted', () => {
    const store = setProjectCustomization(
      setProjectCustomization(readProjectIdentityStore(null), 'project-1', '/repo', { color: '#fb771f', image: null }),
      'project-2',
      '/other',
      { color: '#0ea5e9', image: null },
    );

    const removed = removeProjectCustomization(store, 'project-1', '/repo');
    expect(removed.projects['project-1']).toBeUndefined();
    expect(removed.projects['project-2'].color).toBe('#0ea5e9');
  });

  it('drops malformed or oversized images instead of persisting unusable data', () => {
    const store = setProjectCustomization(readProjectIdentityStore(null), 'project-1', '/repo', {
      color: null,
      image: { ...image, type: 'image/svg+xml', size: 3 * 1024 * 1024 },
    });

    expect(store.projects['project-1']).toBeUndefined();
  });
});
