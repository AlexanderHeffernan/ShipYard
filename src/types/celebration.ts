export const SUNSET_EFFECT_STORAGE_KEY = 'shipyard.sunsetEffect';
const LEGACY_COMPLETION_ANIMATION_STORAGE_KEY = 'shipyard.completionAnimation';

export type CompletionEffectStorage = Pick<Storage, 'getItem' | 'setItem'>;

function browserStorage(): CompletionEffectStorage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

export function readSunsetEffectEnabled(storage?: CompletionEffectStorage): boolean {
  const source = storage ?? browserStorage();
  if (!source) return false;
  try {
    const saved = source.getItem(SUNSET_EFFECT_STORAGE_KEY);
    if (saved !== null) return saved === 'true';
    return source.getItem(LEGACY_COMPLETION_ANIMATION_STORAGE_KEY) === 'shipyard-sunset';
  } catch {
    return false;
  }
}

export function saveSunsetEffectEnabled(
  enabled: boolean,
  storage?: CompletionEffectStorage,
): boolean {
  const target = storage ?? browserStorage();
  try {
    target?.setItem(SUNSET_EFFECT_STORAGE_KEY, String(enabled));
  } catch {
    // The in-memory selection still works when storage is unavailable.
  }
  return enabled;
}
