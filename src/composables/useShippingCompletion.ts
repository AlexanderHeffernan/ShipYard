import { ref } from 'vue';
import type { CompletionAnimation, CompletionAnimationSpeed } from '../types/celebration';
import type { RunState } from '../types/run';
import type { ShippingAction } from '../types/shipping';

export type ShippingCompletionDetails = {
  workItemLabel: string;
  destination: string;
};

export type ShippingCompletion = {
  runId: string;
  action: ShippingAction;
  animation: CompletionAnimation;
  speed: CompletionAnimationSpeed;
  details: ShippingCompletionDetails;
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
  speed: CompletionAnimationSpeed = 'normal',
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
      animation,
      speed,
      details: details ?? {
        workItemLabel: run.scriptLabel,
        destination: run.shippingAction === 'mergePullRequest' || run.shippingAction === 'directToMain'
          ? 'the main line'
          : 'a pull request',
      },
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
  speed: CompletionAnimationSpeed = 'normal',
): ShippingCompletionState {
  return {
    ...state,
    visible: true,
    completion: {
      runId: `preview-${animation}-${Date.now()}`,
      action: 'directToMain',
      animation,
      speed,
      details: {
        workItemLabel: 'your work',
        destination: 'the main line',
      },
      preview: true,
    },
  };
}

export function useShippingCompletion() {
  const state = ref<ShippingCompletionState>({ ...initialShippingCompletionState });

  function observeRun(
    run: RunState | null,
    animation: CompletionAnimation,
    speed: CompletionAnimationSpeed = 'normal',
    details?: ShippingCompletionDetails,
  ) {
    state.value = reduceShippingCompletion(state.value, run, animation, speed, details);
  }

  function dismiss() {
    state.value = dismissShippingCompletion(state.value);
  }

  function preview(animation: CompletionAnimation, speed: CompletionAnimationSpeed = 'normal') {
    state.value = previewShippingCompletion(state.value, animation, speed);
  }

  return { state, observeRun, dismiss, preview };
}
