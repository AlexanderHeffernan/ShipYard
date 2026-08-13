<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { cancelCheckout, checkoutPullRequest } from '../../services/projects';
import type { Project, WorkItem } from '../../types/projects';
import { checkoutButtonState, type CheckoutPhase } from '../../utils/checkoutState';

const CHECKOUT_TIMEOUT_MS = 100_000;
const CANCELLATION_TIMEOUT_MS = 15_000;
const REFRESH_TIMEOUT_MS = 15_000;

const props = defineProps<{ project: Project; workItem: WorkItem }>();
const emit = defineEmits<{ checkedOut: [] }>();
const phase = ref<CheckoutPhase>('idle');
const error = ref<string | null>(null);
const operationId = ref<string | null>(null);
let refreshTimeout: number | undefined;
let cancellationTimeout: number | undefined;
let disposed = false;

const unavailable = computed(() => !props.workItem.pullRequest || !props.project.githubRepository);
const busy = computed(() => phase.value !== 'idle');
const retryable = computed(() => !busy.value && !unavailable.value && !!error.value);
const button = computed(() => checkoutButtonState(phase.value, unavailable.value));

async function checkout() {
  const pullRequest = props.workItem.pullRequest;
  if (!pullRequest || phase.value !== 'idle') return;
  const id = createOperationId();
  operationId.value = id;
  phase.value = 'checking';
  error.value = null;
  clearCancellationTimeout();
  let commandTimeout: number | undefined;
  let timeoutRequested = false;
  try {
    const request = checkoutPullRequest(id, {
      projectId: props.project.id,
      projectPath: props.project.path,
      pullRequestNumber: pullRequest.number,
      headSha: pullRequest.headSha,
      headBranch: pullRequest.headBranch,
    });
    // Keep a rejection handler attached even if the component is unmounted
    // while the backend is still finishing the operation.
    void request.catch(() => undefined);
    commandTimeout = window.setTimeout(() => {
      if (operationId.value !== id || phase.value !== 'checking') return;
      timeoutRequested = true;
      beginCancellationWait(id);
      void cancelCheckout(id).catch((cancelError) => {
        if (operationId.value !== id || phase.value !== 'cancelling') return;
        phase.value = 'checking';
        error.value = checkoutErrorMessage(cancelError);
      });
    }, CHECKOUT_TIMEOUT_MS);
    await request;
    if (disposed || operationId.value !== id) return;
    clearCancellationTimeout();
    phase.value = 'finishing';
    operationId.value = null;
    refreshTimeout = window.setTimeout(() => {
      if (phase.value !== 'finishing' || props.workItem.worktreePath) return;
      phase.value = 'idle';
      error.value = 'Checkout completed, but the project view did not refresh. Retry to reconcile it.';
    }, REFRESH_TIMEOUT_MS);
    emit('checkedOut');
  } catch (checkoutError) {
    if (!disposed && operationId.value === id) {
      error.value = checkoutErrorMessage(checkoutError, timeoutRequested);
    }
  } finally {
    if (commandTimeout !== undefined) window.clearTimeout(commandTimeout);
    if (operationId.value === id && phase.value !== 'finishing') {
      clearCancellationTimeout();
      phase.value = 'idle';
      operationId.value = null;
    }
  }
}

async function cancel() {
  const id = operationId.value;
  if (!id || phase.value !== 'checking') return;
  beginCancellationWait(id);
  try {
    await cancelCheckout(id);
  } catch (cancelError) {
    if (disposed || operationId.value !== id) return;
    const currentPhase = phase.value as CheckoutPhase;
    if (currentPhase === 'cancelling') phase.value = 'checking';
    error.value = checkoutErrorMessage(cancelError);
  }
}

function beginCancellationWait(id: string) {
  if (operationId.value !== id) return;
  phase.value = 'cancelling';
  clearCancellationTimeout();
  cancellationTimeout = window.setTimeout(() => {
    if (operationId.value !== id || !['checking', 'cancelling'].includes(phase.value)) return;
    phase.value = 'recovering';
    error.value = 'Checkout cancellation is taking longer than expected. Shipyard is still waiting for Git to stop; retry will be available when it finishes.';
  }, CANCELLATION_TIMEOUT_MS);
}

function clearCancellationTimeout() {
  if (cancellationTimeout !== undefined) window.clearTimeout(cancellationTimeout);
  cancellationTimeout = undefined;
}

function handleButtonClick() {
  if (phase.value === 'checking') {
    void cancel();
  } else if (phase.value === 'idle') {
    void checkout();
  }
}

