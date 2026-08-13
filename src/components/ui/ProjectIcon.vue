<script setup lang="ts">
import { ref, watch } from 'vue';
import type { ProjectImage } from '../../types/projects';

const props = withDefaults(
  defineProps<{
    color: string;
    image: ProjectImage | null;
    size?: 'small' | 'medium';
  }>(),
  { size: 'small' },
);

const imageFailed = ref(false);

watch(
  () => props.image?.dataUrl,
  () => {
    imageFailed.value = false;
  },
);
</script>

<template>
  <span
    class="project-icon"
    :class="[`project-icon--${size}`, { 'project-icon--image': image && !imageFailed }]"
    :style="{ '--project-icon-color': color }"
    aria-hidden="true"
  >
    <img v-if="image && !imageFailed" :src="image.dataUrl" alt="" @error="imageFailed = true" />
    <span v-else class="project-icon__fallback"></span>
  </span>
</template>

<style scoped>
.project-icon {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  vertical-align: middle;
}

.project-icon--small {
  width: 12px;
  height: 12px;
}

.project-icon--medium {
  width: 18px;
  height: 18px;
}

.project-icon img,
.project-icon__fallback {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.28);
}

.project-icon--small img,
.project-icon--small .project-icon__fallback {
  border-radius: 50%;
}

.project-icon--medium img {
  border-radius: 5px;
}

.project-icon--medium .project-icon__fallback {
  border-radius: 50%;
}

.project-icon__fallback {
  background: var(--project-icon-color);
}
</style>
