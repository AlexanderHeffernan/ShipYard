<script setup lang="ts">
import { Bell, Check, ExternalLink, RefreshCw, Send, ShieldCheck } from '@lucide/vue';
import { computed, onMounted, ref } from 'vue';
import AppButton from '../ui/AppButton.vue';
import { useNotifications } from '../../composables/useNotifications';
import type { NotificationPermissionState, NotificationSettings } from '../../types/notifications';

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

const permissionTitle = computed(() => {
  switch (permission.value) {
    case 'granted': return 'Notifications are allowed';
    case 'denied': return 'Notifications are turned off';
    case 'prompt': return 'Permission needed';
    case 'unsupported': return 'macOS notifications unavailable';
    default: return 'Checking notification permission';
  }
});

const permissionDescription = computed(() => {
  switch (permission.value) {
    case 'granted': return 'Shipyard can alert you about the pull requests you choose below.';
    case 'denied': return 'Allow Shipyard in System Settings → Notifications to receive alerts.';
    case 'prompt': return 'Allow Shipyard to send system notifications before enabling a rule.';
    case 'unsupported': return 'These controls apply to the macOS build. No system alerts are sent from this environment.';
    default: return 'Shipyard could not confirm the current permission. Try allowing notifications, or use Refresh.';
  }
});

const permissionTone = computed(() => {
  if (permission.value === 'granted') return 'success';
  if (permission.value === 'denied') return 'danger';
  if (permission.value === 'unsupported') return 'muted';
  return 'warning';
});

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

function stateLabel(state: NotificationPermissionState) {
  switch (state) {
    case 'granted': return 'Allowed';
    case 'denied': return 'Blocked';
    case 'prompt': return 'Not yet allowed';
    case 'unsupported': return 'macOS only';
    default: return 'Unknown';
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

    <div class="permission-card" :class="`permission-card--${permissionTone}`" aria-live="polite">
      <span class="permission-card__icon">
        <ShieldCheck aria-hidden="true" />
      </span>
      <div class="permission-card__copy">
        <div class="permission-card__title-row">
          <strong>{{ permissionTitle }}</strong>
          <span class="permission-card__state">{{ stateLabel(permission) }}</span>
        </div>
        <p>{{ permissionDescription }}</p>
        <p v-if="permission === 'denied'" class="permission-card__guidance">
          System Settings → Notifications → Shipyard → Allow Notifications
        </p>
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
        <ExternalLink aria-hidden="true" /> Open settings
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

    <details class="behavior-details">
      <summary>How Shipyard avoids notification spam</summary>
      <p>
        Shipyard stores each project + pull request number, its material revision, presence, and last event state on disk.
        A material revision changes when commits, draft state, normalized review/check attention, merge state, or the target
        branch changes. Repeated polls of the same revision are ignored. The first scan establishes a quiet baseline, so
        opening Shipyard does not notify for every existing pull request. A failed project/GitHub scan is ignored rather
        than treated as a disappearance.
      </p>
    </details>

    <div class="privacy-note">
      <Check aria-hidden="true" />
      <p>Alerts include only the project name and pull-request number. Pull-request titles, branches, paths, commit messages, and review text stay private.</p>
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
.permission-card { display: flex; align-items: center; gap: 12px; min-height: 77px; padding: 12px 13px; border: 1px solid var(--border-subtle); border-radius: 9px; }
.permission-card--success { border-color: var(--success-border); background: var(--success-subtle); }
.permission-card--warning { border-color: var(--warning-border); background: var(--warning-subtle); }
.permission-card--danger { border-color: var(--danger-border); background: var(--danger-subtle); }
.permission-card--muted { opacity: .68; }
.permission-card__icon { display: grid; width: 37px; height: 37px; flex: 0 0 auto; place-items: center; color: var(--warning); background: rgba(255,255,255,.05); border-radius: 9px; }
.permission-card--success .permission-card__icon { color: var(--success); }
.permission-card--danger .permission-card__icon { color: var(--danger); }
.permission-card__icon svg { width: 19px; }
.permission-card__copy { min-width: 0; flex: 1; }
.permission-card__title-row { display: flex; align-items: center; gap: 8px; }
.permission-card strong, .test-row strong { font-size: 12px; font-weight: 550; }
.permission-card__state { padding: 2px 5px; font-size: 9px; color: var(--text-secondary); border: 1px solid var(--border-subtle); border-radius: 4px; }
.permission-card__guidance { margin-top: 3px; color: var(--warning); }
.rule-list { display: flex; flex-direction: column; gap: 8px; margin-top: 12px; }
.notification-rule { display: flex; align-items: center; gap: 11px; min-height: 67px; padding: 11px 12px; border: 1px solid var(--border-subtle); border-radius: 9px; cursor: pointer; }
.notification-rule:hover:not(.disabled) { background: var(--surface-hover); }
.notification-rule.enabled { border-color: var(--primary-border); background: var(--primary-subtle); }
.notification-rule.disabled { cursor: not-allowed; opacity: .48; }
.notification-rule__icon { display: grid; width: 32px; height: 32px; flex: 0 0 auto; place-items: center; color: var(--text-secondary); background: var(--surface-subtle); border-radius: 8px; }
.notification-rule.enabled .notification-rule__icon { color: var(--primary-hover); }
.notification-rule__icon svg { width: 16px; height: 16px; }
.notification-rule__copy { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 4px; }
.notification-rule__copy strong { font-size: 12px; font-weight: 550; }
.notification-rule__copy small { font-size: 10px; line-height: 1.35; color: var(--text-secondary); }
.notification-rule input { width: 31px; height: 18px; flex: 0 0 auto; margin: 0; accent-color: var(--primary); }
.behavior-details { margin-top: 12px; padding: 10px 12px; background: var(--surface-subtle); border: 1px solid var(--border-subtle); border-radius: 8px; }
.behavior-details summary { font-size: 10px; font-weight: 550; color: var(--text-secondary); cursor: pointer; }
.behavior-details p { max-width: 520px; margin-top: 8px; font-size: 10px; }
.privacy-note { display: flex; align-items: start; gap: 8px; margin-top: 11px; padding: 9px 10px; background: var(--surface-subtle); border-radius: 7px; }
.privacy-note svg { width: 13px; height: 13px; flex: 0 0 auto; margin-top: 2px; color: var(--success); }
.privacy-note p { margin: 0; font-size: 10px; }
.test-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-top: 13px; padding-top: 13px; border-top: 1px solid var(--border-subtle); }
.test-row p { margin-top: 3px; font-size: 10px; }
.error { margin-top: 12px; color: var(--danger); }
</style>
