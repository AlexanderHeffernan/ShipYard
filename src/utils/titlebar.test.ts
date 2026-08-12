import { describe, expect, it } from 'vitest';
import {
  FULLSCREEN_TITLEBAR_CONTROLS_INSET,
  titlebarControlsInset,
  WINDOWED_TITLEBAR_CONTROLS_INSET,
  workHeaderLeadingInset,
} from './titlebar';

describe('titlebarControlsInset', () => {
  it('keeps the traffic-light-safe inset in windowed mode', () => {
    expect(titlebarControlsInset(false)).toBe(WINDOWED_TITLEBAR_CONTROLS_INSET);
  });

  it('uses a safe edge inset when the native window is fullscreen', () => {
    expect(titlebarControlsInset(true)).toBe(FULLSCREEN_TITLEBAR_CONTROLS_INSET);
  });

  it('keeps the work header close to the sidebar toggle in fullscreen', () => {
    expect(workHeaderLeadingInset(true, false)).toBe(46);
  });

  it('preserves the traffic-light-safe work header inset when windowed', () => {
    expect(workHeaderLeadingInset(false, false)).toBe(116);
  });

  it('keeps the regular work header inset while the sidebar is open', () => {
    expect(workHeaderLeadingInset(true, true)).toBe(16);
    expect(workHeaderLeadingInset(false, true)).toBe(16);
  });
});
