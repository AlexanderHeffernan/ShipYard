<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import { Check, Eye, Sparkles } from '@lucide/vue';
import AppButton from '../ui/AppButton.vue';
import {
  completionAnimationOptions,
  type CompletionAnimation,
} from '../../types/celebration';
import { getCompletionAnimation, setCompletionAnimation } from '../../services/completionAnimation';

const emit = defineEmits<{ preview: [animation: CompletionAnimation] }>();
const selectedAnimation = ref<CompletionAnimation>(getCompletionAnimation());
const saved = ref(false);
const previewing = ref(false);
let savedTimer: number | undefined;
let previewTimer: number | undefined;

const selectedOption = computed(() => completionAnimationOptions.find(
  (option) => option.id === selectedAnimation.value,
) ?? completionAnimationOptions[0]);

function chooseAnimation(event: Event) {
  const value = (event.target as HTMLSelectElement).value as CompletionAnimation;
  selectedAnimation.value = setCompletionAnimation(value);
  saved.value = true;
  window.clearTimeout(savedTimer);
  savedTimer = window.setTimeout(() => (saved.value = false), 1400);
}

function previewAnimation() {
  previewing.value = true;
  emit('preview', selectedAnimation.value);
  window.clearTimeout(previewTimer);
  previewTimer = window.setTimeout(() => (previewing.value = false), 1400);
}

onBeforeUnmount(() => {
  window.clearTimeout(savedTimer);
  window.clearTimeout(previewTimer);
});
</script>

<template>
  <main class="experimental-settings">
    <div class="experimental-settings__heading">
      <div>
        <h3>Experimental</h3>
        <p>Small moments of delight for the end of a successful shipment.</p>
      </div>
      <span class="experimental-settings__badge"><Sparkles aria-hidden="true" /> Experimental</span>
    </div>

    <section class="experimental-card">
      <div class="experimental-card__icon"><Sparkles aria-hidden="true" /></div>
      <div class="experimental-card__content">
        <label for="completion-animation">Completion animation</label>
        <p>Shown once after a merge or push really completes. The default stays out of the way; the other ten options fill the screen.</p>
        <div class="experimental-card__controls">
          <select id="completion-animation" :value="selectedAnimation" @change="chooseAnimation">
            <option v-for="option in completionAnimationOptions" :key="option.id" :value="option.id">
              {{ option.label }}{{ option.default ? ' · Default' : '' }}
            </option>
          </select>
          <AppButton
            variant="ghost"
            size="small"
            type="button"
            :loading="previewing"
            loading-label="Previewing"
            @click="previewAnimation"
          >
            <Eye aria-hidden="true" /> Preview
          </AppButton>
        </div>
        <div class="experimental-card__selection">
          <Check v-if="saved" aria-hidden="true" />
          <div class="experimental-card__selection-copy">
            <strong>{{ selectedOption.label }}</strong>
            <span>{{ selectedOption.description }}</span>
          </div>
        </div>
      </div>
    </section>

    <p class="experimental-note">
      Preview is intentional and never changes project state. The automatic celebration is only triggered by a successful shipping run.
    </p>
  </main>
</template>

<style scoped>
.experimental-settings {
  min-width: 0;
  flex: 1;
  padding: 24px;
  overflow-y: auto;
}

.experimental-settings h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 550;
}

.experimental-settings p {
  margin: 5px 0 0;
  font-size: 11px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.experimental-settings__heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 14px;
  margin-bottom: 20px;
}

.experimental-settings__badge {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 5px;
  height: 22px;
  padding: 0 8px;
  font-size: 9px;
  font-weight: 600;
  color: var(--primary-hover);
  text-transform: uppercase;
  letter-spacing: 0.055em;
  background: var(--primary-subtle);
  border: 1px solid var(--primary-border);
  border-radius: 11px;
}

.experimental-settings__badge svg {
  width: 12px;
  height: 12px;
}

.experimental-card {
  display: flex;
  gap: 14px;
  padding: 16px;
  background: linear-gradient(135deg, rgba(251, 119, 31, 0.08), var(--surface-subtle) 48%);
  border: 1px solid var(--primary-border);
  border-radius: 11px;
}

.experimental-card__icon {
  display: grid;
  width: 38px;
  height: 38px;
  flex: 0 0 auto;
  place-items: center;
  color: var(--primary-hover);
  background: rgba(251, 119, 31, 0.13);
  border: 1px solid rgba(251, 119, 31, 0.25);
  border-radius: 10px;
}

.experimental-card__icon svg {
  width: 19px;
  height: 19px;
  stroke-width: 1.5;
}

.experimental-card__content {
  min-width: 0;
  flex: 1;
}

.experimental-card label {
  display: block;
  margin-bottom: 4px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.experimental-card__controls {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 14px;
}

.experimental-card select {
  min-width: 0;
  height: 32px;
  flex: 1;
  padding: 0 30px 0 9px;
  font: inherit;
  font-size: 11px;
  color: var(--text-primary);
  background-color: var(--surface-input);
  border: 1px solid var(--border-strong);
  border-radius: 7px;
  outline: none;
}

.experimental-card select:focus-visible {
  border-color: var(--focus-ring);
  box-shadow: 0 0 0 2px rgba(251, 119, 31, 0.16);
}

.experimental-card__controls :deep(.app-button) {
  flex: 0 0 auto;
}

.experimental-card__selection {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  min-width: 0;
  margin-top: 12px;
  padding-top: 11px;
  border-top: 1px solid var(--border-subtle);
}

.experimental-card__selection svg {
  width: 13px;
  height: 13px;
  flex: 0 0 auto;
  color: var(--success);
  stroke-width: 2;
}

.experimental-card__selection strong {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-primary);
}

.experimental-card__selection-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.experimental-card__selection span {
  min-width: 0;
  font-size: 10px;
  line-height: 1.35;
  color: var(--text-secondary);
}

.experimental-note {
  margin-top: 16px !important;
  color: var(--text-muted) !important;
}

@media (max-width: 620px) {
  .experimental-card__controls {
    align-items: stretch;
    flex-direction: column;
  }

  .experimental-card__selection {
    align-items: flex-start;
    flex-direction: column;
    gap: 3px;
  }

  .experimental-card__selection span {
    white-space: normal;
  }
}
</style>
