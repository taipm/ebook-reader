<script lang="ts">
  import type {
    Chapter,
    Block,
    Annotation,
    OpenDocumentResponse,
    PendingSelection,
  } from "$lib/types";
  import { untrack } from "svelte";
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
    onUpdateNote,
    onDelete,
  }: {
    doc: OpenDocumentResponse | null;
    currentChapter: Chapter | null;
    currentBlocks: Block[];
    annotations: Annotation[];
    status: "idle" | "opening" | "ready" | "error";
    pendingSelection: PendingSelection | null;
    highlightedAnnotationId: string | null;
    /** Tạo highlight từ pendingSelection, trả id annotation mới (undefined nếu không tạo). */
    onHighlight: () => string | undefined;
    onNote: () => void;
    onCancelSelection: () => void;
    onUpdateNote: (id: string, note: string) => void;
    onDelete: (id: string) => void;
  } = $props();

  /** Container `position: relative` — toolbar được neo vào đây nên cuộn theo nội dung. */
  let bodyEl = $state<HTMLDivElement | null>(null);

  // ── Selection capture ────────────────────────────────────────────────
  function handleMouseUp(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (!target?.closest?.(".reader-content")) return;
    pendingSelection = selectionFromDom();
  }

  /** Double-click vào từ → highlight ngay + mở ô ghi chú tại chỗ (không qua toolbar). */
  function handleDblClick(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (target?.closest(".hl")) return; // highlight có sẵn: click đã mở editor sửa
    const sel = selectionFromDom();
    if (!sel) return;
    pendingSelection = sel;
    const id = onHighlight();
    pendingSelection = null;
    window.getSelection()?.removeAllRanges();
    if (id) openGloss(id, sel.rect.x, (sel.rect_bottom ?? sel.rect.y) + 6, true);
  }

  /** Đọc selection hiện tại của trình duyệt thành PendingSelection (null nếu không hợp lệ). */
  function selectionFromDom(): PendingSelection | null {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || sel.rangeCount === 0) return null;

    const range = sel.getRangeAt(0);
    if (sel.toString().trim().length === 0) return null;

    const blockEl = (range.startContainer.parentElement?.closest(
      "[data-block-id]"
    ) || range.startContainer.parentElement) as HTMLElement | null;
    if (!blockEl || !bodyEl) return null;

    const blockId = blockEl.dataset.blockId;
    const chapterId = blockEl.dataset.chapterId;
    if (!blockId || !chapterId) return null;

    const block = currentBlocks.find((b) => b.id === blockId);
    if (!block) return null;

    const startOffset = computeCharOffset(blockEl, range.startContainer, range.startOffset);
    let endOffset = blockEl.contains(range.endContainer)
      ? computeCharOffset(blockEl, range.endContainer, range.endOffset)
      : -1;
    // Chọn vắt qua đoạn khác → cắt tại cuối đoạn đầu thay vì nuốt im lặng.
    if (endOffset < 0) endOffset = block.text.length;
    endOffset = Math.min(endOffset, block.text.length);

    if (startOffset < 0 || startOffset >= endOffset) return null;
    // Text lấy từ block.text theo offset — khớp 1:1 với char_start/char_end, không đọc DOM.
    const text = block.text.slice(startOffset, endOffset);
    if (text.trim().length === 0) return null;

    // Toạ độ tương đối với .reader-body (absolute), không phải viewport.
    const rect = range.getBoundingClientRect();
    const body = bodyEl.getBoundingClientRect();
    return {
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
    // Triple-click: boundary là Element + chỉ số con → quy về text node/ranh giới ký tự.
    if (node.nodeType === Node.ELEMENT_NODE) {
      const child = node.childNodes[offsetInNode];
      if (!child) return (node.textContent || "").length + elementStart(blockEl, node);
      node = child;
      offsetInNode = 0;
      if (node.nodeType === Node.ELEMENT_NODE) return elementStart(blockEl, node);
    }
    const walker = document.createTreeWalker(blockEl, NodeFilter.SHOW_TEXT);
    let charCount = 0;
    while (walker.nextNode()) {
      const cur = walker.currentNode;
      if (cur === node) return charCount + offsetInNode;
      charCount += (cur.textContent || "").length;
    }
    return -1;
  }

  /** Số ký tự trước `el` trong block (el là Element nằm trong blockEl, hoặc chính blockEl → 0). */
  function elementStart(blockEl: HTMLElement, el: Node): number {
    if (el === blockEl) return 0;
    const walker = document.createTreeWalker(blockEl, NodeFilter.SHOW_TEXT);
    let count = 0;
    while (walker.nextNode()) {
      const cur = walker.currentNode;
      if (el.contains(cur)) return count;
      count += (cur.textContent || "").length;
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
    if (e.key === "Escape" && editing) {
      closeGloss();
      return;
    }
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
      // Highlight chồng nhau: bắt đầu từ chỗ chưa render, bỏ phần đã bị phủ.
      const s = Math.max(hl.start, pos);
      if (hl.end <= s) continue;
      if (s > pos) {
        out.push({ kind: "text", content: text.slice(pos, s) });
      }
      out.push({
        kind: "hl",
        content: text.slice(s, hl.end),
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

  // ── Inline gloss editor (ghi nghĩa nhanh ngay tại highlight) ─────────
  const GLOSS_MAX = 40;
  function glossOf(note: string): string {
    return note.length > GLOSS_MAX ? note.slice(0, GLOSS_MAX) + "…" : note;
  }

  /** fresh = highlight vừa tạo bằng double-click: huỷ/Enter với note rỗng thì xoá luôn. */
  let editing = $state<{ id: string; x: number; y: number; fresh: boolean } | null>(null);
  let draft = $state("");

  // Đổi chương/sách → editor neo vào DOM cũ, đóng lại.
  $effect(() => {
    void currentChapter?.id;
    untrack(closeGloss);
  });

  function openGloss(id: string, x: number, y: number, fresh = false) {
    draft = annotations.find((a) => a.id === id)?.note ?? "";
    editing = { id, x, y, fresh };
    highlightedAnnotationId = id;
  }

  function handleHighlightClick(annotationId: string, el: HTMLElement) {
    if (!bodyEl) return;
    // Kéo chọn chữ kết thúc trên highlight cũng bắn click → không mở editor.
    if (!window.getSelection()?.isCollapsed) return;
    const r = el.getBoundingClientRect();
    const body = bodyEl.getBoundingClientRect();
    openGloss(annotationId, r.left + r.width / 2 - body.left, r.bottom - body.top + 6);
  }

  function commitGloss() {
    if (!editing) return;
    const note = draft.trim();
    if (note.length === 0 && editing.fresh) onDelete(editing.id);
    else onUpdateNote(editing.id, note);
    editing = null;
    draft = "";
    highlightedAnnotationId = null;
  }

  /** Huỷ: highlight vừa tạo bằng dblclick mà chưa có note → xoá, không để rác. */
  function closeGloss() {
    const e = editing;
    if (e?.fresh && annotations.find((a) => a.id === e.id)?.note === "") {
      onDelete(e.id);
    }
    editing = null;
    draft = "";
    highlightedAnnotationId = null;
  }

  function onGlossKey(e: KeyboardEvent) {
    if (e.key === "Enter") commitGloss();
    else if (e.key === "Escape") closeGloss();
  }

  /** Click ngoài editor (và không phải lên highlight vừa mở) → huỷ. */
  function onDocClick(e: MouseEvent) {
    if (!editing) return;
    const t = e.target as HTMLElement | null;
    if (t?.closest(".gloss-editor") || t?.closest(".hl")) return;
    closeGloss();
  }

  const autofocus = (el: HTMLInputElement) => {
    el.focus();
    el.select();
  };

  function onHighlightKey(e: KeyboardEvent, annotationId: string) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleHighlightClick(annotationId, e.currentTarget as HTMLElement);
    }
  }
</script>

<svelte:document onselectionchange={onSelectionChange} onmouseup={handleMouseUp} onclick={onDocClick} />
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
    <div class="reader-content" ondblclick={handleDblClick} role="presentation">
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
                data-note={seg.note ? glossOf(seg.note) : undefined}
                title={seg.note || undefined}
                role="button"
                tabindex="0"
                onclick={(e) => seg.id && handleHighlightClick(seg.id, e.currentTarget)}
                onkeydown={(e) => seg.id && onHighlightKey(e, seg.id)}
              >{seg.content}</span>
            {:else}
              {seg.content}
            {/if}
          {/each}
        </p>
      {/each}
    </div>

    {#if editing}
      <div class="gloss-editor" style="left: {editing.x}px; top: {editing.y}px">
        <input
          type="text"
          placeholder="Add a note…"
          aria-label="Note for highlight"
          bind:value={draft}
          onkeydown={onGlossKey}
          use:autofocus
        />
      </div>
    {/if}

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
  /* Gloss qua ::after: không sinh text node → computeCharOffset/selection không bị lệch */
  .hl.note-hl::after {
    content: " ⟨" attr(data-note) "⟩";
    font-family: var(--font-ui);
    font-size: var(--text-xs);
    color: var(--fg-muted);
    background: var(--bg-reader);
    padding-inline: 0.15em;
  }
  .gloss-editor {
    position: absolute;
    transform: translateX(-50%);
    z-index: 100;
    padding: var(--space-1);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
  }
  .gloss-editor input {
    width: 16rem;
    max-width: 70vw;
    font: inherit;
    font-family: var(--font-ui);
    font-size: var(--text-sm);
    color: var(--fg);
    background: transparent;
    border: 0;
    padding: var(--space-1) var(--space-2);
    outline: none; /* ring vẽ trên wrapper qua :focus-within */
  }
  .gloss-editor:focus-within {
    box-shadow: var(--shadow-md), 0 0 0 2px var(--accent-ring);
  }
  .hl.flash {
    box-shadow: 0 0 0 2px var(--accent-ring);
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
        box-shadow: 0 0 0 2px transparent;
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
