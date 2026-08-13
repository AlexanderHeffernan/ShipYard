import type { NotificationPermissionState } from '../types/notifications';

export function normalizeNotificationPermission(
  permission: NotificationPermission | undefined,
  grantedByPlugin = false,
  nativePluginAvailable = false,
): NotificationPermissionState {
  if (permission === 'granted' || grantedByPlugin) return 'granted';
  if (permission === 'denied') return 'denied';
  if (permission === 'default') return 'prompt';
  if (nativePluginAvailable) return 'prompt';
  return 'unknown';
}
