<script setup lang="ts">
import { Square, TerminalSquare, X } from '@lucide/vue';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';
import { openUrl } from '@tauri-apps/plugin-opener';
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import AppButton from '../ui/AppButton.vue';
import { useRunner } from '../../composables/useRunner';

const MIN_HEIGHT = 120;
const DEFAULT_HEIGHT = 220;

const props = defineProps<{ projectId: string }>();
const { currentRun, cancel, sendInput, resize, clear } = useRunner();
const terminalHost = ref<HTMLElement>();
const height = ref(DEFAULT_HEIGHT);
const visible = computed(() => currentRun.value?.projectId === props.projectId);
const active = computed(() => ['running', 'stopping'].includes(currentRun.value?.status ?? ''));
const interactive = computed(() => currentRun.value?.status === 'running');

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let resizeObserver: ResizeObserver | null = null;
let renderedOutput = '';
let dragStartY = 0;
let dragStartHeight = DEFAULT_HEIGHT;
let resizeTimer: number | undefined;

function mountTerminal() {
  if (!terminalHost.value || terminal) return;
  terminal = new Terminal({
    allowTransparency: true,
    cursorBlink: interactive.value,
    disableStdin: !interactive.value,
    fontFamily: 'SFMono-Regular, Menlo, Monaco, monospace',
    fontSize: 11,
    lineHeight: 1.35,
    scrollback: 10_000,
    theme: {
      background: 'rgba(0, 0, 0, 0)',
      foreground: '#c5c8d0',
      cursor: '#c5c8d0',
      selectionBackground: '#3a4966',
    },
  });
  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(new WebLinksAddon((event, uri) => {
    event.preventDefault();
    void openUrl(uri);
  }));
  terminal.onData(sendInput);
  terminal.onResize(({ cols, rows }) => scheduleTerminalResize(cols, rows));
  terminal.open(terminalHost.value);
  resizeObserver = new ResizeObserver(() => fitAddon?.fit());
  resizeObserver.observe(terminalHost.value);
  syncOutput(currentRun.value?.output ?? '');
  requestAnimationFrame(() => fitAddon?.fit());
}

function unmountTerminal() {
  window.clearTimeout(resizeTimer);
  resizeObserver?.disconnect();
  terminal?.dispose();
  resizeObserver = null;
  terminal = null;
  fitAddon = null;
  renderedOutput = '';
}

function scheduleTerminalResize(columns: number, rows: number) {
  window.clearTimeout(resizeTimer);
  resizeTimer = window.setTimeout(() => resize(columns, rows), 30);
}

function syncOutput(output: string) {
  if (!terminal) return;
  if (output.startsWith(renderedOutput)) {
    terminal.write(output.slice(renderedOutput.length));
  } else {
    terminal.reset();
    terminal.write(output);
  }
  renderedOutput = output;
}

function clampHeight(value: number) {
  const maxHeight = Math.max(MIN_HEIGHT, window.innerHeight - 180);
  return Math.min(maxHeight, Math.max(MIN_HEIGHT, value));
}

function resizeTerminal(event: PointerEvent) {
  height.value = clampHeight(dragStartHeight + dragStartY - event.clientY);
}

function stopResize() {
  document.body.classList.remove('is-resizing-terminal');
  window.removeEventListener('pointermove', resizeTerminal);
  window.removeEventListener('pointerup', stopResize);
  window.removeEventListener('pointercancel', stopResize);
}

function startResize(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  dragStartY = event.clientY;
  dragStartHeight = height.value;
  document.body.classList.add('is-resizing-terminal');
  window.addEventListener('pointermove', resizeTerminal);
  window.addEventListener('pointerup', stopResize);
  window.addEventListener('pointercancel', stopResize);
}

function resizeWithKeyboard(event: KeyboardEvent) {
  if (!['ArrowUp', 'ArrowDown'].includes(event.key)) return;
  event.preventDefault();
  height.value = clampHeight(height.value + (event.key === 'ArrowUp' ? 20 : -20));
}

watch(
  () => currentRun.value?.output,
  (output) => syncOutput(output ?? ''),
  { flush: 'post' },
);

watch(interactive, (isInteractive) => {
  if (!terminal) return;
  terminal.options.disableStdin = !isInteractive;
  terminal.options.cursorBlink = isInteractive;
});

watch(
  visible,
  async (isVisible) => {
    if (!isVisible) return unmountTerminal();
    await nextTick();
    mountTerminal();
  },
  { immediate: true, flush: 'post' },
);

