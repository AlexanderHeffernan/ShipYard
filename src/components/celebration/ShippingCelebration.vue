<script setup lang="ts">
import { Check, X } from '@lucide/vue';
import { computed, onBeforeUnmount, onMounted, ref, useId } from 'vue';
import {
  isFullScreenCompletionAnimation,
  type CompletionAnimation,
} from '../../types/celebration';
import type { ShippingCompletion } from '../../composables/useShippingCompletion';

const props = defineProps<{
  completion: ShippingCompletion;
}>();

const emit = defineEmits<{ close: [] }>();

const titleId = useId();
const descriptionId = useId();
const root = ref<HTMLElement>();
const closeButton = ref<HTMLButtonElement>();
const reducedMotion = ref(false);
const closing = ref(false);
const animation = computed<CompletionAnimation>(() => props.completion.animation);
const quietHandoff = computed(() => !isFullScreenCompletionAnimation(animation.value));
const confettiPieces = Array.from({ length: 28 }, (_, index) => ({
  id: index,
  x: `${((index * 47) % 360) - 180}px`,
  y: `${-180 - ((index * 31) % 170)}px`,
  rotate: `${(index * 53) % 240 - 120}deg`,
  color: ['#fb771f', '#fc9320', '#64cf8c', '#a78bfa', '#ffd166', '#ff9db8'][index % 6],
  delay: `${(index % 8) * 45}ms`,
  size: `${7 + (index % 4) * 2}px`,
}));
const stars = Array.from({ length: 18 }, (_, index) => ({
  id: index,
  x: `${8 + ((index * 43) % 84)}%`,
  y: `${10 + ((index * 29) % 68)}%`,
  delay: `${(index % 7) * 180}ms`,
  size: `${2 + (index % 3)}px`,
}));
const fleetBoats = [
  { id: 'left', className: 'paper-fleet__boat--left', delay: '0ms' },
  { id: 'center', className: 'paper-fleet__boat--center', delay: '320ms' },
  { id: 'right', className: 'paper-fleet__boat--right', delay: '650ms' },
];
const fireworkBursts = [
  { id: 'left', className: 'firework-sky__burst--left', color: '#fb771f', delay: '0ms' },
  { id: 'center', className: 'firework-sky__burst--center', color: '#64cf8c', delay: '260ms' },
  { id: 'right', className: 'firework-sky__burst--right', color: '#a78bfa', delay: '540ms' },
];
const fireworkAngles = Array.from({ length: 12 }, (_, index) => `${index * 30}deg`);

const copy = computed(() => {
  if (props.completion.preview) {
    return {
      eyebrow: 'Completion animation preview',
      title: 'Ship it with confidence',
      detail: 'This is a preview only. No branch, worktree, or remote state changed.',
      announcement: `Previewing the ${animation.value} completion animation. No project state changed.`,
    };
  }

  if (quietHandoff.value) {
    return {
      eyebrow: 'Shipped',
      title: 'Work item shipped',
      detail: 'The changes are safely on their way. Pick your next item when you’re ready.',
      announcement: 'Shipping completed successfully. The work item was shipped.',
    };
  }

  switch (props.completion.action) {
    case 'createPullRequest':
      return {
        eyebrow: 'Pull request created',
        title: 'Your work is on its way',
        detail: 'The branch is pushed and the pull request is ready for review.',
        announcement: 'Shipping completed successfully. The pull request was created and pushed.',
      };
    case 'updatePullRequest':
      return {
        eyebrow: 'Pull request updated',
        title: 'Fresh work, safely aboard',
        detail: 'Your latest changes are pushed and the pull request is up to date.',
        announcement: 'Shipping completed successfully. The pull request was updated and pushed.',
      };
    case 'mergePullRequest':
      return {
        eyebrow: 'Pull request merged',
        title: 'Shipped to the main line',
        detail: 'The pull request merged successfully. Nice work.',
        announcement: 'Shipping completed successfully. The pull request was merged.',
      };
    case 'directToMain':
      return {
        eyebrow: 'Direct ship complete',
        title: 'Made it to the main line',
        detail: 'The resolved commit is pushed to the default branch.',
        announcement: 'Shipping completed successfully. The resolved commit was pushed to the default branch.',
      };
  }
});

let closeTimer: number | undefined;
let exitTimer: number | undefined;
let mediaQuery: MediaQueryList | null = null;
let previousFocus: HTMLElement | null = null;

function updateReducedMotion() {
  reducedMotion.value = mediaQuery?.matches ?? false;
}

function requestClose() {
  if (closing.value) return;
  window.clearTimeout(closeTimer);
  closing.value = true;
  exitTimer = window.setTimeout(() => emit('close'), reducedMotion.value ? 120 : 360);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault();
    event.stopPropagation();
    requestClose();
    return;
  }
  if (event.key !== 'Tab') return;
  const focusable = Array.from(root.value?.querySelectorAll<HTMLElement>('button:not([disabled])') ?? []);
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

onMounted(() => {
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
  updateReducedMotion();
  mediaQuery.addEventListener?.('change', updateReducedMotion);
  window.addEventListener('keydown', onKeydown);
  closeButton.value?.focus({ preventScroll: true });
  closeTimer = window.setTimeout(requestClose, reducedMotion.value ? 1800 : quietHandoff.value ? 2900 : 5200);
});

onBeforeUnmount(() => {
  window.clearTimeout(closeTimer);
  window.clearTimeout(exitTimer);
  mediaQuery?.removeEventListener?.('change', updateReducedMotion);
  window.removeEventListener('keydown', onKeydown);
  if (previousFocus?.isConnected) previousFocus.focus({ preventScroll: true });
});
</script>

