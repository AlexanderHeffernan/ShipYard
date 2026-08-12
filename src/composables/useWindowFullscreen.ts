import { onBeforeUnmount, onMounted, ref } from 'vue';
import { getCurrentWindow, type Window as TauriWindow } from '@tauri-apps/api/window';
import type { UnlistenFn } from '@tauri-apps/api/event';

/**
 * Tracks the native window's fullscreen state for the current webview.
 *
 * Tauri does not expose a separate fullscreen-changed event, so resize events
 * are used only as a signal to ask the window for its authoritative state.
 * This keeps the layout independent from monitor dimensions and works per
 * window when an app has more than one window.
 */
export function useWindowFullscreen() {
  const isFullscreen = ref(false);
  const fullscreenStateReady = ref(false);
  let appWindow: TauriWindow | null = null;
  let unlistenResize: UnlistenFn | null = null;
  let syncFrame: number | null = null;
  let syncGeneration = 0;
  let disposed = false;

  function cancelScheduledSync() {
    if (syncFrame === null) return;
    window.cancelAnimationFrame(syncFrame);
    syncFrame = null;
  }

  async function syncFullscreen() {
    if (!appWindow || disposed) return;
    const generation = ++syncGeneration;
    try {
      const fullscreen = await appWindow.isFullscreen();
      if (!disposed && generation === syncGeneration) {
        isFullscreen.value = fullscreen;
        fullscreenStateReady.value = true;
      }
    } catch {
      // The Vite browser preview does not have a native Tauri window.
      if (!disposed && generation === syncGeneration) {
        isFullscreen.value = false;
        fullscreenStateReady.value = true;
      }
    }
  }

  function scheduleSync() {
    if (disposed || syncFrame !== null) return;
    syncFrame = window.requestAnimationFrame(() => {
      syncFrame = null;
      void syncFullscreen();
    });
  }

  onMounted(() => {
    window.addEventListener('resize', scheduleSync);
    document.addEventListener('fullscreenchange', scheduleSync);

    try {
      appWindow = getCurrentWindow();
    } catch {
      fullscreenStateReady.value = true;
      return;
    }

    void (async () => {
      await syncFullscreen();
      if (disposed || !appWindow) return;

      try {
        const unlisten = await appWindow.onResized(scheduleSync);
        if (disposed) unlisten();
        else unlistenResize = unlisten;
      } catch {
        // Keep the browser preview usable when the Tauri event bridge is absent.
      }
    })();
  });

  onBeforeUnmount(() => {
    disposed = true;
    syncGeneration += 1;
    cancelScheduledSync();
    unlistenResize?.();
    window.removeEventListener('resize', scheduleSync);
    document.removeEventListener('fullscreenchange', scheduleSync);
  });

  return { isFullscreen, fullscreenStateReady };
}
