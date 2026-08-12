<script setup lang="ts">
import {
  CodeView,
  parsePatchFiles,
  type CodeViewItem,
  type FileDiffMetadata,
} from '@pierre/diffs';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { getWorkItemDiff } from '../../services/projects';
import type { Project, WorkItem } from '../../types/projects';

const props = defineProps<{
  project: Project;
  workItem: WorkItem;
}>();

type DiffMode = 'unified' | 'split';

const viewerRoot = ref<HTMLElement>();
const changesRoot = ref<HTMLElement>();
const fileListItems = ref<HTMLElement>();
const codeSearchInput = ref<HTMLInputElement>();
const loading = ref(false);
const error = ref<string | null>(null);
const comparisonLabel = ref('');
const files = ref<FileDiffMetadata[]>([]);
const filter = ref('');
const codeQuery = ref('');
const activeMatchIndex = ref(-1);
const selectedFile = ref<string | null>(null);
const viewedFiles = ref(new Set<string>());
const collapsedFiles = ref(new Set<string>());
const manuallyExpandedFiles = ref(new Set<string>());
const openingFile = ref<string | null>(null);
const openingVisible = ref(false);
const mode = ref<DiffMode>('unified');
const modeChosen = ref(false);
let viewer: CodeView | null = null;
let unsubscribeViewerScroll: (() => void) | null = null;
let resizeObserver: ResizeObserver | null = null;
let searchTimer: ReturnType<typeof setTimeout> | null = null;
let openingIndicatorTimer: ReturnType<typeof setTimeout> | null = null;
let requestVersion = 0;
const itemVersions = new Map<string, number>();
const LARGE_FILE_CHANGE_THRESHOLD = 250;

interface CodeSearchMatch {
  id: string;
  lineNumber: number;
  side: 'additions' | 'deletions';
}

const unsafeCSS = `
  :host {
    border-radius: 8px;
    overflow: clip;
    box-shadow: inset 0 0 0 1px var(--border-subtle);
  }
  [data-diffs-header] {
    min-height: 40px;
    border: 0;
    border-bottom: 1px solid var(--border-subtle);
    border-radius: 0;
    background: var(--surface-elevated);
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  }
  [data-diffs-header] + pre,
  pre {
    border: 0;
  }
  [data-line][data-selected-line],
  [data-column-number][data-selected-line] {
    box-shadow: inset 3px 0 var(--primary), inset 0 1px var(--primary-border), inset 0 -1px var(--primary-border);
  }
`;

const filteredFiles = computed(() => {
  const query = filter.value.trim().toLowerCase();
  return query ? files.value.filter((file) => file.name.toLowerCase().includes(query)) : files.value;
});

const totals = computed(() => files.value.reduce(
  (total, file) => {
    for (const hunk of file.hunks) {
      total.additions += hunk.additionLines;
      total.deletions += hunk.deletionLines;
    }
    return total;
  },
  { additions: 0, deletions: 0 },
));

const viewedCount = computed(() => files.value.filter((file) => viewedFiles.value.has(file.name)).length);

const codeMatches = computed<CodeSearchMatch[]>(() => {
  const query = codeQuery.value.trim().toLocaleLowerCase();
  if (!query) return [];
  const matches: CodeSearchMatch[] = [];

  for (const file of files.value) {
    for (const hunk of file.hunks) {
      for (let offset = 0; offset < hunk.additionCount; offset += 1) {
        if (file.additionLines[hunk.additionLineIndex + offset]?.toLocaleLowerCase().includes(query)) {
          matches.push({ id: file.name, lineNumber: hunk.additionStart + offset, side: 'additions' });
        }
      }
      for (const block of hunk.hunkContent) {
        if (block.type !== 'change') continue;
        for (let offset = 0; offset < block.deletions; offset += 1) {
          const lineIndex = block.deletionLineIndex + offset;
          if (file.deletionLines[lineIndex]?.toLocaleLowerCase().includes(query)) {
            matches.push({
              id: file.name,
              lineNumber: hunk.deletionStart + lineIndex - hunk.deletionLineIndex,
              side: 'deletions',
            });
          }
        }
      }
    }
  }
  return matches;
});

