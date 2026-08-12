import { describe, expect, it } from 'vitest';
import {
  dismissShippingCompletion,
  initialShippingCompletionState,
  previewShippingCompletion,
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
    expect(reduceShippingCompletion(initialShippingCompletionState, run(), 'quiet-handoff')).toEqual(initialShippingCompletionState);
    expect(reduceShippingCompletion(initialShippingCompletionState, run({ kind: 'run', shippingAction: null, status: 'succeeded' }), 'quiet-handoff')).toEqual(initialShippingCompletionState);
    expect(reduceShippingCompletion(initialShippingCompletionState, run({ status: 'failed' }), 'quiet-handoff')).toEqual(initialShippingCompletionState);
    expect(reduceShippingCompletion(initialShippingCompletionState, run({ status: 'cancelled' }), 'quiet-handoff')).toEqual(initialShippingCompletionState);
  });

  it('shows exactly once for a successful shipping run', () => {
    const completed = reduceShippingCompletion(
      initialShippingCompletionState,
      run({ status: 'succeeded' }),
      'constellation-route',
      'slow',
      { workItemLabel: 'feature/harbor', destination: 'main' },
    );
    expect(completed.visible).toBe(true);
    expect(completed.completion).toMatchObject({
      runId: 'run-1',
      action: 'mergePullRequest',
      animation: 'constellation-route',
      speed: 'slow',
      details: { workItemLabel: 'feature/harbor', destination: 'main' },
      preview: false,
    });
    expect(reduceShippingCompletion(completed, run({ status: 'succeeded' }), 'constellation-route')).toEqual(completed);
    expect(reduceShippingCompletion(completed, run({ runId: 'run-2', status: 'succeeded' }), 'quiet-handoff').completion?.runId).toBe('run-2');
  });

  it('keeps the captured speed and receipt details stable during duplicate updates', () => {
    const completed = reduceShippingCompletion(
      initialShippingCompletionState,
      run({ status: 'succeeded' }),
      'firework-sky',
      'fast',
      { workItemLabel: 'feature/harbor', destination: 'main' },
    );

    expect(reduceShippingCompletion(
      completed,
      run({ status: 'succeeded' }),
      'shipyard-sunset',
      'slow',
      { workItemLabel: 'different item', destination: 'a pull request' },
    )).toEqual(completed);
  });

  it('can be dismissed without making the same completion eligible again', () => {
    const completed = reduceShippingCompletion(initialShippingCompletionState, run({ status: 'succeeded' }), 'quiet-handoff');
    const dismissed = dismissShippingCompletion(completed);

    expect(dismissed.visible).toBe(false);
    expect(reduceShippingCompletion(dismissed, run({ status: 'succeeded' }), 'quiet-handoff')).toEqual(dismissed);
  });

  it('keeps a settings preview separate from the real completion receipt', () => {
    const preview = previewShippingCompletion(initialShippingCompletionState, 'sail-away', 'fast');

    expect(preview.visible).toBe(true);
    expect(preview.completion).toMatchObject({
      preview: true,
      animation: 'sail-away',
      speed: 'fast',
      details: { workItemLabel: 'your work', destination: 'the main line' },
    });
    expect(preview.lastCompletedRunId).toBeNull();
    expect(reduceShippingCompletion(preview, run({ status: 'succeeded' }), 'quiet-handoff').completion?.preview).toBe(false);
  });
});
