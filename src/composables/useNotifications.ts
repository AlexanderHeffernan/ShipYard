import { computed, ref } from 'vue';
import type { Project } from '../types/projects';
import {
  DEFAULT_NOTIFICATION_SETTINGS,
  type NotificationEvent,
  type NotificationPermissionState,
  type NotificationProjectSnapshot,
  type NotificationSettings,
} from '../types/notifications';
import {
  getNotificationPermission,
  getNotificationSettings,
  isMacOSPlatform,
  observePullRequests,
  openNotificationSettings,
  requestNotificationPermission,
  saveNotificationSettings,
  sendSystemNotification,
} from '../services/notifications';

const settings = ref<NotificationSettings>({ ...DEFAULT_NOTIFICATION_SETTINGS });
const permission = ref<NotificationPermissionState>('unknown');
const loading = ref(false);
const permissionLoading = ref(false);
const error = ref<string | null>(null);
const testSent = ref(false);

let settingsPromise: Promise<NotificationSettings> | null = null;
let observationQueue = Promise.resolve();
let pollingTimer: number | undefined;
let pollingRefresh: (() => Promise<void> | void) | null = null;
let pollingInFlight = false;
let testFeedbackTimer: number | undefined;

function errorMessage(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}

function snapshotProject(project: Project): NotificationProjectSnapshot {
  const pullRequests = new Map<number, NotificationProjectSnapshot['pullRequests'][number]>();
  for (const item of project.workItems) {
    const pullRequest = item.pullRequest;
    if (!pullRequest || pullRequests.has(pullRequest.number)) continue;
    pullRequests.set(pullRequest.number, {
      number: pullRequest.number,
      headSha: pullRequest.headSha,
      draft: pullRequest.draft,
      mergeState: pullRequest.mergeState,
      attentionState: pullRequest.attentionState,
      baseBranch: pullRequest.baseBranch,
    });
  }
  return {
    id: project.id,
    name: project.name,
    available: project.githubRepository !== null && project.githubError === null,
    pullRequests: [...pullRequests.values()],
  };
}

function shouldNotify(event: NotificationEvent) {
  return event.kind === 'newPullRequest'
    ? settings.value.newPullRequests
    : settings.value.pullRequestUpdates;
}

async function deliver(events: NotificationEvent[]) {
  const eligible = events.filter(shouldNotify);
  if (eligible.length === 0) return;

  if (permission.value !== 'granted') {
    permission.value = await getNotificationPermission();
  }
  if (permission.value !== 'granted') return;

  for (const event of eligible) {
    try {
      sendSystemNotification(event);
    } catch (sendError) {
      error.value = errorMessage(sendError);
      break;
    }
  }
}

export function useNotifications() {
  const isMacOS = isMacOSPlatform();
  const enabledCount = computed(() => Number(settings.value.newPullRequests) + Number(settings.value.pullRequestUpdates));

  async function loadSettings(force = false) {
    if (settingsPromise && !force) return settingsPromise;
    settingsPromise = getNotificationSettings()
      .then((value) => {
        settings.value = { ...DEFAULT_NOTIFICATION_SETTINGS, ...value };
        return settings.value;
      })
      .finally(() => {
        settingsPromise = null;
      });
    return settingsPromise;
  }

  async function refreshPermission() {
    permissionLoading.value = true;
    try {
      permission.value = await getNotificationPermission();
    } finally {
      permissionLoading.value = false;
    }
    return permission.value;
  }

  async function enablePermission() {
    if (permission.value === 'granted') return permission.value;
    permissionLoading.value = true;
    try {
      permission.value = await requestNotificationPermission();
    } finally {
      permissionLoading.value = false;
    }
    return permission.value;
  }

  async function setRule(rule: keyof NotificationSettings, enabled: boolean) {
    error.value = null;
    loading.value = true;
    try {
      if (enabled) {
        const nextPermission = await enablePermission();
        if (nextPermission !== 'granted') return false;
      }
      settings.value = await saveNotificationSettings({
        ...settings.value,
        [rule]: enabled,
      });
      return true;
    } catch (saveError) {
      error.value = errorMessage(saveError);
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function sendTestNotification() {
    error.value = null;
    testSent.value = false;
    const nextPermission = permission.value === 'granted'
      ? permission.value
      : await enablePermission();
    if (nextPermission !== 'granted') return false;
    try {
      sendSystemNotification({
        title: 'Shipyard notifications',
        body: 'System notifications are ready.',
      });
      testSent.value = true;
      window.clearTimeout(testFeedbackTimer);
      testFeedbackTimer = window.setTimeout(() => (testSent.value = false), 1800);
      return true;
    } catch (sendError) {
      error.value = errorMessage(sendError);
      return false;
    }
  }

  function observeProjects(projects: Project[]) {
    const snapshots = projects.map(snapshotProject);
    observationQueue = observationQueue.then(async () => {
      try {
        const events = await observePullRequests(snapshots);
        await deliver(events);
      } catch (observeError) {
        // Browser-only previews do not expose Tauri commands. Keep polling
        // resilient and surface real native errors in the settings UI.
        if (isMacOS) error.value = errorMessage(observeError);
      }
    });
    return observationQueue;
  }

  async function poll() {
    if (!pollingRefresh || pollingInFlight) return;
    pollingInFlight = true;
    try {
      await pollingRefresh();
    } finally {
      pollingInFlight = false;
    }
  }

  function startPolling(refresh: () => Promise<void> | void) {
    pollingRefresh = refresh;
    if (pollingTimer !== undefined) return;
    pollingTimer = window.setInterval(() => void poll(), 5 * 60 * 1000);
  }

  function stopPolling() {
    if (pollingTimer !== undefined) {
      window.clearInterval(pollingTimer);
      pollingTimer = undefined;
    }
    pollingRefresh = null;
    pollingInFlight = false;
    window.clearTimeout(testFeedbackTimer);
  }

  return {
    enabledCount,
    error,
    isMacOS,
    loading,
    permission,
    permissionLoading,
    requestPermission: enablePermission,
    refreshPermission,
    refreshSettings: () => loadSettings(true),
    loadSettings,
    observeProjects,
    openNotificationSettings,
    sendTestNotification,
    setRule,
    settings,
    startPolling,
    stopPolling,
    testSent,
  };
}
