import { invoke } from '@tauri-apps/api/core';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import type {
  NotificationEvent,
  NotificationPermissionState,
  NotificationProjectSnapshot,
  NotificationSettings,
} from '../types/notifications';
import { normalizeNotificationPermission } from '../utils/notifications';

export function isMacOSPlatform() {
  return navigator.platform.startsWith('Mac') || navigator.userAgent.includes('Mac OS X');
}

export function getNotificationSettings() {
  return invoke<NotificationSettings>('get_notification_settings');
}

export function saveNotificationSettings(settings: NotificationSettings) {
  return invoke<NotificationSettings>('save_notification_settings', { settings });
}

export function observePullRequests(projects: NotificationProjectSnapshot[]) {
  return invoke<NotificationEvent[]>('observe_pull_requests', { projects });
}

export function openNotificationSettings() {
  return invoke<void>('open_notification_settings');
}

export async function getNotificationPermission(): Promise<NotificationPermissionState> {
  if (!isMacOSPlatform()) return 'unsupported';

  const browserPermission = typeof Notification === 'undefined'
    ? undefined
    : Notification.permission;
  if (browserPermission === 'granted' || browserPermission === 'denied') {
    return normalizeNotificationPermission(browserPermission);
  }

  try {
    return normalizeNotificationPermission(browserPermission, await isPermissionGranted(), true);
  } catch {
    return normalizeNotificationPermission(browserPermission);
  }
}

export async function requestNotificationPermission(): Promise<NotificationPermissionState> {
  if (!isMacOSPlatform()) return 'unsupported';
  try {
    return normalizeNotificationPermission(await requestPermission());
  } catch {
    return 'unknown';
  }
}

export function sendSystemNotification(event: Pick<NotificationEvent, 'title' | 'body'>) {
  sendNotification({
    title: event.title,
    body: event.body,
    group: 'shipyard.pull-requests',
    sound: 'Ping',
  });
}
