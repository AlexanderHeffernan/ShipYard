<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { checkoutPullRequest } from '../../services/projects';
import type { Project, WorkItem } from '../../types/projects';

const props = defineProps<{ project: Project; workItem: WorkItem }>();
const emit = defineEmits<{ checkedOut: [] }>();
const checkingOut = ref(false);
const error = ref<string | null>(null);

const unavailable = computed(() => !props.workItem.pullRequest || !props.project.githubRepository);

watch(
  () => props.workItem.worktreePath,
  (worktreePath) => {
    if (worktreePath) checkingOut.value = false;
  },
);

async function checkout() {
  const pullRequest = props.workItem.pullRequest;
  if (!pullRequest || checkingOut.value) return;
  checkingOut.value = true;
  error.value = null;
  try {
    await checkoutPullRequest({
      projectId: props.project.id,
      projectPath: props.project.path,
      pullRequestNumber: pullRequest.number,
      headSha: pullRequest.headSha,
    });
    emit('checkedOut');
  } catch (checkoutError) {
    error.value = checkoutError instanceof Error ? checkoutError.message : String(checkoutError);
    checkingOut.value = false;
  }
}
</script>

<template>
  <div class="checkout-control">
    <button type="button" :disabled="unavailable || checkingOut" :title="unavailable ? 'This pull request is not available for checkout' : 'Create a local checkout for this pull request'" @click="checkout">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 2.75v7.5m0 0L5.25 7.5M8 10.25l2.75-2.75M3.25 10.75v2h9.5v-2" /></svg>
      <span>{{ checkingOut ? 'Checking out…' : 'Check out' }}</span>
    </button>
    <p v-if="error">{{ error }}</p>
  </div>
</template>

<style scoped>
.checkout-control { position: relative; }
.checkout-control button { display: flex; align-items: center; gap: 5px; height: 24px; padding: 0 8px; font: inherit; font-size: 11px; font-weight: 600; color: var(--primary-foreground); background: var(--primary); border: 1px solid var(--primary); border-radius: 6px; }
.checkout-control button:hover:not(:disabled) { background: var(--primary-hover); border-color: var(--primary-hover); }
.checkout-control button:disabled { opacity: .45; }
.checkout-control button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }
.checkout-control svg { width: 13px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.35; }
.checkout-control p { position: absolute; z-index: 7; top: 25px; right: 0; width: 230px; margin: 4px 0 0; padding: 7px; font-size: 10px; color: var(--danger); background: var(--surface-elevated); border: 1px solid var(--border-strong); border-radius: 6px; box-shadow: var(--shadow-elevated); }
@media (max-width: 680px) { .checkout-control button { width: 26px; justify-content: center; padding: 0; } .checkout-control span { display: none; } }
</style>
