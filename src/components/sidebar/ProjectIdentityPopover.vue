<script setup lang="ts">
import { CircleAlert, Image as ImageIcon, ImagePlus, Palette, Trash2, Upload } from '@lucide/vue';
import { computed, nextTick, ref, watch } from 'vue';
import type { Project, ProjectImage } from '../../types/projects';
import { MAX_PROJECT_IMAGE_BYTES, normalizeHexColor } from '../../utils/projectIdentity';

type IdentityTab = 'color' | 'image';

const SUPPORTED_IMAGE_TYPES = new Set([
  'image/avif',
  'image/gif',
  'image/jpeg',
  'image/png',
  'image/webp',
]);
const IMAGE_TYPE_BY_EXTENSION: Record<string, string> = {
  avif: 'image/avif',
  gif: 'image/gif',
  jpeg: 'image/jpeg',
  jpg: 'image/jpeg',
  png: 'image/png',
  webp: 'image/webp',
};

const props = defineProps<{
  project: Project;
}>();

const emit = defineEmits<{
  color: [value: string | null];
  image: [value: ProjectImage | null];
}>();

const panel = ref<HTMLElement>();
const fileInput = ref<HTMLInputElement>();
const activeTab = ref<IdentityTab>('color');
const customColor = ref(props.project.colorOverride ?? props.project.color);
const imageError = ref<string | null>(null);
const imageBusy = ref(false);
const imagePreviewFailed = ref(false);

const popoverId = computed(() => `project-identity-${props.project.id.replace(/[^a-zA-Z0-9_-]/g, '-')}`);
const imageName = computed(() => props.project.image?.name ?? 'No image selected');
const imageSize = computed(() => (props.project.image ? formatBytes(props.project.image.size) : ''));

watch(
  () => [props.project.colorOverride, props.project.color] as const,
  ([override, color]) => {
    customColor.value = override ?? color;
  },
);

watch(
  () => props.project.image?.dataUrl,
  () => {
    imagePreviewFailed.value = false;
  },
);

function formatBytes(bytes: number) {
  if (!bytes) return 'Image';
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function focusTab(tab: IdentityTab) {
  nextTick(() => panel.value?.querySelector<HTMLElement>(`[data-identity-tab="${tab}"]`)?.focus());
}

function selectTab(tab: IdentityTab) {
  activeTab.value = tab;
}

function moveTab(event: KeyboardEvent) {
  const tabs: IdentityTab[] = ['color', 'image'];
  const current = tabs.indexOf(activeTab.value);
  let next = current;
  if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (current + 1) % tabs.length;
  if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (current - 1 + tabs.length) % tabs.length;
  if (event.key === 'Home') next = 0;
  if (event.key === 'End') next = tabs.length - 1;
  if (next === current) return;
  event.preventDefault();
  activeTab.value = tabs[next];
  focusTab(tabs[next]);
}

function chooseCustomColor(event: Event) {
  const value = normalizeHexColor((event.target as HTMLInputElement).value);
  if (!value) return;
  customColor.value = value;
  emit('color', value);
}

function openFilePicker() {
  imageError.value = null;
  fileInput.value?.click();
}

function fileType(file: File) {
  const type = file.type.toLowerCase();
  if (SUPPORTED_IMAGE_TYPES.has(type)) return type;
  const extension = file.name.split('.').pop()?.toLowerCase() ?? '';
  return IMAGE_TYPE_BY_EXTENSION[extension] ?? null;
}

function readImage(file: File, type: string) {
  return new Promise<ProjectImage>((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error('Shipyard could not read that file.'));
    reader.onload = () => {
      if (typeof reader.result !== 'string') {
        reject(new Error('Shipyard could not read that file.'));
        return;
      }
      const [, payload = ''] = reader.result.split(',', 2);
      const dataUrl = reader.result.startsWith(`data:${type};base64,`)
        ? reader.result
        : `data:${type};base64,${payload}`;
      const preview = new window.Image();
      preview.onload = () => {
        if (preview.naturalWidth > 4096 || preview.naturalHeight > 4096) {
          reject(new Error('That image is too large in pixel dimensions. Choose an image up to 4096 × 4096 pixels.'));
          return;
        }
        resolve({
          dataUrl,
          name: file.name,
          type,
          size: file.size,
          width: preview.naturalWidth,
          height: preview.naturalHeight,
        });
      };
      preview.onerror = () => reject(new Error('That file is not a readable image. Try a PNG or JPG.'));
      preview.src = dataUrl;
    };
    reader.readAsDataURL(file);
  });
}