function createHeaderButton(collapsed: boolean, onClick: () => void) {
  const button = document.createElement('button');
  const label = collapsed ? 'Expand' : 'Collapse';
  button.type = 'button';
  button.title = `${label} file changes`;
  button.setAttribute('aria-label', button.title);
  button.style.cssText = 'appearance:none;display:flex;align-items:center;gap:4px;height:22px;border:1px solid var(--primary-border);border-radius:6px;color:var(--primary);background:var(--primary-subtle);font:600 10px/20px -apple-system,BlinkMacSystemFont,sans-serif;padding:0 7px;cursor:pointer';
  const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  icon.setAttribute('viewBox', '0 0 12 12');
  icon.setAttribute('aria-hidden', 'true');
  icon.style.cssText = `width:11px;height:11px;fill:none;stroke:currentColor;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round;transform:${collapsed ? 'rotate(0deg)' : 'rotate(180deg)'}`;
  icon.innerHTML = '<path d="m3 4.5 3 3 3-3" />';
  button.append(icon, document.createTextNode(label));
  button.addEventListener('click', (event) => {
    event.stopPropagation();
    onClick();
  });
  return button;
}

function createHeaderControls(_file: unknown, context: { item: CodeViewItem }) {
  const controls = document.createElement('span');
  const id = context.item.id;
  const viewed = viewedFiles.value.has(id);
  const collapsed = collapsedFiles.value.has(id);
  controls.style.cssText = 'display:flex;align-items:center;gap:6px;margin-left:2px';

  const viewedLabel = document.createElement('label');
  viewedLabel.title = viewed ? 'Mark file as not viewed' : 'Mark file as viewed';
  viewedLabel.style.cssText = `display:flex;align-items:center;gap:5px;height:22px;padding:0 7px;color:${viewed ? 'var(--primary)' : 'var(--text-secondary)'};background:${viewed ? 'var(--primary-subtle)' : 'transparent'};border:1px solid ${viewed ? 'var(--primary-border)' : 'var(--border-subtle)'};border-radius:6px;font:600 10px/20px -apple-system,BlinkMacSystemFont,sans-serif;cursor:pointer`;
  const checkbox = document.createElement('input');
  checkbox.type = 'checkbox';
  checkbox.checked = viewed;
  checkbox.setAttribute('aria-label', viewedLabel.title);
  checkbox.style.cssText = `appearance:none;display:grid;width:13px;height:13px;margin:0;place-items:center;border:1px solid ${viewed ? 'var(--primary)' : 'var(--border-strong)'};border-radius:50%;background:${viewed ? 'var(--primary)' : 'transparent'};cursor:pointer`;
  if (viewed) {
    checkbox.style.backgroundImage = 'url("data:image/svg+xml,%3Csvg xmlns=%27http://www.w3.org/2000/svg%27 viewBox=%270 0 12 12%27%3E%3Cpath d=%27m2.5 6 2.2 2.2 4.8-5%27 fill=%27none%27 stroke=%27%23100d18%27 stroke-width=%271.8%27 stroke-linecap=%27round%27 stroke-linejoin=%27round%27/%3E%3C/svg%3E")';
  }
  checkbox.addEventListener('click', (event) => event.stopPropagation());
  checkbox.addEventListener('change', () => toggleViewed(id));
  viewedLabel.append(checkbox, document.createTextNode('Viewed'));
  controls.append(
    viewedLabel,
    createHeaderButton(collapsed, () => toggleCollapsed(id)),
  );
  return controls;
}

function viewerOptions() {
  const bottomNavigationSpace = Math.max(28, (viewerRoot.value?.clientHeight ?? 0) - 54);
  return {
    diffStyle: mode.value,
    diffIndicators: 'bars' as const,
    lineDiffType: 'word-alt' as const,
    overflow: 'scroll' as const,
    themeType: 'dark' as const,
    hunkSeparators: 'line-info-basic' as const,
    collapsedContextThreshold: 8,
    expansionLineCount: 20,
    stickyHeaders: true,
    renderHeaderFilenameSuffix: createHeaderControls,
    unsafeCSS,
    // Leave enough trailing scroll range for the final file header to align at the top.
    layout: { paddingTop: 12, paddingBottom: bottomNavigationSpace, gap: 14 },
  };
}

