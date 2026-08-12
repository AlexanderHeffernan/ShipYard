export const WINDOWED_TITLEBAR_CONTROLS_INSET = 82;
export const FULLSCREEN_TITLEBAR_CONTROLS_INSET = 12;
export const SIDEBAR_TOGGLE_SIZE = 24;
export const TITLEBAR_CONTENT_GAP = 10;

export function titlebarControlsInset(isFullscreen: boolean) {
  return isFullscreen
    ? FULLSCREEN_TITLEBAR_CONTROLS_INSET
    : WINDOWED_TITLEBAR_CONTROLS_INSET;
}

export function workHeaderLeadingInset(isFullscreen: boolean, sidebarOpen: boolean) {
  if (sidebarOpen) return 16;
  return titlebarControlsInset(isFullscreen) + SIDEBAR_TOGGLE_SIZE + TITLEBAR_CONTENT_GAP;
}