async function onFileSelected(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  if (!file) return;

  imageError.value = null;
  const type = fileType(file);
  if (!type) {
    imageError.value = 'That file is not a supported image. Choose a PNG, JPG, GIF, WebP, or AVIF.';
    return;
  }
  if (!SUPPORTED_IMAGE_TYPES.has(type)) {
    imageError.value = 'That image format is not supported. Choose a PNG, JPG, GIF, WebP, or AVIF.';
    return;
  }
  if (file.size > MAX_PROJECT_IMAGE_BYTES) {
    imageError.value = 'Images must be 2 MB or smaller.';
    return;
  }
  if (file.size === 0) {
    imageError.value = 'That file is empty. Choose a different image.';
    return;
  }

  imageBusy.value = true;
  try {
    const image = await readImage(file, type);
    imagePreviewFailed.value = false;
    emit('image', image);
  } catch (error) {
    imageError.value = error instanceof Error ? error.message : 'Shipyard could not use that image.';
  } finally {
    imageBusy.value = false;
  }
}

function removeImage() {
  imageError.value = null;
  imagePreviewFailed.value = false;
  emit('image', null);
}
</script>

<template>
  <section
    :id="popoverId"
    ref="panel"
    class="project-identity-popover"
    role="dialog"
    :aria-label="`Customize ${project.name}`"
  >
    <div class="identity-tabs" role="tablist" aria-label="Project identity options" @keydown="moveTab">
      <button
        data-identity-tab="color"
        type="button"
        role="tab"
        :id="`${popoverId}-color-tab`"
        :aria-selected="activeTab === 'color'"
        :aria-controls="`${popoverId}-color-panel`"
        @click="selectTab('color')"
      >
        <Palette aria-hidden="true" />
        Color
      </button>
      <button
        data-identity-tab="image"
        type="button"
        role="tab"
        :id="`${popoverId}-image-tab`"
        :aria-selected="activeTab === 'image'"
        :aria-controls="`${popoverId}-image-panel`"
        @click="selectTab('image')"
      >
        <ImageIcon aria-hidden="true" />
        Image
      </button>
    </div>

    <div
      v-if="activeTab === 'color'"
      class="identity-tab-panel"
      role="tabpanel"
      :id="`${popoverId}-color-panel`"
      :aria-labelledby="`${popoverId}-color-tab`"
    >
      <strong class="identity-color__label">Choose a color</strong>

      <label class="custom-color-choice">
        <span class="custom-color-choice__input" :style="{ background: customColor }">
          <input
            type="color"
            :value="customColor"
            aria-label="Choose a custom project color"
            @input="chooseCustomColor"
          />
        </span>
        <span class="custom-color-choice__copy">
          <strong>Project color</strong>
          <small>{{ customColor.toUpperCase() }}</small>
        </span>
      </label>
    </div>

    <div
      v-else
      class="identity-tab-panel"
      role="tabpanel"
      :id="`${popoverId}-image-panel`"
      :aria-labelledby="`${popoverId}-image-tab`"
    >
      <input
        ref="fileInput"
        class="identity-image__input"
        type="file"
        accept="image/png,image/jpeg,image/gif,image/webp,image/avif"
        @change="onFileSelected"
      />

      <div v-if="project.image && !imagePreviewFailed" class="identity-image__preview-card">
        <div class="identity-image__preview" :style="{ background: project.color }">
          <img :src="project.image.dataUrl" :alt="`${project.name} project image preview`" @error="imagePreviewFailed = true" />
        </div>
        <div class="identity-image__details">
          <strong>{{ imageName }}</strong>
          <span>{{ imageSize }}<template v-if="project.image.width"> · {{ project.image.width }} × {{ project.image.height }}</template></span>
        </div>
        <div class="identity-image__actions">
          <button type="button" :disabled="imageBusy" @click="openFilePicker">
            <Upload aria-hidden="true" />
            Replace
          </button>
          <button type="button" :disabled="imageBusy" @click="removeImage">
            <Trash2 aria-hidden="true" />
            Remove
          </button>
        </div>
      </div>

      <div v-else class="identity-image__empty">
        <span class="identity-image__empty-icon"><ImagePlus aria-hidden="true" /></span>
        <strong>{{ imagePreviewFailed ? 'This image could not be previewed' : 'Add a project image' }}</strong>
        <span>{{ imagePreviewFailed ? 'Replace it with a PNG, JPG, GIF, WebP, or AVIF.' : 'A square image works best for project icons.' }}</span>
        <button type="button" :disabled="imageBusy" @click="openFilePicker">
          <Upload aria-hidden="true" />
          {{ imageBusy ? 'Reading image…' : imagePreviewFailed ? 'Replace image' : 'Choose image' }}
        </button>
        <button v-if="imagePreviewFailed" class="identity-image__remove-failed" type="button" @click="removeImage">
          Remove image
        </button>
      </div>

      <p v-if="imageError" class="identity-image__error" role="alert">
        <CircleAlert aria-hidden="true" />
        {{ imageError }}
      </p>
      <p class="identity-image__hint">PNG, JPG, GIF, WebP, or AVIF · Up to 2 MB</p>
    </div>

  </section>
