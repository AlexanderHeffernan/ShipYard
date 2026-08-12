<script setup lang="ts">
import { Check, Download, RefreshCw, ShieldCheck } from '@lucide/vue';
import { computed, onMounted } from 'vue';
import AppButton from '../ui/AppButton.vue';
import { useUpdates } from '../../composables/useUpdates';

const {
  automaticChecksEnabled,
  availableUpdate,
  currentVersion,
  downloadProgress,
  error,
  isMacOS,
  lastCheckedAt,
  status,
  checkForUpdates,
  installUpdate,
  loadVersion,
  setAutomaticChecksEnabled,
} = useUpdates();

const lastCheckedLabel = computed(() => {
  if (!lastCheckedAt.value) return 'Not checked yet';
  return `Last checked ${new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(lastCheckedAt.value)}`;
});

const statusLabel = computed(() => {
  switch (status.value) {
    case 'checking': return 'Checking for updates…';
    case 'up-to-date': return 'Shipyard is up to date';
    case 'available': return `Version ${availableUpdate.value?.version ?? ''} is available`;
    case 'installing': return 'Installing update…';
    case 'error': return 'Could not check for updates';
    default: return 'Updates are ready when you are';
  }
});

const statusDescription = computed(() => {
  switch (status.value) {
    case 'checking': return 'Shipyard is checking the latest signed release.';
    case 'up-to-date': return 'You are running the newest signed macOS release.';
    case 'available': return 'Download and restart to move to the latest version.';
    case 'installing': return 'The update is being verified and installed. Shipyard will restart when it is ready.';
    case 'error': return error.value ?? 'Try again in a moment.';
    default: return 'Check manually or let Shipyard check periodically in the background.';
  }
});

function onAutomaticChecksChange(event: Event) {
  setAutomaticChecksEnabled((event.target as HTMLInputElement).checked);
}

onMounted(() => void loadVersion());
</script>

<template>
  <div class="updates-section">
    <div class="section-heading">
      <div>
        <h3>Updates</h3>
        <p>Keep Shipyard current with signed macOS releases from GitHub.</p>
      </div>
      <AppButton
        variant="ghost"
        size="small"
        type="button"
        :loading="status === 'checking'"
        loading-label="Checking"
        :disabled="!isMacOS || status === 'installing'"
        @click="checkForUpdates()"
      >
        <RefreshCw aria-hidden="true" /> Check now
      </AppButton>
    </div>

    <div class="version-card">
      <span class="version-card__icon"><ShieldCheck aria-hidden="true" /></span>
      <div>
        <strong>Shipyard {{ currentVersion || '…' }}</strong>
        <p>Updates are verified before they are installed.</p>
      </div>
      <Check v-if="status === 'up-to-date'" class="version-card__check" aria-hidden="true" />
    </div>

    <label class="automatic-checks" :class="{ disabled: !isMacOS }">
      <span>
        <strong>Check for updates automatically</strong>
        <small>Check once when Shipyard opens and every six hours after that.</small>
      </span>
      <input
        type="checkbox"
        :checked="automaticChecksEnabled"
        :disabled="!isMacOS"
        @change="onAutomaticChecksChange"
      />
    </label>

    <p v-if="!isMacOS" class="notice">
      Automatic updates are configured for the macOS build. This development environment is not macOS.
    </p>

    <div class="update-status" :class="`update-status--${status}`" aria-live="polite">
      <div class="update-status__heading">
        <span class="update-status__indicator"></span>
        <strong>{{ statusLabel }}</strong>
      </div>
      <p>{{ statusDescription }}</p>

      <div v-if="availableUpdate" class="available-update">
        <p v-if="availableUpdate.body" class="release-notes">{{ availableUpdate.body }}</p>
        <div v-if="status === 'installing' && downloadProgress !== null" class="download-progress">
          <div class="download-progress__label">
            <span>Downloading update</span>
            <span>{{ downloadProgress }}%</span>
          </div>
          <progress max="100" :value="downloadProgress">{{ downloadProgress }}%</progress>
        </div>
        <AppButton
          variant="primary"
          type="button"
          :loading="status === 'installing'"
          loading-label="Installing"
          :disabled="!isMacOS"
          @click="installUpdate"
        >
          <Download aria-hidden="true" /> Install and restart
        </AppButton>
      </div>
    </div>

    <p class="last-checked">{{ lastCheckedLabel }}</p>
  </div>
