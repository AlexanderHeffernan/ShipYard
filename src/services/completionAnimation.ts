import {
  readSunsetEffectEnabled,
  saveSunsetEffectEnabled,
  type CompletionEffectStorage,
} from '../types/celebration';

export function getSunsetEffectEnabled(storage?: CompletionEffectStorage) {
  return readSunsetEffectEnabled(storage);
}

export function setSunsetEffectEnabled(
  enabled: boolean,
  storage?: CompletionEffectStorage,
) {
  return saveSunsetEffectEnabled(enabled, storage);
}