<template>
  <div
    ref="root"
    data-modal-layer="top"
    class="celebration"
    :class="[`celebration--${animation}`, { 'celebration--preview': completion.preview, 'celebration--reduced': reducedMotion, 'celebration--closing': closing }]"
    role="dialog"
    aria-modal="true"
    :aria-labelledby="completion.preview ? undefined : titleId"
    :aria-describedby="completion.preview ? undefined : descriptionId"
    :aria-label="completion.preview ? 'Completion animation preview' : undefined"
    @keydown="onKeydown"
    @click.self="requestClose"
  >
    <div class="celebration__backdrop" aria-hidden="true"></div>

    <div class="celebration__scene" aria-hidden="true">
      <template v-if="animation === 'quiet-handoff'">
        <div class="quiet-handoff__glow"></div>
        <div class="quiet-handoff__line quiet-handoff__line--one"></div>
        <div class="quiet-handoff__line quiet-handoff__line--two"></div>
        <div v-if="completion.preview" class="quiet-handoff__preview-mark">
          <Check aria-hidden="true" />
        </div>
      </template>

      <template v-else-if="animation === 'sail-away'">
        <div class="sail-away__moon"></div>
        <div class="sail-away__horizon"></div>
        <div class="sail-away__wake sail-away__wake--one"></div>
        <div class="sail-away__wake sail-away__wake--two"></div>
        <svg class="sail-away__boat" viewBox="0 0 170 100">
          <path class="sail-away__mast" d="M85 11v64" />
          <path class="sail-away__sail" d="M84 14 37 68h47V14Z" />
          <path class="sail-away__flag" d="M85 12h25l-13 9H85Z" />
          <path class="sail-away__hull" d="M24 74h123l-16 15H43L24 74Z" />
          <path class="sail-away__deck" d="M45 70h77" />
        </svg>
      </template>

      <template v-else-if="animation === 'lighthouse-beam'">
        <div class="lighthouse-beam__sky-glow"></div>
        <div class="lighthouse-beam__beam lighthouse-beam__beam--one"></div>
        <div class="lighthouse-beam__beam lighthouse-beam__beam--two"></div>
        <div class="lighthouse-beam__horizon"></div>
        <div class="lighthouse-beam__tower">
          <span class="lighthouse-beam__lamp"></span>
          <span class="lighthouse-beam__roof"></span>
          <span class="lighthouse-beam__window"></span>
        </div>
      </template>

      <template v-else-if="animation === 'confetti-burst'">
        <span
          v-for="piece in confettiPieces"
          :key="piece.id"
          class="confetti-burst__piece"
          :style="{ '--x': piece.x, '--y': piece.y, '--rotate': piece.rotate, '--delay': piece.delay, '--piece-color': piece.color, '--piece-size': piece.size }"
        ></span>
        <div class="confetti-burst__burst confetti-burst__burst--one"></div>
        <div class="confetti-burst__burst confetti-burst__burst--two"></div>
        <div class="confetti-burst__burst confetti-burst__burst--three"></div>
      </template>

      <template v-else-if="animation === 'constellation-route'">
        <span v-for="star in stars" :key="star.id" class="constellation-route__star" :style="{ left: star.x, top: star.y, width: star.size, height: star.size, animationDelay: star.delay }"></span>
        <svg class="constellation-route__map" viewBox="0 0 1200 720" preserveAspectRatio="none">
          <path class="constellation-route__path constellation-route__path--ghost" d="M72 516C235 518 236 247 412 293S629 570 794 344 964 210 1128 188" />
          <path class="constellation-route__path" d="M72 516C235 518 236 247 412 293S629 570 794 344 964 210 1128 188" />
          <path class="constellation-route__check" d="m1081 185 25 25 48-53" />
        </svg>
      </template>

      <template v-else-if="animation === 'tidal-rings'">
        <div class="tidal-rings__waterline tidal-rings__waterline--one"></div>
        <div class="tidal-rings__waterline tidal-rings__waterline--two"></div>
        <div class="tidal-rings__ring tidal-rings__ring--one"></div>
        <div class="tidal-rings__ring tidal-rings__ring--two"></div>
        <div class="tidal-rings__ring tidal-rings__ring--three"></div>
        <div class="tidal-rings__ring tidal-rings__ring--four"></div>
        <div class="tidal-rings__foam"></div>
      </template>

      <template v-else-if="animation === 'dock-stamp'">
        <div class="dock-stamp__grid"></div>
        <span v-for="index in 12" :key="index" class="dock-stamp__ray" :style="{ '--ray-angle': `${index * 30}deg`, animationDelay: `${index * 35}ms` }"></span>
        <div class="dock-stamp__seal"><Check aria-hidden="true" /></div>
      </template>

      <template v-else-if="animation === 'sunrise'">
        <div class="sunrise__stars"></div>
        <div class="sunrise__sunrise-glow"></div>
        <div class="sunrise__sun"></div>
        <div class="sunrise__horizon"></div>
        <div class="sunrise__wave sunrise__wave--one"></div>
        <div class="sunrise__wave sunrise__wave--two"></div>
        <svg class="sunrise__boat" viewBox="0 0 160 80">
          <path d="M30 56h102l-18 13H49L30 56Z" />
          <path d="M80 15v40M80 18 47 54h33V18Z" />
        </svg>
      </template>

      <template v-else-if="animation === 'paper-fleet'">
        <div class="paper-fleet__horizon"></div>
        <div v-for="boat in fleetBoats" :key="boat.id" class="paper-fleet__boat" :class="boat.className" :style="{ animationDelay: boat.delay }">
          <svg viewBox="0 0 150 90">
            <path class="paper-fleet__fold paper-fleet__fold--top" d="M19 44 76 21l53 25-57 12-53-14Z" />
            <path class="paper-fleet__fold paper-fleet__fold--front" d="m19 44 53 14 57-12-55 27-55-29Z" />
            <path class="paper-fleet__fold paper-fleet__fold--shadow" d="m19 44 55 10-2 4-53-14Z" />
          </svg>
          <span></span>
        </div>
        <div class="paper-fleet__wake"></div>
      </template>

      <template v-else-if="animation === 'firework-sky'">
        <div v-for="burst in fireworkBursts" :key="burst.id" class="firework-sky__burst" :class="burst.className" :style="{ '--burst-color': burst.color, animationDelay: burst.delay }">
          <span v-for="angle in fireworkAngles" :key="angle" class="firework-sky__spark" :style="{ '--spark-angle': angle }"></span>
        </div>
        <div class="firework-sky__horizon"></div>
        <span v-for="index in 14" :key="index" class="firework-sky__star" :style="{ left: `${8 + ((index * 47) % 84)}%`, top: `${9 + ((index * 37) % 55)}%`, animationDelay: `${index * 90}ms` }"></span>
      </template>

      <template v-else-if="animation === 'signal-path'">
        <div class="signal-path__grid"></div>
        <svg class="signal-path__route" viewBox="0 0 1200 700" preserveAspectRatio="none">
          <path class="signal-path__route-line signal-path__route-line--ghost" d="M80 510h185l65-184h205l70 180h180l80-253h290" />
          <path class="signal-path__route-line" d="M80 510h185l65-184h205l70 180h180l80-253h290" />
        </svg>
        <span v-for="index in 5" :key="index" class="signal-path__node" :class="`signal-path__node--${index}`"><span></span></span>
        <div class="signal-path__beacon"><span></span></div>
      </template>
    </div>

    <section v-if="!completion.preview" class="celebration__content" :class="{ 'celebration__content--quiet': quietHandoff }">
      <div class="celebration__success-mark">
        <span class="celebration__success-ring celebration__success-ring--one"></span>
        <span class="celebration__success-ring celebration__success-ring--two"></span>
        <Check aria-hidden="true" />
      </div>
      <span class="celebration__eyebrow"><i></i>{{ copy.eyebrow }}</span>
      <h1 :id="titleId">{{ copy.title }}</h1>
      <p :id="descriptionId">{{ copy.detail }}</p>
      <button ref="closeButton" class="celebration__dismiss" type="button" @click="requestClose">
        <span>Done</span>
        <X aria-hidden="true" />
      </button>
    </section>

    <button
      v-if="completion.preview"
      ref="closeButton"
      class="celebration__preview-dismiss"
      type="button"
      aria-label="Close animation preview"
      title="Close animation preview"
      @click="requestClose"
    >
      <X aria-hidden="true" />
    </button>

    <p class="sr-only" role="status" aria-live="assertive">{{ copy.announcement }}</p>
  </div>
