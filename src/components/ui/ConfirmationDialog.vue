<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, useId } from 'vue';
import AppButton from './AppButton.vue';

const props = withDefaults(
  defineProps<{
    title: string;
    description: string;
    confirmLabel: string;
    loading?: boolean;
    loadingLabel?: string;
    confirmDisabled?: boolean;
    error?: string | null;
  }>(),
  {
    loading: false,
    loadingLabel: 'Deleting',
    confirmDisabled: false,
    error: null,
  },
);

const emit = defineEmits<{ cancel: []; confirm: [] }>();
const dialog = ref<HTMLElement>();
const cancelButton = ref<InstanceType<typeof AppButton>>();
const titleId = useId();
const descriptionId = useId();
const errorId = useId();
const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;

function cancel() {
  if (!props.loading) emit('cancel');
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault();
    cancel();
    return;
  }
  if (event.key !== 'Tab' || !dialog.value) return;
  const focusable = Array.from(
    dialog.value.querySelectorAll<HTMLElement>('button:not(:disabled), [href], [tabindex]:not([tabindex="-1"])'),
  );
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown);
  void nextTick(() => cancelButton.value?.$el.focus());
});
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown);
  previousFocus?.focus();
});
</script>

<template>
  <Teleport to="body">
    <div class="confirmation-backdrop" @mousedown.self="cancel">
      <section
        ref="dialog"
        class="confirmation-dialog"
        role="alertdialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        :aria-describedby="`${descriptionId}${error ? ` ${errorId}` : ''}`"
        :aria-busy="loading || undefined"
      >
        <header>
          <h2 :id="titleId">{{ title }}</h2>
          <p :id="descriptionId">{{ description }}</p>
        </header>

        <div class="confirmation-dialog__content">
          <slot />
          <p v-if="error" :id="errorId" class="confirmation-dialog__error" role="alert">{{ error }}</p>
        </div>

        <footer>
          <AppButton ref="cancelButton" type="button" :disabled="loading" @click="cancel">
            Cancel
          </AppButton>
          <AppButton
            variant="danger"
            type="button"
            :disabled="confirmDisabled"
            :loading="loading"
            :loading-label="loadingLabel"
            @click="emit('confirm')"
          >
            {{ confirmLabel }}
          </AppButton>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.confirmation-backdrop {
  position: fixed;
  z-index: 40;
  inset: 0;
  display: grid;
  padding: 24px;
  place-items: center;
  background: var(--surface-scrim);
  backdrop-filter: blur(5px);
}

.confirmation-dialog {
  width: min(440px, calc(100vw - 32px));
  overflow: hidden;
  background: var(--surface-elevated);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  box-shadow: 0 24px 80px rgba(5, 3, 8, 0.6);
}

.confirmation-dialog header {
  padding: 20px 20px 15px;
  border-bottom: 1px solid var(--border-subtle);
}

.confirmation-dialog h2 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}

.confirmation-dialog header p {
  margin: 7px 0 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.confirmation-dialog__content {
  padding: 16px 20px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.confirmation-dialog__content :deep(p) {
  margin: 0;
}

.confirmation-dialog__content :deep(ul) {
  margin: 10px 0 0;
  padding-left: 18px;
}

.confirmation-dialog__content :deep(code) {
  color: var(--text-primary);
  overflow-wrap: anywhere;
}

.confirmation-dialog__error {
  margin-top: 13px !important;
  padding: 9px 10px;
  color: var(--danger);
  background: var(--danger-subtle);
  border: 1px solid var(--danger-border);
  border-radius: 6px;
}

.confirmation-dialog footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  background: var(--surface-subtle);
  border-top: 1px solid var(--border-subtle);
}

.confirmation-dialog footer :deep(.app-button--danger) {
  color: var(--danger);
  background: var(--danger-subtle);
  border-color: var(--danger-border);
}

.confirmation-dialog footer :deep(.app-button--danger:hover:not(:disabled)) {
  color: var(--danger);
  background: var(--danger-subtle);
  border-color: var(--danger);
}

@media (max-width: 520px) {
  .confirmation-backdrop {
    padding: 16px;
  }

  .confirmation-dialog footer {
    flex-direction: column-reverse;
  }

  .confirmation-dialog footer :deep(.app-button) {
    width: 100%;
  }
}
</style>
