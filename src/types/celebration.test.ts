import { describe, expect, it } from 'vitest';
import {
  completionAnimationOptions,
  completionAnimationSpeedMultiplier,
  DEFAULT_COMPLETION_ANIMATION,
  DEFAULT_COMPLETION_ANIMATION_SPEED,
  isFullScreenCompletionAnimation,
  readCompletionAnimation,
  readCompletionAnimationSpeed,
  saveCompletionAnimation,
  saveCompletionAnimationSpeed,
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
  it('ships with one polished default and eleven full-screen variants', () => {
    expect(completionAnimationOptions).toHaveLength(12);
    expect(completionAnimationOptions.filter((option) => option.default)).toHaveLength(1);
    expect(completionAnimationOptions.filter((option) => option.fullScreen)).toHaveLength(11);
    expect(DEFAULT_COMPLETION_ANIMATION).toBe('quiet-handoff');
    expect(isFullScreenCompletionAnimation(DEFAULT_COMPLETION_ANIMATION)).toBe(false);
    expect(isFullScreenCompletionAnimation('sail-away')).toBe(true);
    expect(isFullScreenCompletionAnimation('shipyard-sunset')).toBe(true);
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
    storage.setItem('shipyard.completionAnimation', 'harbor-glow');
    expect(readCompletionAnimation(storage)).toBe(DEFAULT_COMPLETION_ANIMATION);
  });

  it('persists full-screen speed and falls back safely from stale storage', () => {
    const storage = new MemoryStorage();

    expect(readCompletionAnimationSpeed(storage)).toBe(DEFAULT_COMPLETION_ANIMATION_SPEED);
    expect(saveCompletionAnimationSpeed('slow', storage)).toBe('slow');
    expect(readCompletionAnimationSpeed(storage)).toBe('slow');
    expect(saveCompletionAnimationSpeed('too-slow', storage)).toBe(DEFAULT_COMPLETION_ANIMATION_SPEED);

    storage.setItem('shipyard.completionAnimationSpeed', 'too-fast');
    expect(readCompletionAnimationSpeed(storage)).toBe(DEFAULT_COMPLETION_ANIMATION_SPEED);
  });

  it('keeps the speed ordering intuitive for full-screen timing', () => {
    expect(completionAnimationSpeedMultiplier('fast')).toBeLessThan(1);
    expect(completionAnimationSpeedMultiplier('normal')).toBe(1);
    expect(completionAnimationSpeedMultiplier('slow')).toBeGreaterThan(1);
  });

  it('keeps the feature usable when storage is unavailable', () => {
    const storage = {
      getItem: () => { throw new Error('storage blocked'); },
      setItem: () => { throw new Error('storage blocked'); },
    };

    expect(readCompletionAnimation(storage)).toBe(DEFAULT_COMPLETION_ANIMATION);
    expect(saveCompletionAnimation('sail-away', storage)).toBe('sail-away');
    expect(readCompletionAnimationSpeed(storage)).toBe(DEFAULT_COMPLETION_ANIMATION_SPEED);
    expect(saveCompletionAnimationSpeed('fast', storage)).toBe('fast');
  });
});