</template>

<style scoped>
.celebration {
  --celebration-orange: #fb771f;
  --celebration-yellow: #ffd166;
  --celebration-green: #64cf8c;
  --celebration-purple: #a78bfa;
  --celebration-pink: #ff9db8;
  position: fixed;
  z-index: 60;
  inset: 0;
  display: grid;
  overflow: hidden;
  place-items: center;
  color: var(--text-primary);
  background: #100d18;
  isolation: isolate;
  animation: celebration-in 360ms cubic-bezier(0.2, 0.75, 0.25, 1) both;
}

.celebration--quiet-handoff {
  position: absolute;
  z-index: 60;
  inset: 0;
  background: transparent;
  pointer-events: none;
}

.celebration--quiet-handoff .celebration__backdrop {
  display: none;
}

.celebration--quiet-handoff.celebration--preview .quiet-handoff__glow,
.celebration--quiet-handoff.celebration--preview .quiet-handoff__line {
  display: none;
}

.celebration--closing {
  animation: celebration-out 360ms ease both;
  pointer-events: none;
}

.celebration__backdrop,
.celebration__scene {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.celebration__backdrop {
  z-index: -2;
  background:
    radial-gradient(circle at 50% 47%, rgba(251, 119, 31, 0.16), transparent 25%),
    radial-gradient(circle at 20% 90%, rgba(104, 68, 148, 0.18), transparent 35%),
    radial-gradient(circle at 85% 10%, rgba(251, 119, 31, 0.09), transparent 34%),
    #100d18;
}

.celebration__backdrop::before,
.celebration__backdrop::after {
  position: absolute;
  content: '';
}

.celebration__backdrop::before {
  inset: 0;
  opacity: 0.22;
  background-image: linear-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px), linear-gradient(90deg, rgba(255, 255, 255, 0.035) 1px, transparent 1px);
  background-size: 48px 48px;
  mask-image: linear-gradient(to bottom, transparent, black 24%, black 76%, transparent);
}

.celebration__backdrop::after {
  right: -12%;
  bottom: -30%;
  left: -12%;
  height: 60%;
  opacity: 0.55;
  background: radial-gradient(ellipse at center, rgba(251, 119, 31, 0.08), transparent 65%);
  filter: blur(24px);
}

.celebration__scene {
  z-index: -1;
  overflow: hidden;
}

.celebration--quiet-handoff .celebration__scene {
  z-index: 0;
}

.celebration__content {
  position: relative;
  z-index: 3;
  display: flex;
  width: min(430px, calc(100vw - 42px));
  align-items: center;
  flex-direction: column;
  padding: 32px 34px 28px;
  text-align: center;
  background: linear-gradient(150deg, rgba(29, 20, 36, 0.82), rgba(16, 13, 24, 0.74));
  border: 1px solid rgba(255, 240, 248, 0.16);
  border-radius: 20px;
  box-shadow: 0 26px 90px rgba(5, 3, 8, 0.58), inset 0 1px rgba(255, 255, 255, 0.06);
  backdrop-filter: blur(20px) saturate(1.08);
  animation: celebration-card-in 650ms 70ms cubic-bezier(0.2, 0.75, 0.25, 1) both;
}

.celebration--quiet-handoff .celebration__content {
  pointer-events: auto;
}

.celebration__content--quiet {
  width: min(320px, calc(100vw - 42px));
  padding: 24px 26px 22px;
  border-radius: 16px;
  box-shadow: 0 18px 58px rgba(5, 3, 8, 0.42), inset 0 1px rgba(255, 255, 255, 0.06);
  animation: quiet-receipt-in 460ms 80ms cubic-bezier(0.2, 0.75, 0.25, 1) both;
}