function codeItems(): CodeViewItem[] {
  return filteredFiles.value.map((file) => ({
    id: file.name,
    type: 'diff',
    fileDiff: file,
    collapsed: collapsedFiles.value.has(file.name),
    version: itemVersions.get(file.name) ?? 0,
  }));
}

function finishFileNavigation(_scrollTop: number) {
  const name = openingFile.value;
  if (!name || viewer?.getRenderedItems().every((item) => item.id !== name)) return;
  openingFile.value = null;
  openingVisible.value = false;
  if (openingIndicatorTimer) clearTimeout(openingIndicatorTimer);
  openingIndicatorTimer = null;
}

async function renderViewer(reset = false) {
  await nextTick();
  if (!viewerRoot.value) return;
  if (reset) {
    unsubscribeViewerScroll?.();
    unsubscribeViewerScroll = null;
    viewer?.cleanUp();
    viewer = null;
  }
  if (!viewer) {
    viewer = new CodeView(viewerOptions());
    viewer.setup(viewerRoot.value);
    unsubscribeViewerScroll = viewer.subscribeToScroll(finishFileNavigation);
  } else {
    viewer.setOptions(viewerOptions());
  }
  viewer.setItems(codeItems());
  viewer.render(true);
}

async function loadDiff() {
  const version = ++requestVersion;
  loading.value = true;
  error.value = null;
  files.value = [];
  selectedFile.value = null;
  viewer?.setItems([]);
  try {
    const result = await getWorkItemDiff({
      projectPath: props.project.path,
      projectId: props.project.id,
      branch: props.workItem.branch,
      worktreePath: props.workItem.worktreePath,
      headSha: props.workItem.headSha,
      defaultBranch: props.project.defaultBranch,
      pullRequestNumber: props.workItem.pullRequest?.number ?? null,
    });
    if (version !== requestVersion) return;
    const parsed = parsePatchFiles(result.patch, props.workItem.headSha, true);
    files.value = parsed.flatMap((patch) => patch.files);
    itemVersions.clear();
    collapsedFiles.value = new Set(files.value.flatMap((file) => {
      const counts = fileCounts(file);
      return counts.additions + counts.deletions > LARGE_FILE_CHANGE_THRESHOLD
        && !manuallyExpandedFiles.value.has(file.name)
        ? [file.name]
        : [];
    }));
    comparisonLabel.value = result.comparisonLabel;
    selectedFile.value = files.value[0]?.name ?? null;
    loading.value = false;
    await renderViewer(true);
  } catch (loadError) {
    if (version === requestVersion) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    }
  } finally {
    if (version === requestVersion) loading.value = false;
  }
}

function fileCounts(file: FileDiffMetadata) {
  return file.hunks.reduce(
    (total, hunk) => ({
      additions: total.additions + hunk.additionLines,
      deletions: total.deletions + hunk.deletionLines,
    }),
    { additions: 0, deletions: 0 },
  );
}

function statusLabel(file: FileDiffMetadata) {
  if (file.type === 'new') return 'A';
  if (file.type === 'deleted') return 'D';
  if (file.type.startsWith('rename')) return 'R';
  return 'M';
}

function statusName(file: FileDiffMetadata) {
  if (file.type === 'new') return 'Added';
  if (file.type === 'deleted') return 'Deleted';
  if (file.type.startsWith('rename')) return 'Renamed';
  return 'Modified';
}

