<script lang="ts">
  import type { OpenDocumentResponse, FocusMode, BundledBook } from "$lib/types";
  import LibraryPopover from "./LibraryPopover.svelte";
  import { SHORTCUTS, MODE_LABELS } from "$lib/stores/shortcuts";

  let {
    doc,
    status,
    focusMode,
    library,
    libraryLoading,
    libraryOpen = $bindable(),
    currentPath,
    onOpen,
    onSetFocusMode,
    onLoadBook,
  }: {
    doc: OpenDocumentResponse | null;
    status: "idle" | "opening" | "ready" | "error";
    focusMode: FocusMode;
    library: BundledBook[];
    libraryLoading: boolean;
    libraryOpen: boolean;
    currentPath: string | null;
    onOpen: () => void;
    onSetFocusMode: (mode: FocusMode) => void;
    onLoadBook: (book: BundledBook) => void;
  } = $props();

  let trigger: HTMLButtonElement | undefined = $state();

  // Esc anywhere inside the wrapper closes the popover and returns focus to the trigger.
  function onLibraryKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && libraryOpen) {
      e.stopPropagation();
      libraryOpen = false;
      trigger?.focus();
    }
  }
</script>

<header class="app-header">
  <div class="brand">
    <span class="title" title={doc?.meta.title}>{doc ? doc.meta.title : "ebook-reader"}</span>
    {#if doc}
      <span class="format-badge">{doc.meta.format.toUpperCase()}</span>
    {/if}
  </div>
  <div class="header-actions">
    <button class="ghost" onclick={onOpen} disabled={status === "opening"}>
      {status === "opening" ? "Opening…" : "📂 Open"}
    </button>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="library-wrapper" onkeydown={onLibraryKeydown}>
      <button
        bind:this={trigger}
        class="ghost library-trigger"
        class:active={libraryOpen}
        aria-haspopup="menu"
        aria-expanded={libraryOpen}
        onclick={(e) => {
          e.stopPropagation();
          libraryOpen = !libraryOpen;
        }}
        title="Browse bundled books"
      >
        Library <span class="count">{library.length}</span>
      </button>
      {#if libraryOpen}
        <LibraryPopover
          {library}
          {libraryLoading}
          {currentPath}
          onOpenExternal={() => {
            libraryOpen = false;
            onOpen();
          }}
          {onLoadBook}
        />
      {/if}
    </div>
    <div class="focus-toggles" role="group" aria-label="Layout">
      {#each Object.entries(SHORTCUTS) as [key, mode] (mode)}
        <button
          class="focus-btn"
          class:active={focusMode === mode}
          aria-pressed={focusMode === mode}
          aria-keyshortcuts="Meta+{key}"
          onclick={() => onSetFocusMode(mode)}
          title="{MODE_LABELS[mode]} (⌘{key})"
        >{MODE_LABELS[mode]}</button>
      {/each}
    </div>
  </div>
</header>

<style>
  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-4);
    min-height: var(--header-h);
    padding: var(--space-1) var(--space-4);
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    font-size: var(--text-sm);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .title {
    font-weight: var(--weight-semibold);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .format-badge {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }
  button {
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ghost {
    background: transparent;
    color: var(--fg);
    border: 1px solid var(--border);
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-md);
    transition: background var(--ease), border-color var(--ease);
  }
  .ghost:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--border-strong);
  }
  .ghost.active {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .count {
    color: var(--fg-muted);
    font-size: var(--text-xs);
  }
  .library-wrapper {
    position: relative;
  }
  .focus-toggles {
    display: flex;
    gap: 2px;
    background: var(--bg-active);
    padding: 2px;
    border-radius: var(--radius-md);
  }
  .focus-btn {
    background: transparent;
    color: var(--fg-muted);
    border: 1px solid transparent;
    padding: var(--space-1) var(--space-3);
    border-radius: calc(var(--radius-md) - 2px);
    transition: background var(--ease), color var(--ease);
  }
  .focus-btn:hover {
    color: var(--fg);
  }
  .focus-btn.active {
    background: var(--bg-raised);
    border-color: var(--border);
    color: var(--accent);
    font-weight: var(--weight-medium);
    box-shadow: var(--shadow-sm);
  }
  @media (max-width: 720px) {
    .format-badge {
      display: none;
    }
  }
</style>