.celebration__success-mark {
  position: relative;
  display: grid;
  width: 78px;
  height: 78px;
  margin-bottom: 20px;
  place-items: center;
  color: var(--primary-foreground);
  background: linear-gradient(145deg, #ffb14a, var(--celebration-orange));
  border: 1px solid rgba(255, 236, 209, 0.72);
  border-radius: 24px;
  box-shadow: 0 0 0 8px rgba(251, 119, 31, 0.08), 0 12px 34px rgba(251, 119, 31, 0.25), inset 0 1px rgba(255, 255, 255, 0.42);
  transform: rotate(-4deg);
}

.celebration__content--quiet .celebration__success-mark {
  width: 58px;
  height: 58px;
  margin-bottom: 14px;
  border-radius: 18px;
  transform: none;
}

.celebration__content--quiet .celebration__success-mark svg {
  width: 28px;
  height: 28px;
}

.celebration__content--quiet .celebration__success-ring {
  display: none;
}

.celebration__content--quiet .celebration__dismiss {
  margin-top: 18px;
}

.celebration__success-mark svg {
  width: 36px;
  height: 36px;
  stroke-width: 2.5;
  animation: success-draw 650ms 240ms cubic-bezier(0.2, 0.75, 0.25, 1) both;
}

.celebration__success-ring {
  position: absolute;
  border: 1px solid rgba(251, 119, 31, 0.44);
  border-radius: 50%;
  pointer-events: none;
}

.celebration__success-ring--one {
  inset: -17px;
  animation: success-ring 1900ms 280ms ease-out both;
}

.celebration__success-ring--two {
  inset: -31px;
  opacity: 0.6;
  animation: success-ring 2100ms 460ms ease-out both;
}

.celebration__eyebrow {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  margin-bottom: 10px;
  font-size: 10px;
  font-weight: 650;
  color: var(--celebration-green);
  text-transform: uppercase;
  letter-spacing: 0.12em;
}

.celebration__eyebrow i {
  width: 6px;
  height: 6px;
  background: currentColor;
  border-radius: 50%;
  box-shadow: 0 0 12px currentColor;
}

.celebration__content h1 {
  max-width: 340px;
  margin: 0;
  font-size: clamp(21px, 3vw, 28px);
  font-weight: 650;
  line-height: 1.12;
  letter-spacing: -0.026em;
}

.celebration__content p {
  max-width: 320px;
  margin: 10px 0 0;
  font-size: 12px;
  line-height: 1.55;
  color: var(--text-secondary);
}

.celebration__preview-note {
  margin-top: 12px;
  font-size: 10px;
  color: var(--celebration-yellow);
}

.celebration__dismiss {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 31px;
  margin-top: 23px;
  padding: 0 12px 0 14px;
  font: inherit;
  font-size: 11px;
  font-weight: 600;
  color: var(--primary-foreground);
  background: var(--primary);
  border: 1px solid var(--primary-hover);
  border-radius: 8px;
  box-shadow: 0 4px 15px rgba(251, 119, 31, 0.17);
}

.celebration__dismiss:hover {
  background: var(--primary-hover);
}

.celebration__dismiss:focus-visible {
  outline: 2px solid var(--primary-hover);
  outline-offset: 3px;
}

.celebration__dismiss svg {
  width: 12px;
  height: 12px;
  stroke-width: 2;
}

.celebration__preview-dismiss {
  position: absolute;
  z-index: 5;
  top: 18px;
  right: 18px;
  display: grid;
  width: 32px;
  height: 32px;
  padding: 0;
  place-items: center;
  color: rgba(255, 250, 252, 0.74);
  background: rgba(16, 13, 24, 0.56);
  border: 1px solid rgba(255, 240, 248, 0.16);
  border-radius: 50%;
  box-shadow: 0 8px 24px rgba(5, 3, 8, 0.24);
  backdrop-filter: blur(12px);
  pointer-events: auto;
  transition: color 140ms ease, background 140ms ease, border-color 140ms ease, opacity 180ms ease;
}

.celebration__preview-dismiss:hover {
  color: var(--text-primary);
  background: rgba(251, 119, 31, 0.18);
  border-color: rgba(251, 119, 31, 0.4);
}

.celebration__preview-dismiss:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 3px;
}

.celebration__preview-dismiss svg {
  width: 15px;
  height: 15px;
  stroke-width: 1.8;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

/* Quiet handoff — the unobtrusive default. */
.quiet-handoff__glow {
  position: absolute;
  top: 50%;
  left: 50%;
  width: min(52vw, 560px);
  aspect-ratio: 1;
  background: radial-gradient(circle, rgba(251, 119, 31, 0.17), rgba(251, 119, 31, 0.04) 38%, transparent 70%);
  border-radius: 50%;
  transform: translate(-50%, -50%);
  animation: quiet-glow-in 700ms 80ms ease-out both;
}

.quiet-handoff__line {
  position: absolute;
  right: 16%;
  left: 16%;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(100, 207, 140, 0.45), rgba(251, 119, 31, 0.48), transparent);
  box-shadow: 0 0 24px rgba(100, 207, 140, 0.2);
  opacity: 0;
  animation: quiet-line-in 650ms 160ms ease-out both;
}

.quiet-handoff__line--one { top: 42%; }
.quiet-handoff__line--two { top: 58%; animation-delay: 260ms; opacity: 0.5; }

.quiet-handoff__preview-mark {
  position: absolute;
  top: 50%;
  left: 50%;
  display: grid;
  width: 58px;
  height: 58px;
  place-items: center;
  color: var(--primary-foreground);
  background: linear-gradient(145deg, #ffb14a, var(--celebration-orange));
  border: 1px solid rgba(255, 236, 209, 0.72);
  border-radius: 18px;
  box-shadow: 0 0 0 8px rgba(251, 119, 31, 0.08), 0 12px 34px rgba(251, 119, 31, 0.25), inset 0 1px rgba(255, 255, 255, 0.42);
  transform: translate(-50%, -50%);
  opacity: 0;
  animation: quiet-mark-in 480ms 300ms cubic-bezier(0.2, 0.75, 0.25, 1) both;
}

.celebration--quiet-handoff.celebration--preview .quiet-handoff__preview-mark {
  position: fixed;
  top: auto;
  right: 28px;
  bottom: 28px;
  left: auto;
  transform: none;
  animation: quiet-preview-mark-in 480ms 300ms cubic-bezier(0.2, 0.75, 0.25, 1) both;
}

.quiet-handoff__preview-mark svg {
  width: 28px;
  height: 28px;
  stroke-width: 2.5;
}

/* Sail away — a direct ShipYard/boat nod. */
.sail-away__moon {
  position: absolute;
  top: 16%;
  right: 16%;
  width: 52px;
  height: 52px;
  background: #ffdca8;
  border-radius: 50%;
  box-shadow: 0 0 55px rgba(255, 220, 168, 0.28);
}

.sail-away__horizon,
.paper-fleet__horizon,
.sunrise__horizon,
.lighthouse-beam__horizon,
.firework-sky__horizon {
  position: absolute;
  right: -5%;
  bottom: 14%;
  left: -5%;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(100, 207, 140, 0.5), rgba(251, 119, 31, 0.5), transparent);
  box-shadow: 0 0 22px rgba(100, 207, 140, 0.22);
}

