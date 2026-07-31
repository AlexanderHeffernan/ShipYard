<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'default' | 'ghost' | 'primary' | 'danger';
    size?: 'small' | 'medium' | 'icon';
    block?: boolean;
  }>(),
  {
    variant: 'default',
    size: 'medium',
    block: false,
  },
);
</script>

<template>
  <button
    class="app-button"
    :class="[`app-button--${variant}`, `app-button--${size}`, { 'app-button--block': block }]"
  >
    <slot />
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
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  transition: background 100ms ease, border-color 100ms ease, color 100ms ease;
}

.app-button:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--surface-hover);
}

.app-button:active:not(:disabled) {
  background: rgba(255, 255, 255, 0.1);
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
  color: rgba(255, 255, 255, 0.94);
  background: #3478d4;
  border-color: #3f82dd;
}

.app-button--primary:hover:not(:disabled) {
  color: white;
  background: #3c83e0;
  border-color: #4a8ce8;
}

.app-button--danger {
  color: #ff9292;
  background: transparent;
  border-color: transparent;
}

.app-button--danger:hover:not(:disabled) {
  color: #ffaaaa;
  background: rgba(255, 100, 100, 0.1);
}

.app-button :deep(svg) {
  width: 14px;
  height: 14px;
  flex: 0 0 auto;
  stroke-width: 1.7;
}
</style>