function selectFile(name: string) {
  selectedFile.value = name;
  openingFile.value = name;
  openingVisible.value = false;
  if (openingIndicatorTimer) clearTimeout(openingIndicatorTimer);
  openingIndicatorTimer = setTimeout(() => {
    openingIndicatorTimer = null;
    if (openingFile.value === name) openingVisible.value = true;
  }, 120);

  if (collapsedFiles.value.has(name)) toggleCollapsed(name, true);

  const firstHunk = files.value.find((file) => file.name === name)?.hunks[0];
  if (firstHunk) {
    const additionsAvailable = firstHunk.additionCount > 0;
    // CodeView keeps this target pending while it recalculates virtual item heights.
    // Targeting the first real patch row also reserves room for the sticky header.
    viewer?.scrollTo({
      type: 'line',
      id: name,
      lineNumber: additionsAvailable ? firstHunk.additionStart : firstHunk.deletionStart,
      side: additionsAvailable ? 'additions' : 'deletions',
      align: 'start',
      offset: 20,
      behavior: 'instant',
    });
  } else {
    viewer?.scrollTo({ type: 'item', id: name, align: 'start', behavior: 'instant' });
  }
  requestAnimationFrame(() => {
    if (openingFile.value === name && viewerRoot.value) {
      finishFileNavigation(viewerRoot.value.scrollTop);
    }
  });
}

function revealSidebarFile(name: string) {
  const container = fileListItems.value;
  const button = Array.from(container?.querySelectorAll<HTMLButtonElement>('[data-file-name]') ?? [])
    .find((item) => item.dataset.fileName === name);
  if (!container || !button) return;
  if (button.offsetTop < container.scrollTop) container.scrollTop = button.offsetTop - 5;
  else if (button.offsetTop + button.offsetHeight > container.scrollTop + container.clientHeight) {
    container.scrollTop = button.offsetTop + button.offsetHeight - container.clientHeight + 5;
  }
}

function toggleViewed(name: string) {
  const next = new Set(viewedFiles.value);
  const markingViewed = !next.has(name);
  if (markingViewed) next.add(name);
  else next.delete(name);
  viewedFiles.value = next;
  if (markingViewed && !collapsedFiles.value.has(name)) toggleCollapsed(name);
  else updateViewerItem(name);
}

function toggleCollapsed(name: string, forceExpanded = false) {
  const next = new Set(collapsedFiles.value);
  const expanding = forceExpanded || next.has(name);
  if (expanding) {
    next.delete(name);
    manuallyExpandedFiles.value = new Set(manuallyExpandedFiles.value).add(name);
  } else {
    next.add(name);
  }
  collapsedFiles.value = next;
  updateViewerItem(name);
}

function updateViewerItem(name: string) {
  const item = viewer?.getItem(name);
  if (item?.type !== 'diff') return;
  const version = (itemVersions.get(name) ?? 0) + 1;
  itemVersions.set(name, version);
  viewer?.updateItem({
    ...item,
    collapsed: collapsedFiles.value.has(name),
    version,
  });
}

function moveCodeMatch(direction: 1 | -1) {
  const matches = codeMatches.value;
  if (!matches.length) return;
  activeMatchIndex.value = (activeMatchIndex.value + direction + matches.length) % matches.length;
  const match = matches[activeMatchIndex.value];
  selectedFile.value = match.id;
  revealSidebarFile(match.id);
  if (collapsedFiles.value.has(match.id)) toggleCollapsed(match.id, true);
  requestAnimationFrame(() => viewer?.scrollTo({
    type: 'line',
    id: match.id,
    lineNumber: match.lineNumber,
    side: match.side,
    align: 'center',
    behavior: 'smooth-auto',
  }));
  viewer?.setSelectedLines({
    id: match.id,
    range: {
      start: match.lineNumber,
      end: match.lineNumber,
      side: match.side,
      endSide: match.side,
    },
  });
}

function navigate(direction: number) {
  if (filteredFiles.value.length === 0) return;
  const current = filteredFiles.value.findIndex((file) => file.name === selectedFile.value);
  const index = current < 0
    ? 0
    : Math.min(filteredFiles.value.length - 1, Math.max(0, current + direction));
  selectFile(filteredFiles.value[index].name);
}

function handleKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
    event.preventDefault();
    codeSearchInput.value?.focus();
    codeSearchInput.value?.select();
    return;
  }
  if (event.target instanceof HTMLInputElement) return;
  if (event.key === 'n' || (event.altKey && event.key === 'ArrowDown')) {
    event.preventDefault();
    navigate(1);
  } else if (event.key === 'p' || (event.altKey && event.key === 'ArrowUp')) {
    event.preventDefault();
    navigate(-1);
  }
}

