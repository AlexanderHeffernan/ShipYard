import { describe, expect, it } from 'vitest';
import {
  FULLSCREEN_TITLEBAR_CONTROLS_INSET,
  titlebarControlsInset,
  WINDOWED_TITLEBAR_CONTROLS_INSET,
} from './titlebar';

describe('titlebarControlsInset', () => {
  it('keeps the traffic-light-safe inset in windowed mode', () => {
    expect(titlebarControlsInset(false)).toBe(WINDOWED_TITLEBAR_CONTROLS_INSET);
  });

  it('uses a safe edge inset when the native window is fullscreen', () => {
    expect(titlebarControlsInset(true)).toBe(FULLSCREEN_TITLEBAR_CONTROLS_INSET);
  });
});
