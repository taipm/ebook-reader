<script lang="ts">
  import type { BundledBook } from "$lib/types";

  let {
    library,
    libraryLoading,
    currentPath,
    onOpenExternal,
    onLoadBook,
  }: {
    library: BundledBook[];
    libraryLoading: boolean;
    currentPath: string | null;
    onOpenExternal: () => void;
    onLoadBook: (book: BundledBook) => void;
  } = $props();

  let root: HTMLDivElement | undefined = $state();

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  // ↑/↓ move between items; Home/End jump. Esc is handled by the parent wrapper.
  function onKeydown(e: KeyboardEvent) {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
    const items = Array.from(root?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? []);
    if (items.length === 0) return;
    e.preventDefault();
    const i = items.indexOf(document.activeElement as HTMLElement);
    const next =
      e.key === "Home" ? 0
      : e.key === "End" ? items.length - 1
      : e.key === "ArrowDown" ? (i + 1) % items.length
      : (i - 1 + items.length) % items.length;
    items[next].focus();
  }

  // Focus the current book (or first item) when the popover opens.
  $effect(() => {
    const el =
      root?.querySelector<HTMLElement>('[role="menuitem"][aria-current]') ??
      root?.querySelector<HTMLElement>('[role="menuitem"]');
    el?.focus();
  });
</script>

<div class="library-popover" role="menu" tabindex="-1" aria-label="Library" bind:this={root} onkeydown={onKeydown}>
  {#if libraryLoading}
    <div class="library-empty">Loading…</div>
  {:else if library.length === 0}
    <div class="library-empty">
      <p>No bundled books yet.</p>
      <button class="library-action" role="menuitem" onclick={onOpenExternal}>Open a file…</button>
    </div>
  {:else}
    <div class="library-header">
      <span>Bundled books</span>
      <button class="library-action" role="menuitem" onclick={onOpenExternal}>Open a file…</button>
    </div>
    <ul class="library-list" role="none">
      {#each library as book (book.id)}
        <li role="none">
          <button
            class="library-item"
            role="menuitem"
            aria-current={book.path === currentPath ? "true" : undefined}
            onclick={() => onLoadBook(book)}
          >
            <span class="library-format">{book.format === "epub" ? "EPUB" : book.format === "markdown" ? "MD" : "TXT"}</span>
            <span class="library-meta">
              <span class="library-title">{book.title}</span>
              {#if book.author}
                <span class="library-author">{book.author}</span>
              {/if}
            </span>
            <span class="library-size">{formatSize(book.size_bytes)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  button {
    cursor: pointer;
  }
  .library-popover {
    position: absolute;
    top: calc(100% + var(--space-2));
    right: 0;
    width: min(360px, calc(100vw - 2 * var(--space-4)));
    max-height: 480px;
    overflow-y: auto;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    z-index: 200;
    padding: var(--space-2);
    font-size: var(--text-sm);
  }
  .library-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-1) var(--space-3) var(--space-2);
    border-bottom: 1px solid var(--border);
    margin-bottom: var(--space-2);
    color: var(--fg-muted);
    font-size: var(--text-xs);
  }
  .library-action {
    background: transparent;
    color: var(--accent);
    border: 0;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
  }
  .library-action:hover {
    background: var(--bg-hover);
    color: var(--accent-hover);
  }
  .library-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .library-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    color: var(--fg);
    border: 0;
    border-radius: var(--radius-md);
    text-align: left;
    transition: background var(--ease);
  }
  .library-item:hover {
    background: var(--bg-hover);
  }
  .library-item[aria-current] {
    background: var(--accent-soft);
  }
  .library-item[aria-current] .library-title {
    color: var(--accent);
  }
  .library-format {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--fg-muted);
    min-width: 2.75em;
    flex-shrink: 0;
  }
  .library-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .library-title {
    font-weight: var(--weight-medium);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .library-author {
    font-size: var(--text-xs);
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .library-size {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .library-empty {
    padding: var(--space-5) var(--space-4);
    text-align: center;
    color: var(--fg-muted);
  }
  .library-empty p {
    margin: 0 0 var(--space-2);
  }
</style>
