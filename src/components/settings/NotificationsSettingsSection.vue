<script setup lang="ts">
import { Bell, ExternalLink, RefreshCw, Send } from '@lucide/vue';
import { onMounted, ref } from 'vue';
import AppButton from '../ui/AppButton.vue';
import { useNotifications } from '../../composables/useNotifications';
import type { NotificationSettings } from '../../types/notifications';

const {
  error,
  isMacOS,
  loading,
  permission,
  permissionLoading,
  requestPermission,
  refreshPermission,
  refreshSettings,
  openNotificationSettings,
  sendTestNotification,
  setRule,
  settings,
  testSent,
} = useNotifications();

const busyRule = ref<keyof NotificationSettings | null>(null);
const openingSystemSettings = ref(false);
const localError = ref<string | null>(null);

function ruleDisabled(rule: keyof NotificationSettings) {
  return !isMacOS || permission.value !== 'granted' || loading.value || busyRule.value === rule;
}

async function onRuleChange(rule: keyof NotificationSettings, event: Event) {
  const enabled = (event.target as HTMLInputElement).checked;
  busyRule.value = rule;
  localError.value = null;
  try {
    await setRule(rule, enabled);
  } finally {
    busyRule.value = null;
  }
}

async function allowNotifications() {
  busyRule.value = null;
  localError.value = null;
  try {
    await requestPermission();
  } catch (permissionError) {
    localError.value = String(permissionError);
  }
}

async function openSystemSettings() {
  openingSystemSettings.value = true;
  localError.value = null;
  try {
    await openNotificationSettings();
    await refreshPermission();
  } catch (settingsError) {
    localError.value = String(settingsError);
  } finally {
    openingSystemSettings.value = false;
  }
}

async function test() {
  localError.value = null;
  const sent = await sendTestNotification();
  if (!sent && permission.value === 'denied') {
    localError.value = 'macOS is blocking notifications. Open System Settings, allow Shipyard, then try again.';
  }
}

onMounted(() => {
  void Promise.all([refreshSettings(), refreshPermission()]);
});
</script>

<template>
  <main class="notifications-section">
    <div class="section-heading">
      <div>
        <h3>Notifications</h3>
        <p>Quiet, opt-in macOS alerts for pull requests across every project.</p>
      </div>
      <AppButton
        v-if="permission === 'prompt' || permission === 'unknown'"
        variant="primary"
        size="small"
        type="button"
        :loading="permissionLoading"
        loading-label="Allowing"
        @click="allowNotifications"
      >
        Allow notifications
      </AppButton>
      <AppButton
        v-else-if="permission === 'denied'"
        variant="ghost"
        size="small"
        type="button"
        :loading="openingSystemSettings"
        loading-label="Opening"
        @click="openSystemSettings"
      >
        <ExternalLink aria-hidden="true" /> Blocked · Open settings
      </AppButton>
      <span v-else-if="permission === 'unsupported'" class="permission-status permission-status--muted">
        macOS only
      </span>
      <AppButton
        v-else
        variant="ghost"
        size="small"
        type="button"
        :loading="permissionLoading"
        loading-label="Checking"
        :success="permission === 'granted'"
        success-label="Allowed"
        @click="refreshPermission"
      >
        <RefreshCw aria-hidden="true" /> Refresh
      </AppButton>
    </div>

    <div class="rule-list" :aria-disabled="!isMacOS || permission !== 'granted'">
      <label class="notification-rule" :class="{ enabled: settings.newPullRequests, disabled: ruleDisabled('newPullRequests') }">
        <span class="notification-rule__icon"><Bell aria-hidden="true" /></span>
        <span class="notification-rule__copy">
          <strong>New pull requests</strong>
          <small>Alert once when an open pull request first appears in a project.</small>
        </span>
        <input
          type="checkbox"
          :checked="settings.newPullRequests"
          :disabled="ruleDisabled('newPullRequests')"
          aria-label="Notify when a new pull request appears"
          @change="onRuleChange('newPullRequests', $event)"
        />
      </label>

      <label class="notification-rule" :class="{ enabled: settings.pullRequestUpdates, disabled: ruleDisabled('pullRequestUpdates') }">
        <span class="notification-rule__icon"><RefreshCw aria-hidden="true" /></span>
        <span class="notification-rule__copy">
          <strong>Pull request updates</strong>
          <small>Alert when new commits, review/check attention, draft state, merge state, or the target branch changes.</small>
        </span>
        <input
          type="checkbox"
          :checked="settings.pullRequestUpdates"
          :disabled="ruleDisabled('pullRequestUpdates')"
          aria-label="Notify when a pull request is materially updated"
          @change="onRuleChange('pullRequestUpdates', $event)"
        />
      </label>
    </div>

    <div v-if="permission === 'granted'" class="test-row">
      <div>
        <strong>Test notifications</strong>
        <p>Send a safe sample alert without changing your rules.</p>
      </div>
      <AppButton
        variant="ghost"
        size="small"
        type="button"
        :success="testSent"
        success-label="Sent"
        @click="test"
      >
        <Send aria-hidden="true" /> Send test
      </AppButton>
    </div>

    <p v-if="localError || error" class="error" role="alert">{{ localError || error }}</p>
  </main>
</template>

<style scoped>
h3 { margin: 0; font-size: 14px; font-weight: 550; }
p { display: block; margin: 5px 0 0; font-size: 11px; line-height: 1.45; color: var(--text-secondary); }
.notifications-section { min-width: 0; flex: 1; padding: 24px; overflow-y: auto; }
.section-heading { display: flex; align-items: start; justify-content: space-between; gap: 12px; margin-bottom: 18px; }
.section-heading :deep(.app-button svg) { width: 13px; height: 13px; }
.permission-status { font-size: 10px; color: var(--text-secondary); }
.permission-status--muted { opacity: .68; }
.rule-list { display: flex; flex-direction: column; gap: 8px; margin-top: 12px; }
.notification-rule { display: flex; align-items: center; gap: 11px; min-height: 67px; padding: 11px 12px; border: 1px solid var(--border-subtle); border-radius: 9px; cursor: pointer; }
.notification-rule:hover:not(.disabled) { background: var(--surface-hover); }
.notification-rule.enabled { background: var(--surface-subtle); }
.notification-rule.disabled { cursor: not-allowed; opacity: .48; }
.notification-rule__icon { display: grid; width: 32px; height: 32px; flex: 0 0 auto; place-items: center; color: var(--text-secondary); background: var(--surface-subtle); border-radius: 8px; }
.notification-rule__icon svg { width: 16px; height: 16px; }
.notification-rule__copy { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 4px; }
.notification-rule__copy strong { font-size: 12px; font-weight: 550; }
.notification-rule__copy small { font-size: 10px; line-height: 1.35; color: var(--text-secondary); }
.notification-rule input { width: 31px; height: 18px; flex: 0 0 auto; margin: 0; accent-color: var(--primary); }
.test-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-top: 13px; padding-top: 13px; border-top: 1px solid var(--border-subtle); }
.test-row strong { font-size: 12px; font-weight: 550; }
.test-row p { margin-top: 3px; font-size: 10px; }
.error { margin-top: 12px; color: var(--danger); }
</style>
