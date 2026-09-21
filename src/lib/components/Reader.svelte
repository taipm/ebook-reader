<script lang="ts">
  import type {
    Chapter,
    Block,
    Annotation,
    OpenDocumentResponse,
    PendingSelection,
  } from "$lib/types";
  import SelectionToolbar from "./SelectionToolbar.svelte";

  let {
    doc,
    currentChapter,
    currentBlocks,
    annotations,
    status,
    pendingSelection = $bindable(),
    highlightedAnnotationId = $bindable(),
    onHighlight,
    onNote,
    onCancelSelection,
  }: {
    doc: OpenDocumentResponse | null;
    currentChapter: Chapter | null;
    currentBlocks: Block[];
    annotations: Annotation[];
    status: "idle" | "opening" | "ready" | "error";
    pendingSelection: PendingSelection | null;
    highlightedAnnotationId: string | null;
    onHighlight: () => void;
    onNote: () => void;
    onCancelSelection: () => void;
  } = $props();

  /** Container `position: relative` — toolbar được neo vào đây nên cuộn theo nội dung. */
  let bodyEl = $state<HTMLDivElement | null>(null);

  // ── Selection capture ────────────────────────────────────────────────
  function handleMouseUp(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (!target?.closest?.(".reader-content")) return;

    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || sel.rangeCount === 0) {
      pendingSelection = null;
      return;
    }

    const range = sel.getRangeAt(0);
    const text = sel.toString().trim();
    if (text.length === 0) {
      pendingSelection = null;
      return;
    }

    const blockEl = (range.startContainer.parentElement?.closest(
      "[data-block-id]"
    ) || range.startContainer.parentElement) as HTMLElement | null;
    if (!blockEl || !bodyEl) return;

    const blockId = blockEl.dataset.blockId;
    const chapterId = blockEl.dataset.chapterId;
    if (!blockId || !chapterId) return;

    const startOffset = computeCharOffset(blockEl, range.startContainer, range.startOffset);
    let endOffset = computeCharOffset(blockEl, range.endContainer, range.endOffset);
    // Chọn vắt qua đoạn khác → cắt tại cuối đoạn đầu thay vì nuốt im lặng.
    if (endOffset < 0) endOffset = (blockEl.textContent || "").length;

    if (startOffset < 0 || startOffset >= endOffset) return;

    // Toạ độ tương đối với .reader-body (absolute), không phải viewport.
    const rect = range.getBoundingClientRect();
    const body = bodyEl.getBoundingClientRect();
    pendingSelection = {
      block_id: blockId,
      chapter_id: chapterId,
      char_start: startOffset,
      char_end: endOffset,
      text,
      rect: { x: rect.left + rect.width / 2 - body.left, y: rect.top - body.top },
      rect_bottom: rect.bottom - body.top,
    };
  }

  function computeCharOffset(
    blockEl: HTMLElement,
    node: Node,
    offsetInNode: number
  ): number {
    const walker = document.createTreeWalker(blockEl, NodeFilter.SHOW_TEXT);
    let charCount = 0;
    while (walker.nextNode()) {
      const cur = walker.currentNode;
      if (cur === node) return charCount + offsetInNode;
      charCount += (cur.textContent || "").length;
    }
    return -1;
  }

  /** Ẩn toolbar khi selection biến mất (click chỗ khác, Escape…). */
  function onSelectionChange() {
    if (!pendingSelection) return;
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) onCancelSelection();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape" && pendingSelection) {
      window.getSelection()?.removeAllRanges();
      onCancelSelection();
    }
  }

  // ── Highlight rendering ──────────────────────────────────────────────
  type Hl = { start: number; end: number; note: string; id: string; color: string };
  type Segment = { kind: "text" | "hl"; content: string; note?: string; id?: string; color?: string };

  const MARKERS = ["yellow", "green", "blue", "rose"] as const;
  /** Annotation.color là tên marker của DESIGN.md; giá trị lạ (hex cũ) → yellow. */
  function markerOf(color: string): string {
    return (MARKERS as readonly string[]).includes(color) ? color : "yellow";
  }

  function renderBlockText(block: Block) {
    const highlights: Hl[] = [];
    for (const ann of annotations) {
      if (ann.location.block_id === block.id) {
        highlights.push({
          start: ann.location.char_start,
          end: ann.location.char_end,
          note: ann.note,
          id: ann.id,
          color: markerOf(ann.color),
        });
      }
    }
    highlights.sort((a, b) => a.start - b.start);
    return { text: block.text, highlights };
  }

  function splitTextWithHighlights(text: string, highlights: Hl[]): Segment[] {
    if (highlights.length === 0) return [{ kind: "text", content: text }];
    const out: Segment[] = [];
    let pos = 0;
    for (const hl of highlights) {
      if (hl.start > pos) {
        out.push({ kind: "text", content: text.slice(pos, hl.start) });
      }
      out.push({
        kind: "hl",
        content: text.slice(hl.start, hl.end),
        note: hl.note,
        id: hl.id,
        color: hl.color,
      });
      pos = hl.end;
    }
    if (pos < text.length) {
      out.push({ kind: "text", content: text.slice(pos) });
    }
    return out;
  }

  function handleHighlightClick(annotationId: string) {
    highlightedAnnotationId = annotationId;
    setTimeout(() => (highlightedAnnotationId = null), 1500);
  }

  function onHighlightKey(e: KeyboardEvent, annotationId: string) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleHighlightClick(annotationId);
    }
  }