</template>

<style scoped>
.project-identity-popover {
  position: absolute;
  z-index: 10;
  top: -9px;
  left: calc(100% + 9px);
  width: 306px;
  overflow: hidden;
  color: var(--text-primary);
  background: var(--surface-elevated);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  box-shadow: var(--shadow-elevated);
  transform-origin: top left;
  animation: identity-popover-in 130ms cubic-bezier(0.2, 0.75, 0.25, 1);
}

.identity-tabs {
  display: flex;
  gap: 4px;
  padding: 0 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.identity-tabs button {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 34px;
  padding: 0 8px;
  font: inherit;
  font-size: 11px;
  color: var(--text-secondary);
  background: transparent;
  border: 0;
}

.identity-tabs button:hover,
.identity-tabs button[aria-selected='true'] {
  color: var(--text-primary);
}

.identity-tabs button[aria-selected='true']::after {
  position: absolute;
  right: 7px;
  bottom: -1px;
  left: 7px;
  height: 2px;
  content: '';
  background: var(--primary);
  border-radius: 2px 2px 0 0;
}

.identity-tabs svg {
  width: 14px;
  height: 14px;
  stroke-width: 1.7;
}

.identity-tab-panel {
  min-height: 148px;
  padding: 10px 16px 8px;
}

.identity-color__label,
.custom-color-choice__copy strong {
  display: block;
  font-size: 12px;
  font-weight: 550;
}

.custom-color-choice:focus-within,
.identity-image__actions button:focus-visible,
.identity-image__empty button:focus-visible,
.identity-image__remove-failed:focus-visible,
.identity-tabs button:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 1px;
}

.custom-color-choice__input {
  display: grid;
  flex: 0 0 auto;
  place-items: center;
  border-radius: 50%;
  box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.32);
}

.custom-color-choice {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  margin-top: 8px;
  min-height: 50px;
  padding: 8px 10px;
  cursor: pointer;
  color: var(--text-primary);
  background: var(--surface-subtle);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  transition: background 100ms ease, border-color 100ms ease;
}

.custom-color-choice:hover {
  background: var(--surface-hover);
  border-color: var(--border-strong);
}

