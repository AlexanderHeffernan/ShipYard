import { describe, expect, it } from 'vitest';
import {
  readSunsetEffectEnabled,
  saveSunsetEffectEnabled,
} from './celebration';

class MemoryStorage {
  private readonly values = new Map<string, string>();

  getItem(key: string) {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string) {
    this.values.set(key, value);
  }
}

describe('sunset effect setting', () => {
  it('defaults off and persists the toggle', () => {
    const storage = new MemoryStorage();
    expect(readSunsetEffectEnabled(storage)).toBe(false);
    expect(saveSunsetEffectEnabled(true, storage)).toBe(true);
    expect(readSunsetEffectEnabled(storage)).toBe(true);
    expect(saveSunsetEffectEnabled(false, storage)).toBe(false);
    expect(readSunsetEffectEnabled(storage)).toBe(false);
  });

  it('migrates only the old sunset selection', () => {
    const sunsetStorage = new MemoryStorage();
    sunsetStorage.setItem('shipyard.completionAnimation', 'shipyard-sunset');
    expect(readSunsetEffectEnabled(sunsetStorage)).toBe(true);

    const otherStorage = new MemoryStorage();
    otherStorage.setItem('shipyard.completionAnimation', 'firework-sky');
    expect(readSunsetEffectEnabled(otherStorage)).toBe(false);
  });

  it('remains usable when storage is unavailable', () => {
    const storage = {
      getItem: () => { throw new Error('storage blocked'); },
      setItem: () => { throw new Error('storage blocked'); },
    };
    expect(readSunsetEffectEnabled(storage)).toBe(false);
    expect(saveSunsetEffectEnabled(true, storage)).toBe(true);
  });
});
