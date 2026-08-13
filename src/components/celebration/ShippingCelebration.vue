<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import shipyardSunsetBase from '../../assets/shipyard-sunset-base.png';
import shipyardSunsetForegroundFog from '../../assets/shipyard-sunset-foreground-fog.png';
import shipyardSunsetRearFog from '../../assets/shipyard-sunset-rear-fog.png';
import shipyardSunsetShip from '../../assets/shipyard-sunset-ship.png';
import shipyardSunsetWake from '../../assets/shipyard-sunset-wake.png';
import type { ShippingCompletion } from '../../composables/useShippingCompletion';

const props = defineProps<{ completion: ShippingCompletion }>();
const emit = defineEmits<{ close: [] }>();

const root = ref<HTMLElement>();
const ready = ref(false);
const closing = ref(false);
const reducedMotion = ref(false);
let closeTimer: number | undefined;
let exitTimer: number | undefined;
let mediaQuery: MediaQueryList | undefined;

function updateReducedMotion(event: MediaQueryListEvent) {
  reducedMotion.value = event.matches;
}

function requestClose() {
  if (closing.value) return;
  window.clearTimeout(closeTimer);
  closing.value = true;
  exitTimer = window.setTimeout(() => emit('close'), reducedMotion.value ? 120 : 640);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return;
  event.preventDefault();
  requestClose();
}

onMounted(async () => {
  mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
  reducedMotion.value = mediaQuery.matches;
  mediaQuery.addEventListener?.('change', updateReducedMotion);
  window.addEventListener('keydown', onKeydown);

  await nextTick();
  const images = Array.from(root.value?.querySelectorAll('img') ?? []);
  await Promise.allSettled(images.map((image) => image.decode?.() ?? Promise.resolve()));
  if (!root.value) return;
  requestAnimationFrame(() => {
    ready.value = true;
    closeTimer = window.setTimeout(requestClose, reducedMotion.value ? 1800 : 3000);
  });
});