function chooseMode(next: DiffMode) {
  modeChosen.value = true;
  mode.value = next;
}

watch(
  () => `${props.project.id}:${props.workItem.id}:${props.workItem.headSha}:${props.workItem.updatedAt}`,
  loadDiff,
  { immediate: true },
);
watch(filter, () => void renderViewer());
watch(mode, () => void renderViewer());
watch(codeQuery, (query) => {
  if (searchTimer) clearTimeout(searchTimer);
  activeMatchIndex.value = -1;
  viewer?.clearSelectedLines();
  if (!query.trim()) return;
  searchTimer = setTimeout(() => {
    searchTimer = null;
    if (codeMatches.value.length) moveCodeMatch(1);
  }, 120);
});

onMounted(() => {
  resizeObserver = new ResizeObserver(([entry]) => {
    if (!modeChosen.value) mode.value = entry.contentRect.width >= 1300 ? 'split' : 'unified';
    viewer?.setOptions(viewerOptions());
  });
  if (changesRoot.value) resizeObserver.observe(changesRoot.value);
});

onBeforeUnmount(() => {
  requestVersion += 1;
  if (searchTimer) clearTimeout(searchTimer);
  if (openingIndicatorTimer) clearTimeout(openingIndicatorTimer);
  unsubscribeViewerScroll?.();
  resizeObserver?.disconnect();
  viewer?.cleanUp();
});
</script>

