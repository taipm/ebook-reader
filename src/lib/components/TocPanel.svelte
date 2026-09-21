<script lang="ts">
  import type { Chapter, ChapterId } from "$lib/types";

  let {
    chapters,
    currentChapterId,
    annotationCount,
    totalChars,
    onSelect,
  }: {
    chapters: Chapter[];
    currentChapterId: ChapterId | null;
    annotationCount: number;
    totalChars: number;
    onSelect: (chapter: Chapter) => void;
  } = $props();

  let list: HTMLOListElement | undefined = $state();

  const currentIndex = $derived(chapters.findIndex((c) => c.id === currentChapterId));
  const prev = $derived(currentIndex > 0 ? chapters[currentIndex - 1] : null);
  const next = $derived(
    currentIndex >= 0 && currentIndex < chapters.length - 1 ? chapters[currentIndex + 1] : null
  );

  function formatChars(n: number): string {
    if (n < 1000) return `${n}`;
    if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`;
    return `${(n / 1_000_000).toFixed(1)}M`;
  }

  // Keep the current chapter in view when it changes (e.g. via prev/next or search).
  $effect(() => {
    if (!currentChapterId) return;
    list?.querySelector('[aria-current="page"]')?.scrollIntoView({ block: "nearest" });
  });
</script>

<h3 class="col-title">Contents</h3>
{#if chapters.length > 0}
  <ol class="toc" bind:this={list}>
    {#each chapters as ch (ch.id)}
      <li class="toc-item" style="--depth: {ch.level - 1}">
        <button
          class="toc-link"
          aria-current={currentChapterId === ch.id ? "page" : undefined}
          title={ch.title}
          onclick={() => onSelect(ch)}
        >{ch.title}</button>
      </li>
    {/each}
  </ol>
  <nav class="chapter-nav" aria-label="Chapter navigation">
    <button class="nav-btn" disabled={!prev} aria-label="Previous chapter" title={prev?.title} onclick={() => prev && onSelect(prev)}>
      ← Previous
    </button>
    <span class="nav-pos">{currentIndex >= 0 ? currentIndex + 1 : "–"} / {chapters.length}</span>
    <button class="nav-btn" disabled={!next} aria-label="Next chapter" title={next?.title} onclick={() => next && onSelect(next)}>
      Next →
    </button>
  </nav>
  <div class="col-footer">
    <div>{chapters.length} chapters · {formatChars(totalChars)} chars</div>
    <div>{annotationCount} annotations</div>
  </div>
{/if}

<style>
  button {
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .col-title {
    margin: 0 0 var(--space-2);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    color: var(--fg-muted);
  }
  .toc {
    list-style: none;
    padding: 0;
    margin: 0;
    font-size: var(--text-sm);
  }
  .toc-link {
    width: 100%;
    display: block;
    background: transparent;
    color: var(--fg);
    border: 0;
    border-left: 2px solid transparent;
    padding: var(--space-1) var(--space-2);
    padding-left: calc(var(--space-2) + var(--depth, 0) * var(--space-3));
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: background var(--ease);
  }
  .toc-link:hover {
    background: var(--bg-hover);
  }
  .toc-link[aria-current="page"] {
    background: var(--accent-soft);
    border-left-color: var(--accent);
    color: var(--accent);
    font-weight: var(--weight-medium);
  }
  .chapter-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-top: var(--space-3);
    font-size: var(--text-xs);
  }
  .nav-btn {
    background: transparent;
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-1) var(--space-2);
    font-size: var(--text-xs);
    transition: background var(--ease);
  }
  .nav-btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  .nav-pos {
    color: var(--fg-muted);
    font-family: var(--font-mono);
  }
  .col-footer {
    margin-top: var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
    font-size: var(--text-xs);
    color: var(--fg-muted);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
</style>
