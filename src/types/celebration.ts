export const COMPLETION_ANIMATION_STORAGE_KEY = 'shipyard.completionAnimation';
export const COMPLETION_ANIMATION_SPEED_STORAGE_KEY = 'shipyard.completionAnimationSpeed';

export const completionAnimationOptions = [
  {
    id: 'quiet-handoff',
    label: 'Quiet handoff',
    description: 'A gentle fade clears the review surface and reveals a centered ShipYard receipt.',
    default: true,
    fullScreen: false,
  },
  {
    id: 'sail-away',
    label: 'Sail away',
    description: 'A tiny ShipYard boat catches the wind and leaves a shimmering wake.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'lighthouse-beam',
    label: 'Lighthouse beam',
    description: 'A warm beam sweeps the harbor while the shipping mark comes into focus.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'confetti-burst',
    label: 'Confetti burst',
    description: 'A bright, celebratory shower of paper shapes for a little more energy.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'constellation-route',
    label: 'Constellation route',
    description: 'A route of stars connects the work to its destination in the night sky.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'tidal-rings',
    label: 'Tidal rings',
    description: 'Soft rings move across the water and settle into a polished success moment.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'dock-stamp',
    label: 'Dock stamp',
    description: 'A satisfying SHIPPED stamp lands with a restrained paper-and-ink snap.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'sunrise',
    label: 'Sunrise',
    description: 'A new day rises over the horizon with a warm, optimistic finish.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'paper-fleet',
    label: 'Paper fleet',
    description: 'Three folded boats drift through the frame like a tiny celebratory flotilla.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'firework-sky',
    label: 'Firework sky',
    description: 'Three different bursts light up the sky without losing the ShipYard palette.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'signal-path',
    label: 'Signal path',
    description: 'A glowing route travels node to node until it reaches the final beacon.',
    default: false,
    fullScreen: true,
  },
  {
    id: 'shipyard-sunset',
    label: 'Shipyard sunset',
    description: 'An authored cargo ship gently recedes from the waterline into the sunset.',
    default: false,
    fullScreen: true,
  },
] as const;

export type CompletionAnimation = (typeof completionAnimationOptions)[number]['id'];

export const DEFAULT_COMPLETION_ANIMATION: CompletionAnimation = 'quiet-handoff';

export const completionAnimationSpeedOptions = [
  { id: 'fast', label: 'Fast', multiplier: 0.72 },
  { id: 'normal', label: 'Normal', multiplier: 1 },
  { id: 'slow', label: 'Slow', multiplier: 1.38 },
] as const;

export type CompletionAnimationSpeed = (typeof completionAnimationSpeedOptions)[number]['id'];

export const DEFAULT_COMPLETION_ANIMATION_SPEED: CompletionAnimationSpeed = 'normal';

export function completionAnimationOption(animation: CompletionAnimation) {
  return completionAnimationOptions.find((option) => option.id === animation) ?? completionAnimationOptions[0];
}

export function isFullScreenCompletionAnimation(animation: CompletionAnimation) {
  return completionAnimationOption(animation).fullScreen;
}

export function isCompletionAnimationSpeed(value: unknown): value is CompletionAnimationSpeed {
  return completionAnimationSpeedOptions.some((option) => option.id === value);
}

export function normalizeCompletionAnimationSpeed(value: unknown): CompletionAnimationSpeed {
  return isCompletionAnimationSpeed(value) ? value : DEFAULT_COMPLETION_ANIMATION_SPEED;
}

export function completionAnimationSpeedMultiplier(speed: CompletionAnimationSpeed) {
  return completionAnimationSpeedOptions.find((option) => option.id === speed)?.multiplier ?? 1;
}

export function isCompletionAnimation(value: unknown): value is CompletionAnimation {
  return completionAnimationOptions.some((option) => option.id === value);
}

export function normalizeCompletionAnimation(value: unknown): CompletionAnimation {
  // `harbor-glow` was the original default. Treat it as the new quiet default
  // instead of reviving the old, more intrusive experience for existing users.
  if (value === 'harbor-glow') return DEFAULT_COMPLETION_ANIMATION;
  return isCompletionAnimation(value) ? value : DEFAULT_COMPLETION_ANIMATION;
}

export type CompletionAnimationStorage = Pick<Storage, 'getItem' | 'setItem'>;

function browserStorage(): CompletionAnimationStorage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

export function readCompletionAnimation(storage?: CompletionAnimationStorage): CompletionAnimation {
  const source = storage ?? browserStorage();
  if (!source) return DEFAULT_COMPLETION_ANIMATION;
  try {
    return normalizeCompletionAnimation(source.getItem(COMPLETION_ANIMATION_STORAGE_KEY));
  } catch {
    return DEFAULT_COMPLETION_ANIMATION;
  }
}

export function saveCompletionAnimation(
  animation: unknown,
  storage?: CompletionAnimationStorage,
): CompletionAnimation {
  const normalized = normalizeCompletionAnimation(animation);
  const target = storage ?? browserStorage();
  try {
    target?.setItem(COMPLETION_ANIMATION_STORAGE_KEY, normalized);
  } catch {
    // The in-memory selection still works when storage is unavailable.
  }
  return normalized;
}

export function readCompletionAnimationSpeed(storage?: CompletionAnimationStorage): CompletionAnimationSpeed {
  const source = storage ?? browserStorage();
  if (!source) return DEFAULT_COMPLETION_ANIMATION_SPEED;
  try {
    return normalizeCompletionAnimationSpeed(source.getItem(COMPLETION_ANIMATION_SPEED_STORAGE_KEY));
  } catch {
    return DEFAULT_COMPLETION_ANIMATION_SPEED;
  }
}

export function saveCompletionAnimationSpeed(
  speed: unknown,
  storage?: CompletionAnimationStorage,
): CompletionAnimationSpeed {
  const normalized = normalizeCompletionAnimationSpeed(speed);
  const target = storage ?? browserStorage();
  try {
    target?.setItem(COMPLETION_ANIMATION_SPEED_STORAGE_KEY, normalized);
  } catch {
    // The in-memory selection still works when storage is unavailable.
  }
  return normalized;
}
