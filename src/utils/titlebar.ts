export const WINDOWED_TITLEBAR_CONTROLS_INSET = 82;
export const FULLSCREEN_TITLEBAR_CONTROLS_INSET = 12;

export function titlebarControlsInset(isFullscreen: boolean) {
  return isFullscreen
    ? FULLSCREEN_TITLEBAR_CONTROLS_INSET
    : WINDOWED_TITLEBAR_CONTROLS_INSET;
}