onBeforeUnmount(() => {
  stopResize();
  unmountTerminal();
});
</script>

<template>
  <section
    v-if="visible && currentRun"
    class="run-console"
    :style="{ height: `${height}px` }"
  >
    <div
      class="run-console__resize-handle"
      role="separator"
      aria-label="Resize terminal"
      aria-orientation="horizontal"
      :aria-valuemin="MIN_HEIGHT"
      :aria-valuenow="height"
      tabindex="0"
      @pointerdown="startResize"
      @keydown="resizeWithKeyboard"
    ></div>
    <header>
      <TerminalSquare class="run-console__terminal-icon" aria-hidden="true" />
      <span class="run-console__status" :class="`run-console__status--${currentRun.status}`"></span>
      <strong>{{ currentRun.scriptLabel }}</strong>
      <span class="run-console__status-label">{{ currentRun.status }}</span>
      <div class="run-console__actions">
        <AppButton
          v-if="active"
          variant="danger"
          size="small"
          type="button"
          :disabled="currentRun.status === 'stopping'"
          @click="cancel"
        >
          <Square aria-hidden="true" />
          {{ currentRun.status === 'stopping' ? 'Stopping…' : 'Stop' }}
        </AppButton>
        <AppButton
          v-else
          variant="ghost"
          size="small"
          type="button"
          @click="clear"
        >
          <X aria-hidden="true" />
          Close
        </AppButton>
      </div>
    </header>
    <div class="run-console__body">
      <div ref="terminalHost" class="run-console__terminal"></div>
      <span v-if="!currentRun.output" class="run-console__waiting">Waiting for output…</span>
    </div>
  </section>
</template>

<style scoped>
.run-console {
  position: relative;
  display: flex;
  flex: 0 0 auto;
  min-height: 0;
  flex-direction: column;
  background: transparent;
  border-top: 1px solid var(--border-subtle);
}

.run-console__resize-handle {
  position: absolute;
  z-index: 2;
  top: -3px;
  right: 0;
  left: 0;
  height: 7px;
  cursor: row-resize;
}

.run-console__resize-handle::after {
  position: absolute;
  top: 3px;
  right: 0;
  left: 0;
  height: 1px;
  content: '';
  background: transparent;
  transition: background 100ms ease;
}

.run-console__resize-handle:hover::after,
.run-console__resize-handle:focus-visible::after {
  background: var(--resize-indicator);
}

.run-console__resize-handle:focus-visible {
  outline: 0;
}

.run-console header {
  display: flex;
  flex: 0 0 34px;
  align-items: center;
  gap: 7px;
  padding: 0 7px 0 11px;
  font-size: 10px;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border-subtle);
}

.run-console strong {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-primary);
}

.run-console__terminal-icon {
  width: 13px;
  height: 13px;
  margin-right: 1px;
  stroke-width: 1.6;
}

.run-console__actions {
  display: flex;
  margin-left: auto;
}

.run-console__status {
  width: 7px;
  height: 7px;
  background: #e7b950;
  border-radius: 50%;
}

.run-console__status-label {
  text-transform: capitalize;
}

.run-console__status--succeeded {
  background: #64cf8c;
}

.run-console__status--failed,
.run-console__status--cancelled,
.run-console__status--stopping {
  background: #ff7777;
}

.run-console__body {
  position: relative;
  min-height: 0;
  flex: 1;
  padding: 9px 4px 5px 11px;
  overflow: hidden;
}

.run-console__terminal {
  width: 100%;
  height: 100%;
}

.run-console__waiting {
  position: absolute;
  top: 11px;
  left: 13px;
  font: 11px/1.35 SFMono-Regular, Menlo, Monaco, monospace;
  color: rgba(255, 255, 255, 0.36);
  pointer-events: none;
}

.run-console :deep(.xterm) {
  height: 100%;
}

.run-console :deep(.xterm-viewport) {
  background: transparent;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.14) transparent;
}

.run-console :deep(.xterm-viewport::-webkit-scrollbar) {
  width: 6px;
}

.run-console :deep(.xterm-viewport::-webkit-scrollbar-track) {
  background: transparent;
}

.run-console :deep(.xterm-viewport::-webkit-scrollbar-thumb) {
  background: rgba(255, 255, 255, 0.14);
  border-radius: 999px;
}

.run-console :deep(.xterm-viewport::-webkit-scrollbar-thumb:hover) {
  background: rgba(255, 255, 255, 0.24);
}
</style>
