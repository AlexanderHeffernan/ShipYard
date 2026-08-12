import { describe, expect, it } from 'vitest';
import {
  completionAnimationOptions,
  DEFAULT_COMPLETION_ANIMATION,
  readCompletionAnimation,
  saveCompletionAnimation,
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

describe('completion animation selection', () => {
  it('ships with one polished default and ten additional variants', () => {
    expect(completionAnimationOptions).toHaveLength(11);
    expect(completionAnimationOptions.filter((option) => option.default)).toHaveLength(1);
    expect(DEFAULT_COMPLETION_ANIMATION).toBe('harbor-glow');
  });

  it('persists a valid selection and safely falls back from stale storage', () => {
    const storage = new MemoryStorage();

    expect(readCompletionAnimation(storage)).toBe(DEFAULT_COMPLETION_ANIMATION);
    expect(saveCompletionAnimation('firework-sky', storage)).toBe('firework-sky');
    expect(readCompletionAnimation(storage)).toBe('firework-sky');
    expect(saveCompletionAnimation('removed-variant', storage)).toBe(DEFAULT_COMPLETION_ANIMATION);
    expect(readCompletionAnimation(storage)).toBe(DEFAULT_COMPLETION_ANIMATION);

    storage.setItem('shipyard.completionAnimation', 'removed-variant');
    expect(readCompletionAnimation(storage)).toBe(DEFAULT_COMPLETION_ANIMATION);
  });

  it('keeps the feature usable when storage is unavailable', () => {
    const storage = {
      getItem: () => { throw new Error('storage blocked'); },
      setItem: () => { throw new Error('storage blocked'); },
    };

    expect(readCompletionAnimation(storage)).toBe(DEFAULT_COMPLETION_ANIMATION);
    expect(saveCompletionAnimation('sail-away', storage)).toBe('sail-away');
  });
});