<template>
  <section ref="changesRoot" class="changes" tabindex="-1" @keydown="handleKeydown">
    <header class="changes__toolbar">
      <div class="changes__summary">
        <strong>{{ files.length }} file{{ files.length === 1 ? '' : 's' }}</strong>
        <span v-if="comparisonLabel">compared with {{ comparisonLabel }}</span>
        <span class="changes__additions">+{{ totals.additions }}</span>
        <span class="changes__deletions">−{{ totals.deletions }}</span>
      </div>
      <label class="code-search">
        <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.25" /><path d="m10.25 10.25 3 3" /></svg>
        <input
          ref="codeSearchInput"
          v-model="codeQuery"
          type="search"
          placeholder="Search changed code"
          aria-label="Search changed code"
          @keydown.enter.prevent="moveCodeMatch($event.shiftKey ? -1 : 1)"
        />
        <span v-if="codeQuery" class="code-search__count">
          {{ codeMatches.length ? `${activeMatchIndex + 1 || '–'} / ${codeMatches.length}` : 'No matches' }}
        </span>
        <kbd v-else>⌘F</kbd>
        <button type="button" :disabled="!codeMatches.length" title="Previous match" aria-label="Previous code match" @click="moveCodeMatch(-1)">↑</button>
        <button type="button" :disabled="!codeMatches.length" title="Next match" aria-label="Next code match" @click="moveCodeMatch(1)">↓</button>
      </label>
      <div class="changes__actions">
        <button type="button" title="Previous file (P)" aria-label="Previous file" @click="navigate(-1)">
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4.5 9.5 3.5-3 3.5 3" /></svg>
        </button>
        <button type="button" title="Next file (N)" aria-label="Next file" @click="navigate(1)">
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4.5 6.5 3.5 3 3.5-3" /></svg>
        </button>
        <div class="changes__mode" aria-label="Diff layout">
          <button type="button" :class="{ active: mode === 'unified' }" @click="chooseMode('unified')">Unified</button>
          <button type="button" :class="{ active: mode === 'split' }" @click="chooseMode('split')">Split</button>
        </div>
      </div>
    </header>

    <div v-if="loading" class="changes__state" aria-live="polite">
      <span class="changes__spinner"></span>
      <strong>Preparing changes…</strong>
      <span>Reading the worktree and highlighting changed files.</span>
    </div>
    <div v-else-if="error" class="changes__state changes__state--error" role="alert">
      <strong>Couldn’t load changes</strong>
      <span>{{ error }}</span>
      <button type="button" @click="loadDiff">Try again</button>
    </div>
    <div v-else-if="files.length === 0" class="changes__state">
      <strong>No file changes</strong>
      <span>This work item currently matches its comparison point.</span>
    </div>

    <div v-else class="changes__workspace">
      <aside class="file-list" aria-label="Changed files">
        <label class="file-list__search">
          <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.25" /><path d="m10.25 10.25 3 3" /></svg>
          <input v-model="filter" type="search" placeholder="Filter files" aria-label="Filter changed files" />
        </label>
        <div class="file-list__heading">
          <span>Files</span>
          <span>{{ viewedCount }}/{{ files.length }} viewed</span>
        </div>
        <div ref="fileListItems" class="file-list__items">
          <button
            v-for="file in filteredFiles"
            :key="file.name"
            type="button"
            :data-file-name="file.name"
            class="file-list__item"
            :class="{ 'file-list__item--active': selectedFile === file.name }"
            :aria-busy="openingFile === file.name"
            @click="selectFile(file.name)"
          >
            <span class="file-list__status" :class="`file-list__status--${file.type}`" :title="statusName(file)">
              {{ statusLabel(file) }}
            </span>
            <span class="file-list__path" :title="file.name">
              <strong>{{ file.name.split('/').pop() }}</strong>
              <small v-if="openingFile === file.name && openingVisible" class="file-list__opening">
                <i aria-hidden="true"></i>Opening…
              </small>
              <small v-else-if="file.name.includes('/')">{{ file.name.slice(0, file.name.lastIndexOf('/')) }}</small>
            </span>
            <span class="file-list__counts">
              <i v-if="fileCounts(file).additions">+{{ fileCounts(file).additions }}</i>
              <b v-if="fileCounts(file).deletions">−{{ fileCounts(file).deletions }}</b>
            </span>
            <span
              class="file-list__viewed"
              :class="{ 'file-list__viewed--checked': viewedFiles.has(file.name) }"
              :title="viewedFiles.has(file.name) ? 'Mark as not viewed' : 'Mark as viewed'"
              role="checkbox"
              :aria-checked="viewedFiles.has(file.name)"
              @click.stop="toggleViewed(file.name)"
            >
              <svg v-if="viewedFiles.has(file.name)" viewBox="0 0 12 12" aria-hidden="true"><path d="m2.5 6 2.2 2.2 4.8-5" /></svg>
            </span>
          </button>
          <p v-if="filteredFiles.length === 0">No files match “{{ filter }}”.</p>
        </div>
      </aside>
      <div ref="viewerRoot" class="changes__viewer"></div>
    </div>
  </section>
</template>

<style scoped>
.changes {
  display: flex;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  outline: none;
  background: var(--surface-content);
}

.changes__toolbar {
  display: flex;
  min-height: 45px;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px 0 16px;
  border-bottom: 1px solid var(--border-subtle);
  background: rgba(16, 13, 24, 0.92);
}

.code-search {
  display: flex;
  width: clamp(210px, 30vw, 340px);
  height: 29px;
  align-items: center;
  gap: 5px;
  margin: 0 12px;
  padding: 0 4px 0 8px;
  color: var(--text-muted);
  background: var(--surface-input);
  border: 1px solid var(--border-subtle);
  border-radius: 7px;
}
.code-search:focus-within { border-color: var(--primary-border); box-shadow: 0 0 0 1px var(--primary-subtle); }
.code-search > svg { width: 13px; flex: 0 0 auto; fill: none; stroke: currentColor; stroke-width: 1.25; }
.code-search input { min-width: 80px; flex: 1; padding: 0; color: var(--text-primary); background: transparent; border: 0; outline: 0; font: inherit; font-size: 11px; }
.code-search input::placeholder { color: var(--text-muted); }
.code-search kbd, .code-search__count { white-space: nowrap; color: var(--text-muted); font-family: inherit; font-size: 9px; line-height: 20px; }
.code-search button { width: 20px; height: 20px; padding: 0; color: var(--text-secondary); background: transparent; border: 0; border-radius: 4px; font: 11px/20px inherit; }
.code-search button:not(:disabled):hover { color: var(--text-primary); background: var(--surface-hover); }
.code-search button:disabled { opacity: .3; }