onBeforeUnmount(() => {
  window.clearTimeout(closeTimer);
  window.clearTimeout(exitTimer);
  mediaQuery?.removeEventListener?.('change', updateReducedMotion);
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <div
    ref="root"
    data-modal-layer="top"
    class="shipyard-sunset"
    :class="{ 'shipyard-sunset--ready': ready, 'shipyard-sunset--closing': closing, 'shipyard-sunset--reduced': reducedMotion }"
    role="status"
    aria-label="Shipping complete"
  >
    <div class="shipyard-sunset__canvas" aria-hidden="true">
      <img class="shipyard-sunset__base" :src="shipyardSunsetBase" alt="" draggable="false" />
      <img class="shipyard-sunset__rear-fog" :src="shipyardSunsetRearFog" alt="" draggable="false" />
      <img class="shipyard-sunset__static-fog" :src="shipyardSunsetForegroundFog" alt="" draggable="false" />
      <div class="shipyard-sunset__voyage">
        <img class="shipyard-sunset__wake" :src="shipyardSunsetWake" alt="" draggable="false" />
        <img class="shipyard-sunset__ship" :src="shipyardSunsetShip" alt="" draggable="false" />
      </div>
      <img class="shipyard-sunset__foreground-fog" :src="shipyardSunsetForegroundFog" alt="" draggable="false" />
    </div>

    <p class="sr-only" role="status" aria-live="assertive">
      {{ completion.details.workItemLabel }} shipped successfully.
    </p>
  </div>
</template>

<style scoped>
.shipyard-sunset {
  position: fixed;
  z-index: 60;
  inset: 0;
  overflow: hidden;
  opacity: 0;
  background: rgba(16, 13, 24, 0.12);
  backdrop-filter: blur(3px) saturate(0.88);
  isolation: isolate;
}

.shipyard-sunset--ready {
  animation: sunset-overlay-in 640ms cubic-bezier(0.4, 0, 0.2, 1) both;
}

.shipyard-sunset--closing {
  animation: sunset-overlay-out 640ms cubic-bezier(0.4, 0, 0.2, 1) both;
  pointer-events: none;
}

.shipyard-sunset__canvas {
  position: absolute;
  inset: 0;
  overflow: hidden;
  mask-image:
    radial-gradient(ellipse 54% 53% at 50% 49%, black 42%, rgba(0, 0, 0, 0.9) 58%, rgba(0, 0, 0, 0.48) 76%, transparent 100%),
    radial-gradient(ellipse 28% 42% at 20% 44%, rgba(0, 0, 0, 0.54), rgba(0, 0, 0, 0.18) 54%, transparent 82%),
    radial-gradient(ellipse 31% 38% at 82% 58%, rgba(0, 0, 0, 0.5), rgba(0, 0, 0, 0.16) 57%, transparent 84%),
    radial-gradient(ellipse 38% 23% at 42% 12%, rgba(0, 0, 0, 0.38), rgba(0, 0, 0, 0.1) 56%, transparent 86%),
    radial-gradient(ellipse 35% 25% at 58% 91%, rgba(0, 0, 0, 0.42), rgba(0, 0, 0, 0.12) 55%, transparent 84%);
}

.shipyard-sunset__base,
.shipyard-sunset__rear-fog,
.shipyard-sunset__static-fog,
.shipyard-sunset__voyage,
.shipyard-sunset__ship,
.shipyard-sunset__wake,
.shipyard-sunset__foreground-fog {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  pointer-events: none;
  user-select: none;
}

.shipyard-sunset__base { z-index: 0; }
.shipyard-sunset__rear-fog { z-index: 1; opacity: 0; animation: rear-fog-in 2880ms 120ms linear forwards; }
.shipyard-sunset__static-fog { z-index: 1; opacity: 0.24; }
.shipyard-sunset__voyage {
  z-index: 2;
  transform: scale(1);
  transform-origin: 50% 74.2%;
  animation: sunset-voyage 2880ms 120ms linear both;
  will-change: filter, transform;
}
.shipyard-sunset__wake { opacity: 0.86; animation: wake-dissipate 2880ms 120ms linear both; }
.shipyard-sunset__foreground-fog {
  z-index: 3;
  opacity: 0;
  animation: foreground-fog-in 2880ms 120ms linear both;
  -webkit-mask-image:
    radial-gradient(ellipse 25% 25% at 50% 66%, black 18%, rgba(0, 0, 0, 0.82) 48%, rgba(0, 0, 0, 0.24) 76%, transparent 100%),
    radial-gradient(ellipse 16% 14% at 40% 69%, rgba(0, 0, 0, 0.42), rgba(0, 0, 0, 0.12) 56%, transparent 88%),
    radial-gradient(ellipse 18% 16% at 60% 63%, rgba(0, 0, 0, 0.38), rgba(0, 0, 0, 0.1) 58%, transparent 90%);
  mask-image:
    radial-gradient(ellipse 25% 25% at 50% 66%, black 18%, rgba(0, 0, 0, 0.82) 48%, rgba(0, 0, 0, 0.24) 76%, transparent 100%),
    radial-gradient(ellipse 16% 14% at 40% 69%, rgba(0, 0, 0, 0.42), rgba(0, 0, 0, 0.12) 56%, transparent 88%),
    radial-gradient(ellipse 18% 16% at 60% 63%, rgba(0, 0, 0, 0.38), rgba(0, 0, 0, 0.1) 58%, transparent 90%);
  will-change: filter, opacity;
}

.shipyard-sunset__rear-fog,
.shipyard-sunset__voyage,
.shipyard-sunset__wake,
.shipyard-sunset__foreground-fog {
  animation-play-state: paused;
}

.shipyard-sunset--ready .shipyard-sunset__rear-fog,
.shipyard-sunset--ready .shipyard-sunset__voyage,
.shipyard-sunset--ready .shipyard-sunset__wake,
.shipyard-sunset--ready .shipyard-sunset__foreground-fog {
  animation-play-state: running;
}

@keyframes sunset-overlay-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes sunset-overlay-out { from { opacity: 1; } to { opacity: 0; } }
@keyframes sunset-voyage {
  from { filter: blur(0) saturate(1); transform: translateY(0) scale(1); }
  to { filter: blur(0.12px) saturate(0.97); transform: translateY(-1.55%) scale(0.77); }
}
@keyframes wake-dissipate {
  0% { opacity: 0.86; }
  35% { opacity: 0.72; }
  65% { opacity: 0.46; }
  86% { opacity: 0.18; }
  100% { opacity: 0; }
}
@keyframes rear-fog-in {
  0% { opacity: 0.1; filter: blur(1.5px); }
  42% { opacity: 0.2; filter: blur(0.8px); }
  100% { opacity: 0.32; filter: blur(0); }
}
@keyframes foreground-fog-in {
  0% { opacity: 0.16; filter: blur(1.2px); }
  24% { opacity: 0.28; filter: blur(1px); }
  50% { opacity: 0.48; filter: blur(0.7px); }
  72% { opacity: 0.68; filter: blur(0.35px); }
  100% { opacity: 0.86; filter: blur(0); }
}

.shipyard-sunset--reduced,
.shipyard-sunset--reduced * { animation-duration: 120ms !important; animation-delay: 0ms !important; }

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>
