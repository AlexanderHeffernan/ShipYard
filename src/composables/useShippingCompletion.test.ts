import { describe, expect, it } from 'vitest';
import {
  dismissShippingCompletion,
  initialShippingCompletionState,
  reduceShippingCompletion,
} from './useShippingCompletion';
import type { RunState } from '../types/run';

function run(overrides: Partial<RunState> = {}): RunState {
  return {
    runId: 'run-1',
    projectId: 'project-1',
    workItemId: 'item-1',
    kind: 'ship',
    shippingAction: 'mergePullRequest',
    scriptLabel: 'Merging pull request',
    output: '',
    status: 'running',
    exitCode: null,
    ...overrides,
  };
}

describe('shipping completion lifecycle', () => {
  it('ignores non-shipping, failed, and in-progress runs', () => {
    expect(reduceShippingCompletion(initialShippingCompletionState, run(), false)).toEqual(initialShippingCompletionState);
    expect(reduceShippingCompletion(initialShippingCompletionState, run({ kind: 'run', shippingAction: null, status: 'succeeded' }), false)).toEqual(initialShippingCompletionState);
    expect(reduceShippingCompletion(initialShippingCompletionState, run({ status: 'failed' }), false)).toEqual(initialShippingCompletionState);
    expect(reduceShippingCompletion(initialShippingCompletionState, run({ status: 'cancelled' }), false)).toEqual(initialShippingCompletionState);
  });

  it('shows exactly once for a successful shipping run', () => {
    const completed = reduceShippingCompletion(
      initialShippingCompletionState,
      run({ status: 'succeeded' }),
      true,
      { workItemLabel: 'feature/harbor', destination: 'main' },
    );
    expect(completed.visible).toBe(true);
    expect(completed.completion).toMatchObject({
      runId: 'run-1',
      action: 'mergePullRequest',
      sunsetEffect: true,
      details: { workItemLabel: 'feature/harbor', destination: 'main' },
    });
    expect(reduceShippingCompletion(completed, run({ status: 'succeeded' }), true)).toEqual(completed);
    expect(reduceShippingCompletion(completed, run({ runId: 'run-2', status: 'succeeded' }), false).completion?.runId).toBe('run-2');
  });

  it('keeps the captured receipt details stable during duplicate updates', () => {
    const completed = reduceShippingCompletion(
      initialShippingCompletionState,
      run({ status: 'succeeded' }),
      false,
      { workItemLabel: 'feature/harbor', destination: 'main' },
    );

    expect(reduceShippingCompletion(
      completed,
      run({ status: 'succeeded' }),
      true,
      { workItemLabel: 'different item', destination: 'a pull request' },
    )).toEqual(completed);
  });

  it('can be dismissed without making the same completion eligible again', () => {
    const completed = reduceShippingCompletion(initialShippingCompletionState, run({ status: 'succeeded' }), false);
    const dismissed = dismissShippingCompletion(completed);

    expect(dismissed.visible).toBe(false);
    expect(reduceShippingCompletion(dismissed, run({ status: 'succeeded' }), false)).toEqual(dismissed);
  });
});
