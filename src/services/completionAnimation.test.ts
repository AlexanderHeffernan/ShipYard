import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  preloadSunsetEffect,
  sunsetEffectAssets,
} from './completionAnimation';

class TestImage {
  static instances: TestImage[] = [];

  complete = false;
  decoding = '';
  loading = '';
  decodeCalls = 0;
  private imageSource = '';
  private readonly listeners = new Map<string, () => void>();

  constructor() {
    TestImage.instances.push(this);
  }

  get src() {
    return this.imageSource;
  }

  set src(value: string) {
    this.imageSource = value;
    this.complete = true;
    this.listeners.get('load')?.();
  }

  addEventListener(type: string, listener: () => void) {
    this.listeners.set(type, listener);
  }

  decode() {
    this.decodeCalls += 1;
    return Promise.resolve();
  }
}

afterEach(() => {
  vi.unstubAllGlobals();
  TestImage.instances = [];
});

describe('sunset effect preloading', () => {
  it('loads and decodes each asset once while sharing the preload promise', async () => {
    vi.stubGlobal('Image', TestImage);

    const firstPreload = preloadSunsetEffect();
    expect(preloadSunsetEffect()).toBe(firstPreload);
    await firstPreload;

    expect(TestImage.instances).toHaveLength(Object.keys(sunsetEffectAssets).length);
    expect(TestImage.instances.map((image) => image.src)).toEqual(Object.values(sunsetEffectAssets));
    expect(TestImage.instances.every((image) => image.decodeCalls === 1)).toBe(true);
  });
});
