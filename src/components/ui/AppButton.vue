<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'default' | 'ghost' | 'primary' | 'danger';
    size?: 'small' | 'medium' | 'icon';
    block?: boolean;
    disabled?: boolean;
    loading?: boolean;
    loadingLabel?: string;
    success?: boolean;
    successLabel?: string;
  }>(),
  {
    variant: 'default',
    size: 'medium',
    block: false,
    disabled: false,
    loading: false,
    success: false,
  },
);
</script>

<template>
  <button
    class="app-button"
    :class="[
      `app-button--${variant}`,
      `app-button--${size}`,
      { 'app-button--block': block, 'app-button--success': success },
    ]"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
  >
    <svg v-if="loading" class="app-button__spinner" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="8" r="5.5" />
    </svg>
    <svg v-else-if="success" class="app-button__check" viewBox="0 0 16 16" aria-hidden="true">
      <path d="m3.5 8.25 3 3 6-6.5" />
    </svg>
    <span v-if="loading && loadingLabel">{{ loadingLabel }}</span>
    <span v-else-if="success && successLabel">{{ successLabel }}</span>
    <slot v-else />
  </button>
</template>

<style scoped>
.app-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 0 10px;
  font: inherit;
  font-size: 11px;
  color: var(--text-secondary);
  background: var(--surface-subtle);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  transition: background 100ms ease, border-color 100ms ease, color 100ms ease;
}

.app-button:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--surface-hover);
}

.app-button:active:not(:disabled) {
  background: var(--surface-active);
}

.app-button:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 1px;
}

.app-button:disabled {
  cursor: not-allowed;
  opacity: 0.48;
}

.app-button--small {
  height: 24px;
  padding: 0 8px;
}

.app-button--medium {
  height: 29px;
}

.app-button--icon {
  width: 28px;
  height: 28px;
  padding: 0;
}

.app-button--block {
  width: 100%;
}

.app-button--ghost {
  background: transparent;
  border-color: transparent;
}

.app-button--primary {
  color: var(--primary-foreground);
  background: var(--primary);
  border-color: var(--primary);
}

.app-button--primary:hover:not(:disabled) {
  color: var(--primary-foreground);
  background: var(--primary-hover);
  border-color: var(--primary-hover);
}

.app-button--danger {
  color: var(--danger);
  background: transparent;
  border-color: transparent;
}

.app-button--danger:hover:not(:disabled) {
  color: var(--danger);
  background: var(--danger-subtle);
}

.app-button--success {
  color: var(--success);
  border-color: var(--success-border);
}

.app-button :deep(svg) {
  width: 14px;
  height: 14px;
  flex: 0 0 auto;
  stroke-width: 1.7;
}

.app-button .app-button__spinner {
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-dasharray: 24 12;
  animation: app-button-spin 700ms linear infinite;
}

.app-button .app-button__check {
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
}

@keyframes app-button-spin {
  to { transform: rotate(360deg); }
}
</style>