.changes__summary,
.changes__actions {
  display: flex;
  align-items: center;
}

.changes__summary { gap: 9px; font-size: 11px; color: var(--text-muted); }
.changes__summary strong { font-size: 12px; font-weight: 550; color: var(--text-primary); }
.changes__additions { color: var(--success); }
.changes__deletions { color: var(--danger); }
.changes__actions { gap: 4px; }

.changes__actions > button,
.changes__mode button {
  height: 27px;
  padding: 0 8px;
  color: var(--text-secondary);
  background: transparent;
  border: 0;
  border-radius: 5px;
  font: inherit;
  font-size: 11px;
}

.changes__actions > button { width: 28px; padding: 0; }
.changes__actions svg { width: 16px; fill: none; stroke: currentColor; stroke-width: 1.3; stroke-linecap: round; stroke-linejoin: round; }
.changes__actions button:hover { color: var(--text-primary); background: var(--surface-hover); }
.changes__actions button:focus-visible { outline: 2px solid var(--focus-ring); }

.changes__mode {
  display: flex;
  margin-left: 5px;
  padding: 2px;
  background: var(--surface-input);
  border: 1px solid var(--border-subtle);
  border-radius: 7px;
}

.changes__mode button.active { color: var(--text-primary); background: var(--surface-elevated); box-shadow: 0 0 0 1px var(--border-subtle); }

.changes__workspace { display: grid; height: 0; min-height: 0; flex: 1; overflow: hidden; grid-template-columns: 224px minmax(0, 1fr); }
.changes__viewer {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  overscroll-behavior: contain;
  padding: 0 14px;
  --diffs-font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  --diffs-header-font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  --diffs-font-size: 11.5px;
  --diffs-line-height: 19px;
  --diffs-bg: var(--surface-content);
  --diffs-bg-buffer-override: var(--surface-content);
  --diffs-bg-context-override: var(--surface-content);
  --diffs-bg-context-gutter-override: color-mix(in srgb, var(--surface-elevated) 55%, var(--surface-content));
  --diffs-bg-addition-override: color-mix(in srgb, var(--success) 16%, var(--surface-content));
  --diffs-bg-addition-number-override: color-mix(in srgb, var(--success) 24%, var(--surface-content));
  --diffs-bg-addition-emphasis-override: color-mix(in srgb, var(--success) 36%, var(--surface-content));
  --diffs-bg-deletion-override: color-mix(in srgb, var(--danger) 17%, var(--surface-content));
  --diffs-bg-deletion-number-override: color-mix(in srgb, var(--danger) 25%, var(--surface-content));
  --diffs-bg-deletion-emphasis-override: color-mix(in srgb, var(--danger) 37%, var(--surface-content));
  --diffs-addition-color-override: var(--success);
  --diffs-deletion-color-override: var(--danger);
  --diffs-fg-number-override: var(--text-muted);
  --diffs-bg-separator-override: color-mix(in srgb, var(--surface-elevated) 62%, var(--surface-content));
  --diffs-bg-selection-override: color-mix(in srgb, var(--primary) 27%, var(--surface-content));
  --diffs-bg-selection-number-override: color-mix(in srgb, var(--primary) 36%, var(--surface-content));
}

