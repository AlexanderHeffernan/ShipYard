import { ref } from 'vue';
import type { CompletionAnimation } from '../types/celebration';
import type { RunState } from '../types/run';
import type { ShippingAction } from '../types/shipping';

export type ShippingCompletion = {
  runId: string;
  action: ShippingAction;
  animation: CompletionAnimation;
  preview: boolean;
};

export type ShippingCompletionState = {
  visible: boolean;
  completion: ShippingCompletion | null;
  lastCompletedRunId: string | null;
};

export const initialShippingCompletionState: ShippingCompletionState = {
  visible: false,
  completion: null,
  lastCompletedRunId: null,
};

export function reduceShippingCompletion(
  state: ShippingCompletionState,
  run: RunState | null,
  animation: CompletionAnimation,
): ShippingCompletionState {
  if (
    !run
    || run.kind !== 'ship'
    || run.status !== 'succeeded'
    || !run.shippingAction
    || state.lastCompletedRunId === run.runId
  ) {
    return state;
  }

  return {
    visible: true,
    completion: {
      runId: run.runId,
      action: run.shippingAction,
      animation,
      preview: false,
    },
    lastCompletedRunId: run.runId,
  };
}

export function dismissShippingCompletion(state: ShippingCompletionState): ShippingCompletionState {
  return state.visible ? { ...state, visible: false } : state;
}

export function previewShippingCompletion(
  state: ShippingCompletionState,
  animation: CompletionAnimation,
): ShippingCompletionState {
  return {
    ...state,
    visible: true,
    completion: {
      runId: `preview-${animation}-${Date.now()}`,
      action: 'directToMain',
      animation,
      preview: true,
    },
  };
}

export function useShippingCompletion() {
  const state = ref<ShippingCompletionState>({ ...initialShippingCompletionState });

  function observeRun(run: RunState | null, animation: CompletionAnimation) {
    state.value = reduceShippingCompletion(state.value, run, animation);
  }

  function dismiss() {
    state.value = dismissShippingCompletion(state.value);
  }

  function preview(animation: CompletionAnimation) {
    state.value = previewShippingCompletion(state.value, animation);
  }

  return { state, observeRun, dismiss, preview };
}