</template>

<style scoped>
h3 { margin: 0; font-size: 14px; font-weight: 550; }
p { display: block; margin: 5px 0 0; font-size: 11px; color: var(--text-secondary); }
.updates-section { min-width: 0; flex: 1; padding: 24px; overflow-y: auto; }
.section-heading { display: flex; align-items: start; justify-content: space-between; gap: 12px; margin-bottom: 20px; }
.section-heading :deep(.app-button svg) { width: 13px; height: 13px; }
.version-card { display: flex; align-items: center; gap: 12px; min-height: 72px; padding: 13px 14px; background: var(--surface-subtle); border: 1px solid var(--border-subtle); border-radius: 9px; }
.version-card__icon { display: grid; width: 38px; height: 38px; flex: 0 0 auto; place-items: center; color: var(--success); background: var(--success-subtle); border-radius: 9px; }
.version-card__icon svg { width: 19px; }
.version-card > div { min-width: 0; flex: 1; }
.version-card strong, .automatic-checks strong { display: block; font-size: 12px; font-weight: 550; }
.version-card__check { width: 16px; color: var(--success); }
.automatic-checks { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-top: 12px; padding: 13px 14px; border: 1px solid var(--border-subtle); border-radius: 9px; cursor: pointer; }
.automatic-checks:hover { background: var(--surface-hover); }
.automatic-checks.disabled { cursor: not-allowed; opacity: .52; }
.automatic-checks span { min-width: 0; }
.automatic-checks small { display: block; margin-top: 4px; font-size: 10px; color: var(--text-secondary); }
.automatic-checks input { width: 31px; height: 18px; flex: 0 0 auto; margin: 0; accent-color: var(--primary); }
.notice { margin-top: 10px; color: var(--warning); }
.update-status { margin-top: 16px; padding: 14px; border: 1px solid var(--border-subtle); border-radius: 9px; }
.update-status--up-to-date { border-color: var(--success-border); background: var(--success-subtle); }
.update-status--available { border-color: var(--primary-border); background: var(--primary-subtle); }
.update-status--error { border-color: var(--danger-border); background: var(--danger-subtle); }
.update-status__heading { display: flex; align-items: center; gap: 8px; }
.update-status__heading strong { font-size: 12px; font-weight: 550; }
.update-status__indicator { width: 7px; height: 7px; flex: 0 0 auto; background: var(--text-muted); border-radius: 50%; }
.update-status--checking .update-status__indicator, .update-status--installing .update-status__indicator { background: var(--primary); box-shadow: 0 0 0 3px var(--primary-subtle); }
.update-status--up-to-date .update-status__indicator { background: var(--success); }
.update-status--available .update-status__indicator { background: var(--primary); }
.update-status--error .update-status__indicator { background: var(--danger); }
.available-update { margin-top: 13px; }
.release-notes { max-height: 100px; padding: 9px; overflow-y: auto; white-space: pre-wrap; background: rgba(0, 0, 0, .14); border-radius: 6px; }
.available-update :deep(.app-button) { margin-top: 13px; }
.download-progress { margin-top: 13px; }
.download-progress__label { display: flex; justify-content: space-between; margin-bottom: 5px; font-size: 10px; color: var(--text-secondary); }
progress { width: 100%; height: 5px; appearance: none; border: 0; border-radius: 4px; }
progress::-webkit-progress-bar { background: var(--surface-input); border-radius: 4px; }
progress::-webkit-progress-value { background: var(--primary); border-radius: 4px; }
.last-checked { margin-top: 12px; font-size: 10px; color: var(--text-muted); }
</style>
