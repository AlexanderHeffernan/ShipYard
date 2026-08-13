<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import { cancelCheckout, checkoutPullRequest } from '../../services/projects';
import type { Project, WorkItem } from '../../types/projects';

const CHECKOUT_TIMEOUT_MS = 100_000;

const props = defineProps<{ project: Project; workItem: WorkItem }>();
const emit = defineEmits<{ checkedOut: [] }>();
const checkingOut = ref(false);
const cancelling = ref(false);
const error = ref<string | null>(null);
const operationId = ref<string | null>(null);

const unavailable = computed(() => !props.workItem.pullRequest || !props.project.githubRepository);
const retryable = computed(() => !checkingOut.value && !unavailable.value && !!error.value);

async function checkout() {
  const pullRequest = props.workItem.pullRequest;
  if (!pullRequest || checkingOut.value) return;
  const id = createOperationId();
  operationId.value = id;
  checkingOut.value = true;
  cancelling.value = false;
  error.value = null;
  let timeout: number | undefined;
  try {
    const request = checkoutPullRequest(id, {
      projectId: props.project.id,
      projectPath: props.project.path,
      pullRequestNumber: pullRequest.number,
      headSha: pullRequest.headSha,
      headBranch: pullRequest.headBranch,
    });
    // The timeout race below intentionally outlives this promise. Attach a
    // rejection handler now so a late backend response cannot become an
    // unhandled rejection after the UI has already recovered.
    void request.catch(() => undefined);
    await Promise.race([
      request,
      new Promise<never>((_, reject) => {
        timeout = window.setTimeout(() => {
          cancelling.value = true;
          void cancelCheckout(id).catch(() => undefined);
          reject(new Error('Checkout timed out after 100 seconds. A cancellation request was sent; retry when it finishes.'));
        }, CHECKOUT_TIMEOUT_MS);
      }),
    ]);
    emit('checkedOut');
  } catch (checkoutError) {
    error.value = checkoutErrorMessage(checkoutError);
  } finally {
    if (timeout !== undefined) window.clearTimeout(timeout);
    checkingOut.value = false;
    cancelling.value = false;
    operationId.value = null;
  }
}

async function cancel() {
  const id = operationId.value;
  if (!id || !checkingOut.value || cancelling.value) return;
  cancelling.value = true;
  try {
    await cancelCheckout(id);
  } catch (cancelError) {
    cancelling.value = false;
    error.value = checkoutErrorMessage(cancelError);
  }
}

function createOperationId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID();
  return `checkout-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function checkoutErrorMessage(checkoutError: unknown) {
  const message = checkoutError instanceof Error ? checkoutError.message : String(checkoutError);
  if (/cancelled/i.test(message)) return 'Checkout cancelled. You can retry it when ready.';
  if (/already being checked out/i.test(message)) {
    return 'This pull request is already being checked out. Wait for the other attempt to finish, then retry.';
  }
  if (/credential|authentication|permission denied|could not read username|terminal prompts disabled|repository not found/i.test(message)) {
    return `${message} Check that Git credentials are configured for this repository, then retry.`;
  }
  return message;
}

onBeforeUnmount(() => {
  const id = operationId.value;
  if (id && checkingOut.value && !props.workItem.worktreePath) {
    void cancelCheckout(id).catch(() => undefined);
  }
});
</script>

<template>
  <div class="checkout-control">
    <div class="checkout-control__actions">
      <button type="button" :disabled="unavailable || checkingOut" :title="unavailable ? 'This pull request is not available for checkout' : 'Create a local checkout for this pull request'" @click="checkout">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 2.75v7.5m0 0L5.25 7.5M8 10.25l2.75-2.75M3.25 10.75v2h9.5v-2" /></svg>
        <span>{{ checkingOut ? 'Checking out…' : 'Check out' }}</span>
      </button>
      <button v-if="checkingOut" class="checkout-control__cancel" type="button" :disabled="cancelling" @click="cancel">
        {{ cancelling ? 'Cancelling…' : 'Cancel' }}
      </button>
    </div>
    <div v-if="error" class="checkout-control__feedback" role="alert" aria-live="polite">
      <p>{{ error }}</p>
      <button v-if="retryable" type="button" @click="checkout">Retry</button>
    </div>
  </div>
</template>

<style scoped>
.checkout-control { position: relative; }
.checkout-control__actions { display: flex; align-items: center; gap: 5px; }
.checkout-control button { display: flex; align-items: center; gap: 5px; height: 24px; padding: 0 8px; font: inherit; font-size: 11px; font-weight: 600; color: var(--primary-foreground); background: var(--primary); border: 1px solid var(--primary); border-radius: 6px; }
.checkout-control button:hover:not(:disabled) { background: var(--primary-hover); border-color: var(--primary-hover); }
.checkout-control button:disabled { opacity: .45; }
.checkout-control button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
.checkout-control__cancel { color: var(--text-secondary) !important; background: var(--surface-subtle) !important; border-color: var(--border-strong) !important; }
.checkout-control svg { width: 13px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.35; }
.checkout-control__feedback { position: absolute; z-index: 7; top: 25px; right: 0; width: 250px; margin: 4px 0 0; padding: 7px; font-size: 10px; color: var(--danger); background: var(--surface-elevated); border: 1px solid var(--border-strong); border-radius: 6px; box-shadow: var(--shadow-elevated); }
.checkout-control__feedback p { margin: 0; line-height: 1.35; }
.checkout-control__feedback button { height: 21px; margin-top: 6px; padding: 0 7px; font-size: 10px; color: var(--text-primary); background: var(--surface-subtle); border-color: var(--border-strong); }
@media (max-width: 680px) { .checkout-control button { width: 26px; justify-content: center; padding: 0; } .checkout-control span { display: none; } }
</style>
