<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue';

const MIN_WIDTH = 220;
const MAX_WIDTH = 420;

const props = defineProps<{
  open: boolean;
  width: number;
}>();

const emit = defineEmits<{
  'update:width': [value: number];
}>();

const isResizing = ref(false);

function clampWidth(width: number) {
  return Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, width));
}

function resizeTo(clientX: number) {
  emit('update:width', clampWidth(clientX));
}

function stopResize() {
  isResizing.value = false;
  document.body.classList.remove('is-resizing-sidebar');
  window.removeEventListener('pointermove', onPointerMove);
  window.removeEventListener('pointerup', stopResize);
  window.removeEventListener('pointercancel', stopResize);
}

function onPointerMove(event: PointerEvent) {
  resizeTo(event.clientX);
}

function startResize(event: PointerEvent) {
  event.preventDefault();
  isResizing.value = true;
  document.body.classList.add('is-resizing-sidebar');
  window.addEventListener('pointermove', onPointerMove);
  window.addEventListener('pointerup', stopResize);
  window.addEventListener('pointercancel', stopResize);
}

function resizeWithKeyboard(event: KeyboardEvent) {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
  event.preventDefault();
  const delta = event.key === 'ArrowLeft' ? -10 : 10;
  emit('update:width', clampWidth(props.width + delta));
}

onBeforeUnmount(stopResize);
</script>

<template>
  <aside
    class="sidebar"
    :class="{
      'sidebar--closed': !open,
      'sidebar--resizing': isResizing,
    }"
    :style="{ width: open ? `${width}px` : '0px' }"
    :aria-hidden="!open"
    :inert="!open"
  >
    <div class="sidebar__body" :style="{ width: `${width}px` }">
      <header class="sidebar__header">
        <h1>Projects</h1>
        <button class="icon-button" type="button" aria-label="Add project" title="Add project">
          <svg viewBox="0 0 20 20" aria-hidden="true">
            <path d="M10 4.25v11.5M4.25 10h11.5" />
          </svg>
        </button>
      </header>

      <nav class="sidebar__content" aria-label="Projects"></nav>
    </div>

    <div
      class="sidebar__resize-handle"
      role="separator"
      aria-label="Resize sidebar"
      aria-orientation="vertical"
      :aria-valuemin="MIN_WIDTH"
      :aria-valuemax="MAX_WIDTH"
      :aria-valuenow="width"
      tabindex="0"
      @pointerdown="startResize"
      @keydown="resizeWithKeyboard"
    />
  </aside>
</template>

<style scoped>
.sidebar {
  position: relative;
  z-index: 2;
  flex: 0 0 auto;
  height: 100%;
  overflow: hidden;
  user-select: none;
  border-right: 1px solid var(--border-subtle);
  background: var(--surface-sidebar);
  box-shadow: 1px 0 0 rgba(0, 0, 0, 0.12);
  transition:
    width 180ms cubic-bezier(0.2, 0.75, 0.25, 1),
    border-color 180ms ease,
    box-shadow 180ms ease;
}

.sidebar--closed {
  border-color: transparent;
  box-shadow: none;
}

.sidebar--resizing {
  transition: none;
}

.sidebar__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 52px;
  padding: 0 12px 0 16px;
}

.sidebar__header h1 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.sidebar__body {
  height: 100%;
  padding-top: var(--titlebar-height);
  overflow: hidden;
}

.sidebar__content {
  height: calc(100% - 52px);
  overflow-y: auto;
}

.icon-button {
  display: grid;
  flex: 0 0 auto;
  width: 28px;
  height: 28px;
  padding: 0;
  place-items: center;
  color: var(--text-secondary);
  background: transparent;
  border: 0;
  border-radius: 7px;
  cursor: default;
}

.icon-button:hover {
  color: var(--text-primary);
  background: var(--surface-hover);
}

.icon-button:focus-visible,
.sidebar__resize-handle:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 1px;
}

.icon-button svg {
  width: 17px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.35;
}

.sidebar__resize-handle {
  position: absolute;
  z-index: 3;
  top: 0;
  right: -4px;
  bottom: 0;
  width: 8px;
  cursor: col-resize;
  touch-action: none;
}

.sidebar--closed .sidebar__resize-handle {
  display: none;
}

.sidebar__resize-handle::after {
  position: absolute;
  top: 0;
  right: 3px;
  bottom: 0;
  width: 1px;
  content: '';
  background: transparent;
  transition: background 120ms ease;
}

.sidebar__resize-handle:hover::after,
.sidebar--resizing .sidebar__resize-handle::after {
  background: var(--resize-indicator);
}
</style>