.file-list { display: flex; min-width: 0; min-height: 0; overflow: hidden; flex-direction: column; border-right: 1px solid var(--border-subtle); background: rgba(24, 15, 29, 0.32); }
.file-list__search { display: flex; height: 46px; flex: 0 0 auto; align-items: center; gap: 7px; margin: 0 10px; }
.file-list__search svg { width: 14px; flex: 0 0 auto; fill: none; stroke: var(--text-muted); stroke-width: 1.25; }
.file-list__search input { min-width: 0; flex: 1; padding: 0; color: var(--text-primary); background: none; border: 0; outline: 0; font: inherit; font-size: 11px; }
.file-list__search input::placeholder { color: var(--text-muted); }
.file-list__search kbd { font-family: inherit; font-size: 9px; color: var(--text-muted); }
.file-list__heading { display: flex; height: 25px; flex: 0 0 auto; align-items: center; justify-content: space-between; padding: 0 10px; color: var(--text-muted); font-size: 9px; font-weight: 600; letter-spacing: .05em; text-transform: uppercase; border-top: 1px solid var(--border-subtle); border-bottom: 1px solid var(--border-subtle); }
.file-list__items { min-height: 0; flex: 1; overflow-y: auto; overscroll-behavior: contain; padding: 5px; scrollbar-gutter: stable; }
.file-list__items { scrollbar-width: thin; scrollbar-color: rgba(255, 255, 255, 0.18) transparent; }
.file-list__items::-webkit-scrollbar { width: 6px; }
.file-list__items::-webkit-scrollbar-track { background: transparent; }
.file-list__items::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.18); border-radius: 999px; }
.file-list__items::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.28); }
.file-list__items > p { padding: 16px 10px; color: var(--text-muted); font-size: 11px; line-height: 1.5; }
.file-list__item { display: grid; width: 100%; min-width: 0; height: 42px; align-items: center; grid-template-columns: 17px minmax(0, 1fr) auto 16px; gap: 5px; padding: 0 5px; color: var(--text-secondary); text-align: left; background: transparent; border: 0; border-radius: 6px; font: inherit; }
.file-list__item:hover { background: var(--surface-hover); }
.file-list__item--active { color: var(--text-primary); background: var(--surface-active) !important; }
.file-list__item:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
.file-list__status { font: 600 10px/1 ui-monospace, monospace; text-align: center; }
.file-list__status--new { color: var(--success); }
.file-list__status--deleted { color: var(--danger); }
.file-list__status--rename-pure, .file-list__status--rename-changed { color: #73b7f5; }
.file-list__status--change { color: var(--warning); }
.file-list__path { display: flex; min-width: 0; flex-direction: column; gap: 2px; }
.file-list__path strong, .file-list__path small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.file-list__path strong { font-size: 11px; font-weight: 500; }
.file-list__path small { color: var(--text-muted); font-size: 9px; }
.file-list__opening { display: flex; align-items: center; gap: 4px; color: var(--primary) !important; }
.file-list__opening i { width: 8px; height: 8px; flex: 0 0 auto; border: 1px solid var(--primary-border); border-top-color: var(--primary); border-radius: 50%; animation: spin .7s linear infinite; }
.file-list__counts { display: flex; gap: 3px; font: 9px ui-monospace, monospace; }
.file-list__counts i { color: var(--success); font-style: normal; }
.file-list__counts b { color: var(--danger); font-weight: 400; }
.file-list__viewed { display: grid; width: 14px; height: 14px; place-items: center; border: 1px solid var(--border-strong); border-radius: 50%; }
.file-list__viewed:hover { border-color: var(--primary-border); }
.file-list__viewed--checked { color: var(--primary-foreground); background: var(--primary); border-color: var(--primary); }
.file-list__viewed svg { width: 9px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }

.changes__state { display: flex; min-height: 0; flex: 1; align-items: center; justify-content: center; flex-direction: column; gap: 7px; color: var(--text-muted); text-align: center; }
.changes__state strong { color: var(--text-primary); font-size: 13px; font-weight: 500; }
.changes__state span { max-width: 380px; font-size: 11px; line-height: 1.5; }
.changes__state button { margin-top: 5px; padding: 6px 10px; color: var(--primary); background: var(--primary-subtle); border: 1px solid var(--primary-border); border-radius: 6px; }
.changes__state--error strong { color: var(--danger); }
.changes__spinner { width: 17px; height: 17px; margin-bottom: 3px; border: 2px solid var(--border-strong); border-top-color: var(--primary); border-radius: 50%; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

@media (max-width: 820px) {
  .changes__workspace { grid-template-columns: 184px minmax(0, 1fr); }
  .changes__summary > span:not(.changes__additions):not(.changes__deletions) { display: none; }
  .code-search { width: 190px; margin-inline: 6px; }
}
</style>
