import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ref } from 'vue';
import {
  cancelRun,
  resizeRunTerminal,
  startRun,
  writeRunInput,
} from '../services/run';
import type { RunScript, RunState } from '../types/run';
import type { Project, WorkItem } from '../types/projects';
import { shipWork as startShipping, type ShippingAction } from '../services/ship';

type OutputEvent = { runId: string; data: number[] };
type FinishedEvent = { runId: string; exitCode: number | null; success: boolean };

const MAX_OUTPUT_LENGTH = 500_000;
const currentRun = ref<RunState | null>(null);
const error = ref<string | null>(null);
const pendingOutput = new Map<string, string>();
const pendingFinished = new Map<string, FinishedEvent>();
const outputDecoders = new Map<string, TextDecoder>();
let listenersPromise: Promise<UnlistenFn[]> | null = null;
let inputQueue = Promise.resolve();

function ensureListeners() {
  listenersPromise ??= Promise.all([
    listen<OutputEvent>('run-output', ({ payload }) => {
      const decoder = outputDecoder(payload.runId);
      appendOutput(payload.runId, decoder.decode(new Uint8Array(payload.data), { stream: true }));
    }),
    listen<FinishedEvent>('run-finished', ({ payload }) => {
      flushOutputDecoder(payload.runId);
      if (currentRun.value?.runId === payload.runId) finishRun(payload);
      else pendingFinished.set(payload.runId, payload);
    }),
  ]);
  return listenersPromise;
}

function outputDecoder(runId: string) {
  const existing = outputDecoders.get(runId);
  if (existing) return existing;
  const decoder = new TextDecoder();
  outputDecoders.set(runId, decoder);
  return decoder;
}

function flushOutputDecoder(runId: string) {
  const decoder = outputDecoders.get(runId);
  if (decoder) appendOutput(runId, decoder.decode());
  outputDecoders.delete(runId);
}

function appendOutput(runId: string, chunk: string) {
  if (!chunk) return;
  if (currentRun.value?.runId === runId) {
    const output = limitedOutput(currentRun.value.output + chunk);
    currentRun.value = { ...currentRun.value, output };
    return;
  }
  pendingOutput.set(runId, limitedOutput((pendingOutput.get(runId) ?? '') + chunk));
}

function limitedOutput(output: string) {
  return output.slice(-MAX_OUTPUT_LENGTH);
}

function finishRun(event: FinishedEvent) {
  if (!currentRun.value) return;
  const cancelled = currentRun.value.status === 'stopping';
  currentRun.value = {
    ...currentRun.value,
    status: cancelled ? 'cancelled' : event.success ? 'succeeded' : 'failed',
    exitCode: event.exitCode,
  };
}

export function useRunner() {
  async function shipWork(project: Project, item: WorkItem, action: ShippingAction) {
    if (currentRun.value && ['running', 'stopping'].includes(currentRun.value.status)) return;
    error.value = null;
    await ensureListeners();
    try {
      const { runId } = await startShipping(project, item, action);
      const labels: Record<ShippingAction, string> = {
        createPullRequest: 'Creating pull request',
        mergePullRequest: 'Merging pull request',
        directToMain: 'Shipping directly to main',
      };
      currentRun.value = {
        runId,
        projectId: project.id,
        workItemId: item.id,
        kind: 'ship',
        scriptLabel: labels[action],
        output: pendingOutput.get(runId) ?? '',
        status: 'running',
        exitCode: null,
      };
      pendingOutput.delete(runId);
      const finished = pendingFinished.get(runId);
      if (finished) finishRun(finished);
      pendingFinished.delete(runId);
    } catch (shipError) {
      error.value = String(shipError);
    }
  }

  async function run(projectId: string, script: RunScript, workingDirectory: string) {
    if (currentRun.value && ['running', 'stopping'].includes(currentRun.value.status)) return;
    error.value = null;
    await ensureListeners();
    try {
      const { runId } = await startRun(projectId, script.id, workingDirectory);
      currentRun.value = {
        runId,
        projectId,
        workItemId: null,
        kind: 'run',
        scriptLabel: script.label,
        output: pendingOutput.get(runId) ?? '',
        status: 'running',
        exitCode: null,
      };
      pendingOutput.delete(runId);
      const finished = pendingFinished.get(runId);
      if (finished) finishRun(finished);
      pendingFinished.delete(runId);
    } catch (runError) {
      error.value = String(runError);
    }
  }

  async function cancel() {
    if (currentRun.value?.status !== 'running') return;
    currentRun.value = { ...currentRun.value, status: 'stopping' };
    try {
      await cancelRun(currentRun.value.runId);
    } catch (cancelError) {
      currentRun.value = { ...currentRun.value, status: 'running' };
      error.value = String(cancelError);
    }
  }

  function sendInput(input: string) {
    const runId = activeRunId();
    if (!runId || !input) return;
    inputQueue = inputQueue
      .then(() => writeRunInput(runId, input))
      .catch((inputError) => setActiveRunError(runId, inputError));
  }

  function resize(columns: number, rows: number) {
    const runId = activeRunId();
    if (!runId) return;
    void resizeRunTerminal(runId, columns, rows).catch((resizeError) =>
      setActiveRunError(runId, resizeError),
    );
  }

  function clear() {
    if (currentRun.value && !['running', 'stopping'].includes(currentRun.value.status)) {
      currentRun.value = null;
    }
  }

  return { currentRun, error, run, shipWork, cancel, sendInput, resize, clear };
}

function activeRunId() {
  if (currentRun.value?.status !== 'running') return null;
  return currentRun.value.runId;
}

function setActiveRunError(runId: string, runError: unknown) {
  if (currentRun.value?.runId === runId && currentRun.value.status === 'running') {
    error.value = String(runError);
  }
}
