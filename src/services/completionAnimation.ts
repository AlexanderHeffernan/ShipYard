import shipyardSunsetBase from '../assets/shipyard-sunset-base.png';
import shipyardSunsetForegroundFog from '../assets/shipyard-sunset-foreground-fog.png';
import shipyardSunsetRearFog from '../assets/shipyard-sunset-rear-fog.png';
import shipyardSunsetShip from '../assets/shipyard-sunset-ship.png';
import shipyardSunsetWake from '../assets/shipyard-sunset-wake.png';
import {
  readSunsetEffectEnabled,
  saveSunsetEffectEnabled,
  type CompletionEffectStorage,
} from '../types/celebration';

export const sunsetEffectAssets = {
  base: shipyardSunsetBase,
  foregroundFog: shipyardSunsetForegroundFog,
  rearFog: shipyardSunsetRearFog,
  ship: shipyardSunsetShip,
  wake: shipyardSunsetWake,
} as const;

// Keep the decoded frames alive so the celebration can reuse them immediately.
const preloadedImages: HTMLImageElement[] = [];
let sunsetEffectPreload: Promise<void> | undefined;

function preloadImage(source: string) {
  if (typeof Image === 'undefined') return Promise.resolve();

  const image = new Image();
  preloadedImages.push(image);
  image.decoding = 'async';
  image.loading = 'eager';

  return new Promise<void>((resolve) => {
    let settled = false;

    const finish = () => {
      if (settled) return;
      settled = true;
      let decoding: Promise<void> | undefined;
      try {
        decoding = image.decode?.();
      } catch {
        resolve();
        return;
      }
      if (!decoding) {
        resolve();
        return;
      }
      void decoding.catch(() => undefined).then(() => resolve());
    };

    const fail = () => {
      if (settled) return;
      settled = true;
      resolve();
    };

    image.addEventListener('load', finish, { once: true });
    image.addEventListener('error', fail, { once: true });
    image.src = source;
    if (image.complete) finish();
  });
}

/** Start loading and decoding the effect assets before the celebration is shown. */
export function preloadSunsetEffect() {
  if (sunsetEffectPreload) return sunsetEffectPreload;

  sunsetEffectPreload = Promise.allSettled(
    Object.values(sunsetEffectAssets).map((source) => preloadImage(source)),
  ).then(() => undefined);
  return sunsetEffectPreload;
}

export function getSunsetEffectEnabled(storage?: CompletionEffectStorage) {
  return readSunsetEffectEnabled(storage);
}

export function setSunsetEffectEnabled(
  enabled: boolean,
  storage?: CompletionEffectStorage,
) {
  return saveSunsetEffectEnabled(enabled, storage);
}
