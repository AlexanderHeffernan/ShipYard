import {
  readCompletionAnimation,
  readCompletionAnimationSpeed,
  saveCompletionAnimation,
  saveCompletionAnimationSpeed,
  type CompletionAnimationStorage,
} from '../types/celebration';

export function getCompletionAnimation(storage?: CompletionAnimationStorage) {
  return readCompletionAnimation(storage);
}

export function setCompletionAnimation(
  animation: unknown,
  storage?: CompletionAnimationStorage,
) {
  return saveCompletionAnimation(animation, storage);
}

export function getCompletionAnimationSpeed(storage?: CompletionAnimationStorage) {
  return readCompletionAnimationSpeed(storage);
}

export function setCompletionAnimationSpeed(
  speed: unknown,
  storage?: CompletionAnimationStorage,
) {
  return saveCompletionAnimationSpeed(speed, storage);
}