.custom-color-choice__input {
  position: relative;
  overflow: hidden;
  width: 24px;
  height: 24px;
}

.custom-color-choice__input input {
  position: absolute;
  inset: -4px;
  width: 25px;
  height: 25px;
  cursor: pointer;
  opacity: 0;
}

.custom-color-choice__copy {
  min-width: 0;
}

.custom-color-choice__copy small {
  display: block;
  margin-top: 2px;
  font: 10px ui-monospace, SFMono-Regular, Menlo, monospace;
  color: var(--text-muted);
}

.identity-image__input {
  display: none;
}

.identity-image__preview-card {
  display: grid;
  grid-template-columns: 50px minmax(0, 1fr);
  gap: 8px;
  align-items: center;
  padding: 8px;
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid var(--border-subtle);
  border-radius: 9px;
}

.identity-image__preview {
  display: grid;
  width: 50px;
  height: 50px;
  overflow: hidden;
  place-items: center;
  border-radius: 9px;
}

.identity-image__preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.identity-image__details {
  min-width: 0;
}

.identity-image__details strong,
.identity-image__details span {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.identity-image__details strong {
  font-size: 11px;
  font-weight: 550;
}

.identity-image__details span {
  margin-top: 4px;
  font-size: 10px;
  color: var(--text-secondary);
}

.identity-image__actions {
  display: flex;
  grid-column: 1 / -1;
  gap: 6px;
}

.identity-image__actions button,
.identity-image__empty button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 26px;
  padding: 0 9px;
  font: inherit;
  font-size: 10px;
  color: var(--text-secondary);
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
}

.identity-image__actions button:hover:not(:disabled),
.identity-image__empty button:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--surface-hover);
  border-color: var(--primary-border);
}

.identity-image__actions button:last-child:hover:not(:disabled) {
  color: var(--danger);
  background: var(--danger-subtle);
  border-color: var(--danger-border);
}

.identity-image__actions svg,
.identity-image__empty button svg {
  width: 13px;
  height: 13px;
}

.identity-image__empty {
  display: flex;
  min-height: 110px;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  padding: 6px 10px;
  text-align: center;
  background: rgba(255, 255, 255, 0.025);
  border: 1px dashed rgba(255, 240, 248, 0.17);
  border-radius: 9px;
}

.identity-image__empty-icon {
  display: grid;
  width: 32px;
  height: 32px;
  margin-bottom: 4px;
  place-items: center;
  color: var(--primary-hover);
  background: var(--primary-subtle);
  border: 1px solid var(--primary-border);
  border-radius: 12px;
}

.identity-image__empty-icon svg {
  width: 17px;
  height: 17px;
  stroke-width: 1.4;
}

.identity-image__empty > strong {
  font-size: 12px;
  font-weight: 550;
}

.identity-image__empty > span:not(.identity-image__empty-icon) {
  max-width: 220px;
  margin: 3px 0 6px;
  font-size: 9px;
  line-height: 1.25;
  color: var(--text-secondary);
}

.identity-image__empty button {
  color: var(--primary-hover);
  background: var(--primary-subtle);
  border-color: var(--primary-border);
}

.identity-image__remove-failed {
  height: auto !important;
  margin-top: 8px;
  padding: 0 !important;
  color: var(--text-muted) !important;
  background: transparent !important;
  border: 0 !important;
}

.identity-image__error {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  margin: 10px 0 0;
  font-size: 10px;
  line-height: 1.4;
  color: var(--danger);
}

.identity-image__error svg {
  flex: 0 0 auto;
  width: 13px;
  height: 13px;
  margin-top: 1px;
}

.identity-image__hint {
  margin: 6px 0 0;
  font-size: 9px;
  color: var(--text-muted);
  text-align: center;
}

@keyframes identity-popover-in {
  from {
    opacity: 0;
    transform: translateY(-3px) scale(0.985);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

@media (max-width: 740px) {
  .project-identity-popover {
    left: 0;
    top: 38px;
  }
}
</style>