function createOperationId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID();
  return `checkout-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function checkoutErrorMessage(checkoutError: unknown, timeoutRequested = false) {
  const message = checkoutError instanceof Error ? checkoutError.message : String(checkoutError);
  if (timeoutRequested && /cancelled|timed out/i.test(message)) {
    return 'Checkout took too long and was cancelled. You can retry it when ready.';
  }
  if (/cancelled/i.test(message)) return 'Checkout cancelled. You can retry it when ready.';
  if (/already being checked out/i.test(message)) {
    return 'This pull request is already being checked out. Wait for the other attempt to finish, then retry.';
  }
  if (/credential|authentication|permission denied|could not read username|terminal prompts disabled|repository not found/i.test(message)) {
    return `${message} Check that Git credentials are configured for this repository, then retry.`;
  }
  return message;
}

watch(
  () => props.workItem.worktreePath,
  (worktreePath) => {
    if (!worktreePath || phase.value !== 'finishing') return;
    if (refreshTimeout !== undefined) window.clearTimeout(refreshTimeout);
    refreshTimeout = undefined;
  },
);

onBeforeUnmount(() => {
  disposed = true;
  if (refreshTimeout !== undefined) window.clearTimeout(refreshTimeout);
  clearCancellationTimeout();
  const id = operationId.value;
  if (id && ['checking', 'cancelling', 'recovering'].includes(phase.value) && !props.workItem.worktreePath) {
    void cancelCheckout(id).catch(() => undefined);
  }
});
</script>

<template>
  <div class="checkout-control">
    <button
      class="checkout-control__button"
      :class="{ 'checkout-control__button--cancellable': button.cancellable }"
      type="button"
      :disabled="button.disabled"
      :title="button.title"
      :aria-label="button.title"
      @click="handleButtonClick"
    >
      <span class="checkout-control__button-content checkout-control__button-content--default" aria-hidden="true">
        <svg viewBox="0 0 16 16"><path v-if="phase === 'idle'" d="M8 2.75v7.5m0 0L5.25 7.5M8 10.25l2.75-2.75M3.25 10.75v2h9.5v-2" /><path v-else d="M5 5h6v6H5z" /></svg>
        <span>{{ button.label }}</span>
      </span>
      <span v-if="button.hoverLabel" class="checkout-control__button-content checkout-control__button-content--cancel" aria-hidden="true">
        <svg viewBox="0 0 16 16"><path d="m5 5 6 6m0-6-6 6" /></svg>
        <span>{{ button.hoverLabel }}</span>
      </span>
    </button>
    <div v-if="error" class="checkout-control__feedback" role="alert" aria-live="polite">
      <p>{{ error }}</p>
      <button v-if="retryable" type="button" @click="checkout">Retry</button>
    </div>
  </div>
</template>

<style scoped>
.checkout-control { position: relative; }
.checkout-control button { display: flex; align-items: center; gap: 5px; height: 24px; padding: 0 8px; font: inherit; font-size: 11px; font-weight: 600; color: var(--primary-foreground); background: var(--primary); border: 1px solid var(--primary); border-radius: 6px; }
.checkout-control button:hover:not(:disabled) { background: var(--primary-hover); border-color: var(--primary-hover); }
.checkout-control button:disabled { opacity: .45; }
.checkout-control button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
.checkout-control__button-content { display: flex; align-items: center; gap: 5px; }
.checkout-control__button-content--cancel { display: none; }
.checkout-control__button--cancellable:hover .checkout-control__button-content--default,
.checkout-control__button--cancellable:focus-visible .checkout-control__button-content--default { display: none; }
.checkout-control__button--cancellable:hover .checkout-control__button-content--cancel,
.checkout-control__button--cancellable:focus-visible .checkout-control__button-content--cancel { display: flex; }
.checkout-control svg { width: 13px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.35; }
.checkout-control__feedback { position: absolute; z-index: 7; top: 25px; right: 0; width: 250px; margin: 4px 0 0; padding: 7px; font-size: 10px; color: var(--danger); background: var(--surface-elevated); border: 1px solid var(--border-strong); border-radius: 6px; box-shadow: var(--shadow-elevated); }
.checkout-control__feedback p { margin: 0; line-height: 1.35; }
.checkout-control__feedback button { height: 21px; margin-top: 6px; padding: 0 7px; font-size: 10px; color: var(--text-primary); background: var(--surface-subtle); border-color: var(--border-strong); }
@media (max-width: 680px) { .checkout-control button { width: 26px; justify-content: center; padding: 0; } .checkout-control span { display: none; } }
@media (hover: none) { .checkout-control__button--cancellable .checkout-control__button-content--default { display: none; } .checkout-control__button--cancellable .checkout-control__button-content--cancel { display: flex; } }
</style>