.sail-away__boat {
  position: absolute;
  top: 61%;
  left: -220px;
  width: 170px;
  height: 100px;
  overflow: visible;
  animation: sail-across 4800ms 200ms cubic-bezier(0.16, 0.7, 0.25, 1) both;
}

.sail-away__hull { fill: #24162b; stroke: rgba(255, 240, 248, 0.7); stroke-width: 1.3; }
.sail-away__deck, .sail-away__mast { fill: none; stroke: rgba(255, 240, 248, 0.72); stroke-linecap: round; stroke-width: 1.4; }
.sail-away__sail { fill: var(--celebration-orange); stroke: #ffc16e; stroke-width: 1; }
.sail-away__flag { fill: var(--celebration-green); }
.sail-away__wake { position: absolute; top: 71%; width: 145px; height: 12px; border-top: 1px solid rgba(100, 207, 140, 0.65); border-radius: 50%; transform: translateX(100vw); animation: wake-across 4700ms 260ms ease-out both; }
.sail-away__wake--one { left: -160px; }
.sail-away__wake--two { top: 74%; left: -104px; width: 80px; opacity: 0.55; animation-delay: 520ms; }

/* Lighthouse beam. */
.lighthouse-beam__sky-glow { position: absolute; top: 8%; left: 50%; width: 56vw; height: 56vw; background: radial-gradient(circle, rgba(255, 209, 102, 0.2), transparent 63%); transform: translateX(-50%); }
.lighthouse-beam__beam { position: absolute; top: 31%; left: 50%; width: 75vw; height: 110px; background: linear-gradient(90deg, transparent, rgba(255, 209, 102, 0.15), rgba(255, 231, 174, 0.42), rgba(255, 209, 102, 0.08), transparent); transform-origin: left center; filter: blur(1px); }
.lighthouse-beam__beam--one { animation: beam-sweep 4300ms ease-in-out infinite; }
.lighthouse-beam__beam--two { animation: beam-sweep 4300ms 1100ms ease-in-out infinite; opacity: 0.6; }
.lighthouse-beam__tower { position: absolute; bottom: 13%; left: 50%; width: 62px; height: 154px; background: linear-gradient(90deg, #332238, #1e1728); clip-path: polygon(24% 20%, 76% 20%, 100% 100%, 0 100%); transform: translateX(-50%); box-shadow: 0 0 24px rgba(251, 119, 31, 0.2); }
.lighthouse-beam__roof { position: absolute; top: 13%; left: 50%; width: 55px; height: 17px; background: var(--celebration-orange); clip-path: polygon(50% 0, 100% 100%, 0 100%); transform: translateX(-50%); }
.lighthouse-beam__lamp { position: absolute; top: 24%; left: 50%; width: 22px; height: 18px; background: #ffeab0; border: 2px solid #493048; border-radius: 3px; transform: translateX(-50%); box-shadow: 0 0 25px #ffd166; }
.lighthouse-beam__window { position: absolute; bottom: 28%; left: 50%; width: 12px; height: 26px; background: rgba(100, 207, 140, 0.5); border: 1px solid rgba(255, 255, 255, 0.3); transform: translateX(-50%); }

/* Confetti burst. */
.confetti-burst__piece { position: absolute; top: 50%; left: 50%; width: var(--piece-size); height: calc(var(--piece-size) * 1.8); background: var(--piece-color); border-radius: 2px; opacity: 0; transform: translate(-50%, -50%); animation: confetti-fly 3400ms var(--delay) cubic-bezier(0.15, 0.8, 0.28, 1) both; }
.confetti-burst__piece:nth-of-type(3n) { border-radius: 50%; }
.confetti-burst__burst { position: absolute; top: 50%; left: 50%; width: 16px; height: 16px; border: 1px solid var(--celebration-orange); border-radius: 50%; transform: translate(-50%, -50%); animation: burst-ring 1500ms ease-out both; }
.confetti-burst__burst--two { border-color: var(--celebration-green); animation-delay: 160ms; }
.confetti-burst__burst--three { border-color: var(--celebration-purple); animation-delay: 320ms; }

/* Constellation route. */
.constellation-route__star { position: absolute; display: block; background: #f8e7c9; border-radius: 50%; box-shadow: 0 0 9px rgba(255, 238, 196, 0.75); animation: star-pulse 1900ms ease-in-out infinite; }
.constellation-route__map { position: absolute; inset: 0; width: 100%; height: 100%; }
.constellation-route__path { fill: none; stroke: var(--celebration-orange); stroke-width: 2; stroke-linecap: round; stroke-dasharray: 1600; stroke-dashoffset: 1600; animation: route-draw 3000ms 260ms cubic-bezier(0.2, 0.75, 0.25, 1) forwards; }
.constellation-route__path--ghost { stroke: rgba(251, 119, 31, 0.12); stroke-width: 8; stroke-dasharray: none; stroke-dashoffset: 0; animation: none; }
.constellation-route__check { fill: none; stroke: var(--celebration-green); stroke-width: 5; stroke-linecap: round; stroke-linejoin: round; stroke-dasharray: 100; stroke-dashoffset: 100; animation: check-draw 700ms 2500ms ease-out forwards; }

/* Tidal rings. */
.tidal-rings__ring { position: absolute; top: 50%; left: 50%; width: 80px; aspect-ratio: 1; border: 1px solid rgba(100, 207, 140, 0.5); border-radius: 50%; transform: translate(-50%, -50%) scale(0.12); opacity: 0; animation: tidal-expand 3400ms ease-out infinite; }
.tidal-rings__ring--two { animation-delay: 650ms; border-color: rgba(251, 119, 31, 0.55); }
.tidal-rings__ring--three { animation-delay: 1300ms; border-color: rgba(167, 139, 250, 0.48); }
.tidal-rings__ring--four { animation-delay: 1950ms; border-color: rgba(255, 209, 102, 0.4); }
.tidal-rings__waterline { position: absolute; left: -10%; width: 120%; height: 30%; border-top: 1px solid rgba(100, 207, 140, 0.28); border-radius: 50%; transform: rotate(-4deg); animation: waterline-drift 3600ms ease-in-out infinite; }
.tidal-rings__waterline--one { top: 25%; }
.tidal-rings__waterline--two { top: 73%; animation-delay: 900ms; opacity: 0.6; }
.tidal-rings__foam { position: absolute; right: 20%; bottom: 16%; left: 20%; height: 1px; background: linear-gradient(90deg, transparent, var(--celebration-green), transparent); box-shadow: 0 0 22px var(--celebration-green); animation: foam-breathe 1800ms ease-in-out infinite; }

/* Dock stamp. */
.dock-stamp__grid { position: absolute; inset: 0; opacity: 0.32; background-image: linear-gradient(rgba(251, 119, 31, 0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(251, 119, 31, 0.08) 1px, transparent 1px); background-size: 52px 52px; transform: perspective(440px) rotateX(58deg) scale(1.35) translateY(22%); transform-origin: center bottom; }
.dock-stamp__ray { position: absolute; top: 50%; left: 50%; width: min(38vw, 450px); height: 1px; background: linear-gradient(90deg, transparent, rgba(251, 119, 31, 0.44), transparent); transform-origin: left center; transform: rotate(var(--ray-angle)) scaleX(0.1); opacity: 0; animation: stamp-ray 1200ms 120ms cubic-bezier(0.2, 0.75, 0.25, 1) both; }
.dock-stamp__seal { position: absolute; top: 17%; left: 50%; display: grid; width: 112px; height: 112px; place-items: center; color: var(--celebration-orange); background: rgba(251, 119, 31, 0.1); border: 1px dashed rgba(251, 119, 31, 0.6); border-radius: 50%; transform: translate(-50%, -50%); animation: seal-in 700ms 120ms cubic-bezier(0.2, 0.75, 0.25, 1) both; }
.dock-stamp__seal::before { position: absolute; inset: 8px; content: ''; border: 1px solid rgba(251, 119, 31, 0.35); border-radius: 50%; }
.dock-stamp__seal::after { position: absolute; bottom: -20px; content: 'SHIPPED'; font-size: 9px; font-weight: 700; letter-spacing: 0.2em; color: var(--celebration-orange); }
.dock-stamp__seal svg { width: 39px; height: 39px; stroke-width: 1.7; }

/* Sunrise. */
.sunrise__stars { position: absolute; inset: 0; opacity: 0.56; background-image: radial-gradient(circle at 10% 24%, #f8e7c9 0 1px, transparent 1.5px), radial-gradient(circle at 26% 12%, #f8e7c9 0 1px, transparent 1.5px), radial-gradient(circle at 68% 18%, #f8e7c9 0 1px, transparent 1.5px), radial-gradient(circle at 88% 28%, #f8e7c9 0 1px, transparent 1.5px), radial-gradient(circle at 51% 9%, #f8e7c9 0 1px, transparent 1.5px); animation: sunrise-stars 3000ms 900ms ease-out forwards; }
.sunrise__sunrise-glow { position: absolute; bottom: 12%; left: 50%; width: 70vw; height: 45vw; background: radial-gradient(ellipse at bottom, rgba(251, 119, 31, 0.3), transparent 68%); transform: translateX(-50%); }
.sunrise__sun { position: absolute; bottom: 12%; left: 50%; width: 116px; height: 116px; background: linear-gradient(#ffd166, #fb771f); border-radius: 50%; box-shadow: 0 0 90px rgba(251, 119, 31, 0.45); transform: translate(-50%, 58%); animation: sun-rise 2600ms 120ms cubic-bezier(0.2, 0.75, 0.25, 1) both; }
.sunrise__horizon { bottom: 12%; background: linear-gradient(90deg, transparent, rgba(255, 209, 102, 0.75), transparent); }
.sunrise__wave { position: absolute; right: -10%; bottom: 15%; left: -10%; height: 70px; border-top: 1px solid rgba(100, 207, 140, 0.5); border-radius: 50%; transform: rotate(-2deg); }
.sunrise__wave--two { bottom: 10%; opacity: 0.5; transform: rotate(2deg); }
.sunrise__boat { position: absolute; bottom: 13%; left: 65%; width: 160px; fill: #191322; stroke: rgba(255, 240, 248, 0.54); stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.2; animation: sunrise-boat 3000ms 240ms ease-out both; }

/* Paper fleet. */
.paper-fleet__horizon { bottom: 24%; opacity: 0.56; }
.paper-fleet__boat { --fleet-scale: 1; --fleet-rotation: 0deg; position: absolute; bottom: 22%; opacity: 0; animation: paper-drift 4200ms cubic-bezier(0.2, 0.75, 0.25, 1) both; }
.paper-fleet__boat svg { width: 150px; overflow: visible; }
.paper-fleet__boat--left { --fleet-scale: 0.72; --fleet-rotation: -5deg; left: 10%; }
.paper-fleet__boat--center { --fleet-scale: 1.12; --fleet-rotation: 3deg; left: 40%; }
.paper-fleet__boat--right { --fleet-scale: 0.58; --fleet-rotation: 8deg; right: 8%; }
.paper-fleet__fold { stroke: rgba(255, 240, 248, 0.65); stroke-linejoin: round; stroke-width: 1.2; }
.paper-fleet__fold--top { fill: #ffb14a; }
.paper-fleet__fold--front { fill: #d95822; }
.paper-fleet__fold--shadow { fill: rgba(77, 32, 40, 0.55); stroke: none; }
.paper-fleet__boat span { position: absolute; right: 24%; bottom: 12%; left: 20%; height: 1px; background: rgba(100, 207, 140, 0.55); border-radius: 50%; box-shadow: 0 0 10px rgba(100, 207, 140, 0.42); }
.paper-fleet__wake { position: absolute; right: 15%; bottom: 16%; left: 15%; height: 15px; border-top: 1px solid rgba(100, 207, 140, 0.6); border-radius: 50%; transform: rotate(-2deg); animation: foam-breathe 1800ms ease-in-out infinite; }

/* Firework sky. */
.firework-sky__burst { position: absolute; width: 8px; height: 8px; border-radius: 50%; background: var(--burst-color); box-shadow: 0 0 25px var(--burst-color); animation: firework-pop 2200ms cubic-bezier(0.2, 0.75, 0.25, 1) both; }
.firework-sky__burst--left { top: 28%; left: 22%; }
.firework-sky__burst--center { top: 19%; left: 51%; }
.firework-sky__burst--right { top: 34%; right: 17%; }
.firework-sky__spark { position: absolute; top: 50%; left: 50%; width: 2px; height: 46px; background: linear-gradient(var(--burst-color), transparent); transform-origin: center bottom; transform: translate(-50%, -100%) rotate(var(--spark-angle)) scaleY(0.1); opacity: 0; animation: firework-ray 1800ms 260ms ease-out both; }
.firework-sky__burst--center .firework-sky__spark { height: 62px; animation-delay: 520ms; }
.firework-sky__burst--right .firework-sky__spark { height: 38px; animation-delay: 780ms; }
.firework-sky__horizon { bottom: 16%; opacity: 0.42; }
.firework-sky__star { position: absolute; width: 3px; height: 3px; background: #f8e7c9; border-radius: 50%; box-shadow: 0 0 8px #f8e7c9; animation: star-pulse 1800ms ease-in-out infinite; }

/* Signal path. */
.signal-path__grid { position: absolute; inset: 0; opacity: 0.28; background-image: linear-gradient(rgba(100, 207, 140, 0.1) 1px, transparent 1px), linear-gradient(90deg, rgba(100, 207, 140, 0.1) 1px, transparent 1px); background-size: 56px 56px; mask-image: radial-gradient(ellipse at center, black, transparent 72%); }
.signal-path__route { position: absolute; inset: 0; width: 100%; height: 100%; }
.signal-path__route-line { fill: none; stroke: var(--celebration-orange); stroke-width: 3; stroke-linecap: round; stroke-linejoin: round; stroke-dasharray: 2000; stroke-dashoffset: 2000; animation: route-draw 3400ms 220ms cubic-bezier(0.2, 0.75, 0.25, 1) forwards; }
.signal-path__route-line--ghost { stroke: rgba(251, 119, 31, 0.12); stroke-width: 12; stroke-dasharray: none; stroke-dashoffset: 0; animation: none; }
.signal-path__node { position: absolute; display: grid; width: 15px; height: 15px; place-items: center; background: #161020; border: 1px solid rgba(100, 207, 140, 0.72); border-radius: 50%; box-shadow: 0 0 0 5px rgba(100, 207, 140, 0.06), 0 0 17px rgba(100, 207, 140, 0.3); opacity: 0; animation: node-pop 420ms cubic-bezier(0.2, 0.75, 0.25, 1) forwards; }
.signal-path__node span { width: 5px; height: 5px; background: var(--celebration-green); border-radius: 50%; }
.signal-path__node--1 { top: calc(73% - 8px); left: calc(22% - 8px); animation-delay: 700ms; }
.signal-path__node--2 { top: calc(46% - 8px); left: calc(29% - 8px); animation-delay: 1300ms; }
.signal-path__node--3 { top: calc(72% - 8px); left: calc(46% - 8px); animation-delay: 1900ms; }
.signal-path__node--4 { top: calc(46% - 8px); left: calc(61% - 8px); animation-delay: 2550ms; }
.signal-path__node--5 { top: calc(36% - 8px); left: calc(78% - 8px); animation-delay: 3300ms; }
.signal-path__beacon { position: absolute; top: calc(36% - 15px); right: 9%; display: grid; width: 30px; height: 30px; place-items: center; background: rgba(251, 119, 31, 0.15); border: 1px solid rgba(251, 119, 31, 0.52); border-radius: 50%; animation: beacon-pulse 1800ms 3500ms ease-out both; }
.signal-path__beacon::before, .signal-path__beacon::after { position: absolute; content: ''; border: 1px solid rgba(251, 119, 31, 0.36); border-radius: 50%; animation: beacon-wave 1800ms 3500ms ease-out both; }
.signal-path__beacon::before { inset: -9px; }
.signal-path__beacon::after { inset: -18px; animation-delay: 3650ms; }
.signal-path__beacon span { width: 9px; height: 9px; background: var(--celebration-orange); border-radius: 50%; box-shadow: 0 0 18px var(--celebration-orange); }

.celebration--reduced .celebration__scene { display: none; }
.celebration--reduced.celebration--quiet-handoff .celebration__scene { display: block; }
.celebration--reduced .quiet-handoff__glow,
.celebration--reduced .quiet-handoff__line { display: none; }
.celebration--reduced .quiet-handoff__preview-mark { opacity: 1; animation: none; }
.celebration--reduced .celebration__content,
.celebration--reduced .celebration__success-mark,
.celebration--reduced .celebration__success-mark svg { animation: none; }
.celebration--reduced .celebration__success-mark { transform: none; }
.celebration--reduced .celebration__success-ring { display: none; }

@keyframes celebration-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes celebration-out { from { opacity: 1; } to { opacity: 0; } }
@keyframes celebration-card-in { from { opacity: 0; transform: translateY(12px) scale(0.97); } to { opacity: 1; transform: translateY(0) scale(1); } }
@keyframes quiet-receipt-in { from { opacity: 0; transform: translateY(8px) scale(0.98); } to { opacity: 1; transform: translateY(0) scale(1); } }
@keyframes quiet-glow-in { from { opacity: 0; transform: translate(-50%, -50%) scale(0.9); } to { opacity: 1; transform: translate(-50%, -50%) scale(1); } }
@keyframes quiet-line-in { from { opacity: 0; transform: scaleX(0.72); } to { opacity: 1; transform: scaleX(1); } }
@keyframes quiet-mark-in { from { opacity: 0; transform: translate(-50%, -50%) scale(0.78); } 70% { opacity: 1; transform: translate(-50%, -50%) scale(1.04); } to { opacity: 1; transform: translate(-50%, -50%) scale(1); } }
@keyframes quiet-preview-mark-in { from { opacity: 0; transform: translateY(8px) scale(0.84); } 70% { opacity: 1; transform: translateY(-2px) scale(1.04); } to { opacity: 1; transform: translateY(0) scale(1); } }
@keyframes success-draw { from { opacity: 0; transform: scale(0.4) rotate(-18deg); } to { opacity: 1; transform: scale(1) rotate(0); } }
@keyframes success-ring { 0% { opacity: 0.55; transform: scale(0.55); } 75%, 100% { opacity: 0; transform: scale(1.28); } }
@keyframes sail-across { from { transform: translateX(0) translateY(12px) rotate(2deg); } 18% { transform: translateX(25vw) translateY(-5px) rotate(-1deg); } 66% { transform: translateX(75vw) translateY(4px) rotate(1deg); } to { transform: translateX(calc(100vw + 250px)) translateY(-10px) rotate(-2deg); } }
@keyframes wake-across { from { opacity: 0; transform: translateX(0) scaleX(0.3); } 18% { opacity: 0.85; } to { opacity: 0; transform: translateX(calc(100vw + 250px)) scaleX(1.2); } }
@keyframes beam-sweep { 0%, 100% { opacity: 0.1; transform: rotate(-25deg); } 50% { opacity: 0.85; transform: rotate(21deg); } }
@keyframes confetti-fly { 0% { opacity: 0; transform: translate(-50%, -50%) scale(0.2) rotate(0); } 12% { opacity: 1; } 100% { opacity: 0.9; transform: translate(calc(-50% + var(--x)), calc(-50% + var(--y) + 52vh)) rotate(var(--rotate)); } }
@keyframes burst-ring { from { opacity: 0.7; transform: translate(-50%, -50%) scale(0.2); } to { opacity: 0; transform: translate(-50%, -50%) scale(15); } }
@keyframes star-pulse { 0%, 100% { opacity: 0.32; transform: scale(0.65); } 50% { opacity: 1; transform: scale(1.25); } }
@keyframes route-draw { to { stroke-dashoffset: 0; } }
@keyframes check-draw { to { stroke-dashoffset: 0; } }
@keyframes tidal-expand { 0% { opacity: 0.7; transform: translate(-50%, -50%) scale(0.12); } 75% { opacity: 0.2; } 100% { opacity: 0; transform: translate(-50%, -50%) scale(13); } }
@keyframes waterline-drift { 0%, 100% { margin-left: -2%; } 50% { margin-left: 2%; } }
@keyframes foam-breathe { 0%, 100% { opacity: 0.35; transform: scaleX(0.88); } 50% { opacity: 0.9; transform: scaleX(1.08); } }
@keyframes stamp-ray { from { opacity: 0; transform: rotate(var(--ray-angle)) scaleX(0.1); } 28% { opacity: 0.75; } to { opacity: 0; transform: rotate(var(--ray-angle)) scaleX(1); } }
@keyframes seal-in { from { opacity: 0; transform: translate(-50%, -50%) scale(2.3) rotate(18deg); } 68% { transform: translate(-50%, -50%) scale(0.92) rotate(-3deg); } to { opacity: 1; transform: translate(-50%, -50%) scale(1) rotate(-5deg); } }
@keyframes sunrise-stars { from { opacity: 0.56; } to { opacity: 0.1; } }
@keyframes sun-rise { from { transform: translate(-50%, 58%); } to { transform: translate(-50%, 5%); } }
@keyframes sunrise-boat { from { opacity: 0; transform: translateX(20px); } 28% { opacity: 1; } to { opacity: 0.75; transform: translateX(-18px); } }
@keyframes paper-drift { from { opacity: 0; transform: translateY(22px) scale(var(--fleet-scale)) rotate(var(--fleet-rotation)); } 18% { opacity: 1; } 50% { transform: translateY(-8px) scale(var(--fleet-scale)) rotate(calc(var(--fleet-rotation) + 2deg)); } to { opacity: 0.82; transform: translateY(3px) scale(var(--fleet-scale)) rotate(var(--fleet-rotation)); } }
@keyframes firework-pop { from { opacity: 0; transform: scale(0.2); } 22% { opacity: 1; transform: scale(1); } 70%, 100% { opacity: 0.2; } }
@keyframes firework-ray { from { opacity: 0; transform: translate(-50%, -100%) rotate(var(--spark-angle)) scaleY(0.1); } 28% { opacity: 1; transform: translate(-50%, -100%) rotate(var(--spark-angle)) scaleY(1); } to { opacity: 0; transform: translate(-50%, -100%) rotate(var(--spark-angle)) scaleY(0.85); } }
@keyframes node-pop { from { opacity: 0; transform: scale(0.1); } 70% { transform: scale(1.18); } to { opacity: 1; transform: scale(1); } }
@keyframes beacon-pulse { from { transform: scale(0.5); opacity: 0; } 40% { opacity: 1; } to { transform: scale(1); opacity: 1; } }
@keyframes beacon-wave { from { opacity: 0.65; transform: scale(0.4); } to { opacity: 0; transform: scale(1.4); } }

@media (max-width: 620px) {
  .celebration__content { padding: 28px 24px 24px; }
  .sail-away__moon { right: 8%; }
  .constellation-route__map, .signal-path__route { transform: scale(1.3); }
  .sunrise__boat { left: 54%; }
}

@media (prefers-reduced-motion: reduce) {
  .celebration,
  .celebration--closing {
    animation-duration: 160ms !important;
    animation-timing-function: ease !important;
  }

  .celebration *,
  .celebration::before,
  .celebration::after {
    animation-duration: 1ms !important;
    animation-iteration-count: 1 !important;
    scroll-behavior: auto !important;
  }
}
</style>
