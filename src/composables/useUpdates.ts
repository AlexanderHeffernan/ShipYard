import { getVersion } from '@tauri-apps/api/app';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater';
import { ref, shallowRef } from 'vue';

const AUTOMATIC_CHECKS_KEY = 'shipyard.updates.automaticChecks';
const CHECK_INTERVAL = 6 * 60 * 60 * 1000;

export type UpdateStatus = 'idle' | 'checking' | 'up-to-date' | 'available' | 'installing' | 'error';

const currentVersion = ref('');
const availableUpdate = shallowRef<Update | null>(null);
const status = ref<UpdateStatus>('idle');
const error = ref<string | null>(null);
const lastCheckedAt = ref<number | null>(null);
const downloadProgress = ref<number | null>(null);
const automaticChecksEnabled = ref(readAutomaticChecksPreference());

let automaticCheckTimer: number | undefined;
let versionPromise: Promise<void> | null = null;
let checkPromise: Promise<void> | null = null;

function isMacOSPlatform() {
  return navigator.platform.startsWith('Mac') || navigator.userAgent.includes('Mac OS X');
}

function readAutomaticChecksPreference() {
  try {
    return localStorage.getItem(AUTOMATIC_CHECKS_KEY) !== 'false';
  } catch {
    return true;
  }
}

function errorMessage(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}

export function useUpdates() {
  async function loadVersion() {
    if (currentVersion.value) return;
    versionPromise ??= getVersion()
      .then((version) => {
        currentVersion.value = version;
      })
      .catch(() => {
        currentVersion.value = 'Unknown';
      })
      .finally(() => {
        versionPromise = null;
      });
    await versionPromise;
  }

  async function checkForUpdates(silent = false) {
    if (!isMacOSPlatform() || checkPromise || status.value === 'installing') return;

    status.value = 'checking';
    error.value = null;
    checkPromise = check({ timeout: 15_000 })
      .then((update) => {
        availableUpdate.value = update;
        status.value = update ? 'available' : 'up-to-date';
      })
      .catch((checkError) => {
        if (silent) {
          status.value = availableUpdate.value ? 'available' : 'idle';
          return;
        }
        availableUpdate.value = null;
        status.value = 'error';
        error.value = errorMessage(checkError);
      })
      .finally(() => {
        lastCheckedAt.value = Date.now();
        checkPromise = null;
      });
    await checkPromise;
  }

  async function installUpdate() {
    const update = availableUpdate.value;
    if (!update || status.value === 'installing') return;

    status.value = 'installing';
    error.value = null;
    downloadProgress.value = 0;
    let contentLength: number | undefined;
    let downloaded = 0;

    try {
      await update.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === 'Started') {
          contentLength = event.data.contentLength;
          downloaded = 0;
          downloadProgress.value = contentLength ? 0 : null;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          downloadProgress.value = contentLength
            ? Math.min(100, Math.round((downloaded / contentLength) * 100))
            : null;
        } else if (event.event === 'Finished') {
          downloadProgress.value = 100;
        }
      });
      await relaunch();
      availableUpdate.value = null;
      status.value = 'up-to-date';
      downloadProgress.value = null;
    } catch (installError) {
      status.value = 'error';
      error.value = errorMessage(installError);
      downloadProgress.value = null;
    }
  }

  function startAutomaticChecks() {
    if (automaticCheckTimer !== undefined || !automaticChecksEnabled.value || !isMacOSPlatform()) return;

    void loadVersion();
    void checkForUpdates(true);
    automaticCheckTimer = window.setInterval(() => void checkForUpdates(true), CHECK_INTERVAL);
  }

  function stopAutomaticChecks() {
    if (automaticCheckTimer === undefined) return;
    window.clearInterval(automaticCheckTimer);
    automaticCheckTimer = undefined;
  }

  function setAutomaticChecksEnabled(enabled: boolean) {
    automaticChecksEnabled.value = enabled;
    try {
      localStorage.setItem(AUTOMATIC_CHECKS_KEY, String(enabled));
    } catch {
      // A missing local storage implementation should not block update checks.
    }

    if (enabled) startAutomaticChecks();
    else stopAutomaticChecks();
  }

  return {
    automaticChecksEnabled,
    availableUpdate,
    currentVersion,
    downloadProgress,
    error,
    isMacOS: isMacOSPlatform(),
    lastCheckedAt,
    status,
    checkForUpdates,
    installUpdate,
    loadVersion,
    setAutomaticChecksEnabled,
    startAutomaticChecks,
    stopAutomaticChecks,
  };
}
