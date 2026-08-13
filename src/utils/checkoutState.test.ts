import { describe, expect, it } from 'vitest';
import { checkoutButtonState } from './checkoutState';

describe('checkoutButtonState', () => {
  it('keeps checkout active while the project refresh is pending', () => {
    const state = checkoutButtonState('finishing', false);

    expect(state.label).toBe('Finishing…');
    expect(state.label).not.toBe('Check out');
    expect(state.disabled).toBe(true);
  });

  it('turns the active checkout button into a cancel action on hover', () => {
    const state = checkoutButtonState('checking', false);

    expect(state.label).toBe('Checking out…');
    expect(state.hoverLabel).toBe('Cancel');
    expect(state.title).toBe('Cancel pull request checkout');
    expect(state.cancellable).toBe(true);
    expect(state.disabled).toBe(false);
  });

  it('does not expose a second action while cancellation is settling', () => {
    const state = checkoutButtonState('cancelling', false);

    expect(state.label).toBe('Cancelling…');
    expect(state.hoverLabel).toBeNull();
    expect(state.disabled).toBe(true);
  });

  it('blocks a retry while a slow cancellation is still settling', () => {
    const state = checkoutButtonState('recovering', false);

    expect(state.label).toBe('Stopping checkout…');
    expect(state.title).toBe('Waiting for checkout cancellation to finish');
    expect(state.disabled).toBe(true);
    expect(state.hoverLabel).toBeNull();
  });

  it('only enables a fresh checkout when the pull request is available', () => {
    expect(checkoutButtonState('idle', false).disabled).toBe(false);
    expect(checkoutButtonState('idle', true).disabled).toBe(true);
  });
});
