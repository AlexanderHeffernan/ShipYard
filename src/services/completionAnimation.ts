import {
  readCompletionAnimation,
  saveCompletionAnimation,
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
