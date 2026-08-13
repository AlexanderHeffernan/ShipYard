<script setup lang="ts">
import { Sparkles } from '@lucide/vue';
import { ref } from 'vue';
import {
  getSunsetEffectEnabled,
  preloadSunsetEffect,
  setSunsetEffectEnabled,
} from '../../services/completionAnimation';

const sunsetEffect = ref(getSunsetEffectEnabled());

function updateSunsetEffect(event: Event) {
  sunsetEffect.value = setSunsetEffectEnabled((event.target as HTMLInputElement).checked);
  if (sunsetEffect.value) void preloadSunsetEffect();
}
</script>

<template>
  <main class="experimental-settings">
    <div class="experimental-settings__heading">
      <div>
        <h3>Experimental</h3>
        <p>Try new Shipyard features before they are generally available.</p>
      </div>
      <span class="experimental-settings__badge"><Sparkles aria-hidden="true" /> Experimental</span>
    </div>

    <section class="experimental-card">
      <div class="experimental-card__icon"><Sparkles aria-hidden="true" /></div>
      <div class="experimental-card__copy">
        <label for="sunset-effect">Full-screen ship effect</label>
        <p>Watch your ship sail into the sunset after shipping.</p>
      </div>
      <label class="toggle" for="sunset-effect">
        <input
          id="sunset-effect"
          type="checkbox"
          :checked="sunsetEffect"
          @change="updateSunsetEffect"
        />
        <span aria-hidden="true"></span>
      </label>
    </section>

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
  height: 22px;
  flex: 0 0 auto;
  align-items: center;
  gap: 5px;
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

.experimental-settings__badge svg { width: 12px; height: 12px; }

.experimental-card {
  display: flex;
  min-height: 72px;
  align-items: center;
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

.experimental-card__icon svg { width: 19px; height: 19px; stroke-width: 1.5; }
.experimental-card__copy { min-width: 0; flex: 1; }
.experimental-card__copy > label { font-size: 12px; font-weight: 600; color: var(--text-primary); }

.toggle { position: relative; display: block; width: 34px; height: 20px; flex: 0 0 auto; }
.toggle input { position: absolute; width: 1px; height: 1px; opacity: 0; }
.toggle span {
  position: absolute;
  inset: 0;
  cursor: pointer;
  background: var(--surface-input);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  transition: background 160ms ease, border-color 160ms ease;
}
.toggle span::after {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 12px;
  height: 12px;
  content: '';
  background: var(--text-secondary);
  border-radius: 50%;
  transition: transform 160ms ease, background 160ms ease;
}
.toggle input:checked + span { background: var(--primary); border-color: var(--primary); }
.toggle input:checked + span::after { background: white; transform: translateX(14px); }
.toggle input:focus-visible + span { outline: 2px solid var(--focus-ring); outline-offset: 2px; }

</style>
