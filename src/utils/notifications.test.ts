import { describe, expect, it } from 'vitest';
import { normalizeNotificationPermission } from './notifications';

describe('notification permission states', () => {
  it('distinguishes granted, denied, and first-run prompt states', () => {
    expect(normalizeNotificationPermission('granted')).toBe('granted');
    expect(normalizeNotificationPermission('denied')).toBe('denied');
    expect(normalizeNotificationPermission('default')).toBe('prompt');
  });

  it('accepts a native plugin grant when the browser permission is still default', () => {
    expect(normalizeNotificationPermission('default', true)).toBe('granted');
  });

  it('treats a responsive native plugin with no grant as a first-run prompt', () => {
    expect(normalizeNotificationPermission(undefined, false, true)).toBe('prompt');
  });

  it('keeps an unavailable permission state explicit', () => {
    expect(normalizeNotificationPermission(undefined)).toBe('unknown');
  });
});