</script>

<svelte:document onselectionchange={onSelectionChange} onmouseup={handleMouseUp} />
<svelte:window onkeydown={onKeyDown} />

{#if status === "opening"}
  <div class="reader-column skeleton" aria-busy="true" aria-label="Opening book">
    <div class="sk-title"></div>
    {#each [92, 100, 97, 88, 100, 95, 60] as w}
      <div class="sk-line" style="width: {w}%"></div>
    {/each}
  </div>
{:else if !doc}
  <div class="empty-state">
    <p>Click Open to start reading an EPUB or Markdown file.</p>
    <p class="hint">Select any passage to highlight it or add a note.</p>
  </div>
{:else if currentChapter}
  <div class="reader-column">
  <h2 class="reader-title">{currentChapter.title}</h2>
  <div class="reader-body" bind:this={bodyEl}>
    <div class="reader-content">
      {#each currentBlocks as block (block.id)}
        {@const rendered = renderBlockText(block)}
        {@const segments = splitTextWithHighlights(rendered.text, rendered.highlights)}
        <p class="block" data-block-id={block.id} data-chapter-id={block.chapter_id}>
          {#each segments as seg}
            {#if seg.kind === "hl"}
              <!-- span (không phải <mark>/<button>): mark không được nhận role=button, button chặn kéo-chọn chữ trong WebKit -->
              <span
                class="hl"
                class:note-hl={seg.note && seg.note.length > 0}
                class:flash={seg.id === highlightedAnnotationId}
                style="background: var(--hl-{seg.color})"
                data-annotation-id={seg.id}
                role="button"
                tabindex="0"
                onclick={() => seg.id && handleHighlightClick(seg.id)}
                onkeydown={(e) => seg.id && onHighlightKey(e, seg.id)}
              >{seg.content}</span>
            {:else}
              {seg.content}
            {/if}
          {/each}
        </p>
      {/each}
    </div>

    {#if pendingSelection}
      <SelectionToolbar
        {pendingSelection}
        {onHighlight}
        {onNote}
        onCancel={onCancelSelection}
      />
    {/if}
  </div>
  </div>
{/if}

<style>
  /* ch của --reading-measure tính theo font-size wrapper, không theo h2 */
  .reader-column {
    max-width: var(--reading-measure);
    margin-inline: auto;
    font-size: var(--reading-size);
  }
  .reader-body {
    position: relative;
  }
  .reader-title {
    margin: 0 0 var(--space-5);
    font-family: var(--font-reading);
    font-size: var(--text-xl);
    font-weight: var(--weight-semibold);
    line-height: var(--leading-tight);
    color: var(--fg);
  }
  .reader-content {
    font-family: var(--font-reading);
    font-size: var(--reading-size);
    line-height: var(--reading-leading);
    color: var(--fg);
    hyphens: auto;
    text-wrap: pretty;
  }
  .block {
    margin: 0;
  }
  .block + .block {
    margin-top: var(--reading-para-gap);
  }
  .hl {
    color: inherit;
    padding: 0.05em 0.1em;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .hl:hover {
    box-shadow: 0 0 0 2px var(--bg-active);
  }
  .hl.note-hl {
    text-decoration: underline dotted var(--accent);
    text-decoration-thickness: 2px;
    text-underline-offset: 0.2em;
  }
  .hl.flash {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  @media (prefers-reduced-motion: no-preference) {
    .hl {
      transition: box-shadow var(--ease);
    }
    .hl.flash {
      animation: pulse 0.6s 2;
    }
    @keyframes pulse {
      50% {
        outline-color: transparent;
      }
    }
  }
  .empty-state {
    text-align: center;
    padding: var(--space-8) var(--space-6);
    color: var(--fg-muted);
    font-family: var(--font-ui);
  }
  .empty-state .hint {
    font-size: var(--text-sm);
    color: var(--fg-faint);
    margin-top: var(--space-2);
  }
  .skeleton {
    padding-top: var(--space-2);
  }
  .sk-title,
  .sk-line {
    background: var(--bg-active);
    border-radius: var(--radius-sm);
  }
  .sk-title {
    height: var(--text-xl);
    width: 45%;
    margin-bottom: var(--space-5);
  }
  .sk-line {
    height: calc(var(--reading-size) * 0.7);
    margin-bottom: calc(var(--reading-size) * 0.95);
  }
</style>
