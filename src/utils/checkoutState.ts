export type CheckoutPhase = 'idle' | 'checking' | 'cancelling' | 'recovering' | 'finishing';

export type CheckoutButtonState = {
  label: string;
  hoverLabel: string | null;
  title: string;
  disabled: boolean;
  cancellable: boolean;
};

export function checkoutButtonState(
  phase: CheckoutPhase,
  unavailable: boolean,
): CheckoutButtonState {
  switch (phase) {
    case 'checking':
      return {
        label: 'Checking out…',
        hoverLabel: 'Cancel',
        title: 'Cancel pull request checkout',
        disabled: false,
        cancellable: true,
      };
    case 'cancelling':
      return {
        label: 'Cancelling…',
        hoverLabel: null,
        title: 'Waiting for checkout cancellation',
        disabled: true,
        cancellable: false,
      };
    case 'recovering':
      return {
        label: 'Stopping checkout…',
        hoverLabel: null,
        title: 'Waiting for checkout cancellation to finish',
        disabled: true,
        cancellable: false,
      };
    case 'finishing':
      return {
        label: 'Finishing…',
        hoverLabel: null,
        title: 'Refreshing the project after checkout',
        disabled: true,
        cancellable: false,
      };
    default:
      return {
        label: 'Check out',
        hoverLabel: null,
        title: unavailable
          ? 'This pull request is not available for checkout'
          : 'Create a local checkout for this pull request',
        disabled: unavailable,
        cancellable: false,
      };
  }
}
