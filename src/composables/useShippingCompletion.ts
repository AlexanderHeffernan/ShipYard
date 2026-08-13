import { ref } from 'vue';
import type { RunState } from '../types/run';
import type { ShippingAction } from '../types/shipping';

export type ShippingCompletionDetails = {
  workItemLabel: string;
  destination: string;
};

export type ShippingCompletion = {
  runId: string;
  action: ShippingAction;
  sunsetEffect: boolean;
  details: ShippingCompletionDetails;
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
  sunsetEffect: boolean,
  details?: ShippingCompletionDetails,
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
      sunsetEffect,
      details: details ?? {
        workItemLabel: run.scriptLabel,
        destination: run.shippingAction === 'mergePullRequest' || run.shippingAction === 'directToMain'
          ? 'the main line'
          : 'a pull request',
      },
    },
    lastCompletedRunId: run.runId,
  };
}

export function dismissShippingCompletion(state: ShippingCompletionState): ShippingCompletionState {
  return state.visible ? { ...state, visible: false } : state;
}

export function useShippingCompletion() {
  const state = ref<ShippingCompletionState>({ ...initialShippingCompletionState });

  function observeRun(
    run: RunState | null,
    sunsetEffect: boolean,
    details?: ShippingCompletionDetails,
  ) {
    state.value = reduceShippingCompletion(state.value, run, sunsetEffect, details);
  }

  function dismiss() {
    state.value = dismissShippingCompletion(state.value);
  }

  return { state, observeRun, dismiss };
}
