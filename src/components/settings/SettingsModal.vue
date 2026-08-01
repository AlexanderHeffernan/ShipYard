<script setup lang="ts">
import { X } from '@lucide/vue';
import { onBeforeUnmount, onMounted, useId } from 'vue';
import AppButton from '../ui/AppButton.vue';

withDefaults(
  defineProps<{
    title: string;
    subtitle: string;
    size?: 'standard' | 'large';
    navigationLabel: string;
  }>(),
  { size: 'standard' },
);

const emit = defineEmits<{ close: [] }>();
const titleId = useId();

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close');
}

onMounted(() => document.addEventListener('keydown', onKeydown));
onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown));
</script>

<template>
  <div class="settings-backdrop" @mousedown.self="emit('close')">
    <section
      class="settings-modal"
      :class="`settings-modal--${size}`"
      role="dialog"
      aria-modal="true"
      :aria-labelledby="titleId"
    >
      <header class="settings-header">
        <div>
          <h2 :id="titleId">{{ title }}</h2>
          <span>{{ subtitle }}</span>
        </div>
        <AppButton variant="ghost" size="icon" type="button" aria-label="Close settings" @click="emit('close')">
          <X aria-hidden="true" />
        </AppButton>
      </header>

      <div class="settings-layout">
        <nav class="settings-nav" :aria-label="navigationLabel">
          <slot name="navigation" />
        </nav>
        <slot />
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-backdrop {
  position: fixed;
  z-index: 30;
  inset: 0;
  display: grid;
  padding: 32px;
  place-items: center;
  background: var(--surface-scrim);
  backdrop-filter: blur(5px);
}

.settings-modal {
  display: flex;
  width: min(720px, 90vw);
  height: min(500px, 82vh);
  flex-direction: column;
  overflow: hidden;
  background: var(--surface-elevated);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  box-shadow: 0 24px 80px rgba(5, 3, 8, 0.6);
}

.settings-modal--large {
  width: min(820px, 90vw);
  height: min(570px, 84vh);
}

.settings-header {
  display: flex;
  flex: 0 0 64px;
  align-items: center;
  justify-content: space-between;
  padding: 0 15px 0 20px;
  border-bottom: 1px solid var(--border-subtle);
}

.settings-modal--large .settings-header {
  flex-basis: 58px;
}

.settings-header h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 550;
}

.settings-header span {
  display: block;
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-secondary);
}

.settings-layout {
  display: flex;
  min-height: 0;
  flex: 1;
}

.settings-nav {
  flex: 0 0 160px;
  padding: 12px 9px;
  background: var(--surface-subtle);
  border-right: 1px solid var(--border-subtle);
}

.settings-nav :deep(button) {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  height: 34px;
  padding: 0 10px;
  font: inherit;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: 0;
  border-radius: 6px;
}

.settings-nav :deep(button:hover),
.settings-nav :deep(button[aria-current='page']) {
  color: var(--text-primary);
  background: var(--surface-hover);
}

.settings-nav :deep(svg) {
  width: 14px;
  height: 14px;
  stroke-width: 1.7;
}
</style>
