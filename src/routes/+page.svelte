<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import type {
    Chapter,
    Block,
    Annotation,
    OpenDocumentResponse,
    PendingSelection,
    FocusMode,
    BundledBook,
  } from "$lib/types";
  import AppHeader from "$lib/components/AppHeader.svelte";
  import TocPanel from "$lib/components/TocPanel.svelte";
  import Reader from "$lib/components/Reader.svelte";
  import NotesPanel from "$lib/components/NotesPanel.svelte";
  import { SHORTCUTS } from "$lib/stores/shortcuts";
  import { slide } from "svelte/transition";
  import { onMount, tick } from "svelte";

  // ── App state ────────────────────────────────────────────────────────
  let doc = $state<OpenDocumentResponse | null>(null);
  let currentChapter = $state<Chapter | null>(null);
  let currentBlocks = $state<Block[]>([]);
  let annotations = $state<Annotation[]>([]);
  let status = $state<"idle" | "opening" | "ready" | "error">("idle");
  let errorMsg = $state(""); // technical detail, shown under <details>
  let errorTitle = $state(""); // what happened, in the user's words
  let currentPath = $state<string | null>(null);
  let failedPath = $state<string | null>(null); // for Retry in the banner

  const reducedMotion =
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const colSlide = { axis: "x" as const, duration: reducedMotion ? 0 : 160 };

  function basename(path: string): string {
    return path.split(/[\\/]/).pop() || path;
  }

  // UI state
  let focusMode = $state<FocusMode>("normal");
  let lastMode = $state<FocusMode>("normal"); // for ⌘0 toggle reader ↔ last
  let columnWidths = $state({ contents: 20, notes: 20 });
  let pendingSelection = $state<PendingSelection | null>(null);
  let noteDraft = $state("");
  let editingAnnotationId = $state<string | null>(null);
  let draftIsNew = $state(false); // editor opened for a just-created note → Cancel discards it
  let highlightedAnnotationId = $state<string | null>(null);

  // Library state
  let library = $state<BundledBook[]>([]);
  let libraryOpen = $state(false);
  let libraryLoading = $state(false);

  // Load library once on mount (not an $effect: an empty/failed result would
  // flip libraryLoading and re-trigger it forever).
  function refreshLibrary() {
    if (libraryLoading) return;
    libraryLoading = true;
    invoke<BundledBook[]>("list_bundled_books")
      .then((books) => {
        library = books;
      })
      .catch((e) => {
        console.error("Failed to load library:", e);
      })
      .finally(() => {
        libraryLoading = false;
      });
  }
  onMount(refreshLibrary);

  // Click outside to close popover
  function handleWindowClick(event: MouseEvent) {
    if (!libraryOpen) return;
    const target = event.target as HTMLElement;
    if (!target.closest(".library-popover") && !target.closest(".library-trigger")) {
      libraryOpen = false;
    }
  }

  // ── File open ────────────────────────────────────────────────────────
  async function handleOpen() {
    status = "opening";
    errorMsg = "";
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: "ebook / markdown", extensions: ["epub", "md", "markdown"] },
        ],
      });
      if (!selected) {
        status = doc ? "ready" : "idle";
        return;
      }
      await openPath(selected as string);
    } catch (e) {
      fail("Couldn't open the file picker", e);
    }
  }

  // Request tokens: a stale response (open two books fast, mash `]`) must not
  // overwrite the newer one.
  let openSeq = 0;
  let chapterSeq = 0;

  function resetSession() {
    pendingSelection = null;
    editingAnnotationId = null;
    noteDraft = "";
    highlightedAnnotationId = null;
    draftIsNew = false;
  }

  async function openPath(path: string) {
    const seq = ++openSeq;
    status = "opening";
    errorMsg = "";
    try {
      const result = await invoke<OpenDocumentResponse>("open_document", { path });
      if (seq !== openSeq) return;
      doc = result;
      currentPath = path;
      failedPath = null;
      status = "ready";
      annotations = [];
      resetSession();
      const first = result.chapters[0];
      if (first) await selectChapter(first);
    } catch (e) {
      if (seq !== openSeq) return;
      failedPath = path;
      fail(`Couldn't open "${basename(path)}"`, e);
    }
  }

  function fail(title: string, e: unknown) {
    status = "error";
    errorTitle = title;
    errorMsg = String(e);
  }

  async function selectChapter(chapter: Chapter) {
    if (!doc) return;
    const seq = ++chapterSeq;
    const docId = doc.meta.id;
    pendingSelection = null;
    highlightedAnnotationId = null;
    try {
      const blocks = await invoke<Block[]>("get_chapter_blocks", {
        chapterId: chapter.id,
      });
      if (seq !== chapterSeq || doc?.meta.id !== docId) return;
      currentChapter = chapter;
      currentBlocks = blocks;
    } catch (e) {
      if (seq !== chapterSeq || doc?.meta.id !== docId) return;
      fail(`Couldn't load chapter "${chapter.title}"`, e);
    }
  }

  function dismissError() {
    status = doc ? "ready" : "idle";
    errorMsg = "";
    errorTitle = "";
  }

  function retryError() {
    if (failedPath) openPath(failedPath);
    else dismissError();
  }

  async function loadBundledBook(book: BundledBook) {
    libraryOpen = false;
    await openPath(book.path);
  }

  // ── Chapter prev/next ────────────────────────────────────────────────
  const chapterIndex = $derived(
    doc && currentChapter ? doc.chapters.findIndex((c) => c.id === currentChapter!.id) : -1
  );
  const prevChapter = $derived(chapterIndex > 0 ? doc!.chapters[chapterIndex - 1] : null);
  const nextChapter = $derived(
    doc && chapterIndex >= 0 && chapterIndex < doc.chapters.length - 1
      ? doc.chapters[chapterIndex + 1]
      : null
  );

  // ── Add annotation ───────────────────────────────────────────────────
  function addHighlight(): string | undefined {
    if (!pendingSelection || !doc || !currentChapter) return undefined;
    const ann: Annotation = {
      id: crypto.randomUUID(),
      document_id: doc.meta.id,
      location: {
        document_id: doc.meta.id,
        chapter_id: currentChapter.id,
        block_id: pendingSelection.block_id,
        char_start: pendingSelection.char_start,
        char_end: pendingSelection.char_end,
      },
      kind: "highlight",
      highlighted_text: pendingSelection.text,
      note: "",
      tags: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      color: "yellow",
    };
    annotations = [...annotations, ann];
    pendingSelection = null;
    window.getSelection()?.removeAllRanges();
    return ann.id;
  }

  function addNoteFromSelection() {
    if (!pendingSelection || !doc || !currentChapter) return;
    const ann: Annotation = {
      id: crypto.randomUUID(),
      document_id: doc.meta.id,
      location: {
        document_id: doc.meta.id,
        chapter_id: currentChapter.id,
        block_id: pendingSelection.block_id,
        char_start: pendingSelection.char_start,
        char_end: pendingSelection.char_end,
      },
      kind: "note",
      highlighted_text: pendingSelection.text,
      note: "",
      tags: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      color: "rose",
    };
    annotations = [...annotations, ann];
    if (columnWidths.notes === 0) setFocusMode("normal");
    editingAnnotationId = ann.id;
    draftIsNew = true;
    noteDraft = "";
    pendingSelection = null;
  }

  function saveNote() {
    if (!editingAnnotationId) return;
    annotations = annotations.map((a) =>
      a.id === editingAnnotationId
        ? { ...a, note: noteDraft, updated_at: new Date().toISOString() }
        : a
    );
    editingAnnotationId = null;
    draftIsNew = false;
    noteDraft = "";
  }

  function cancelNote() {
    if (editingAnnotationId && draftIsNew) {
      annotations = annotations.filter((a) => a.id !== editingAnnotationId);
    }
    editingAnnotationId = null;
    draftIsNew = false;
    noteDraft = "";
  }

  /** Sửa note tại chỗ từ Reader (gloss); note rỗng → giữ highlight, note = "". */
  function updateNote(id: string, note: string) {
    annotations = annotations.map((a) =>
      a.id === id ? { ...a, note, updated_at: new Date().toISOString() } : a
    );
  }

  function deleteAnnotation(id: string) {
    annotations = annotations.filter((a) => a.id !== id);
    if (editingAnnotationId === id) {
      editingAnnotationId = null;
      draftIsNew = false;
      noteDraft = "";
    }
  }

  let flashTimer: ReturnType<typeof setTimeout> | null = null;

  async function scrollToAnnotation(ann: Annotation) {
    if (currentChapter?.id !== ann.location.chapter_id) {
      const chap = doc?.chapters.find((c) => c.id === ann.location.chapter_id);
      if (!chap) return;
      await selectChapter(chap);
      if (status === "error") return;
      await tick();
    }
    const el = document.querySelector(
      `[data-block-id="${ann.location.block_id}"]`
    ) as HTMLElement | null;
    if (!el) return;
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    if (flashTimer) clearTimeout(flashTimer);
    highlightedAnnotationId = ann.id;
    flashTimer = setTimeout(() => (highlightedAnnotationId = null), 1500);
  }

  // ── Focus modes ──────────────────────────────────────────────────────
  const widthsForMode = (mode: FocusMode) => {
    switch (mode) {
      case "normal":
        return { contents: 20, notes: 20 };
      case "reader":
        return { contents: 0, notes: 0 };
      case "research":
        return { contents: 25, notes: 25 };
      case "notes":
        return { contents: 0, notes: 40 };
    }
  };

  function setFocusMode(mode: FocusMode) {
    if (focusMode !== "reader") lastMode = focusMode;
    focusMode = mode;
    columnWidths = widthsForMode(mode);
  }

  // ── Resize handle ────────────────────────────────────────────────────
  type Side = "contents" | "notes";
  let dragging = $state<Side | null>(null);
  let innerWidth = $state(1280);
  const panelMaxPx = $derived(Math.min(480, Math.floor(innerWidth * 0.4)));

  // Clamp in px so a narrow window can't squash a panel below --panel-w-min.
  function clampPct(px: number): number {
    const total = window.innerWidth;
    const minPx =
      parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--panel-w-min")) || 180;
    const lo = Math.max(minPx, total * 0.1);
    const hi = Math.min(480, total * 0.4);
    return (Math.max(lo, Math.min(hi, px)) / total) * 100;
  }

  function setWidth(side: Side, px: number) {
    columnWidths[side] = clampPct(px);
  }

  function widthPx(side: Side): number {
    return Math.round((columnWidths[side] / 100) * window.innerWidth);
  }

  function onResizeMouseDown(side: Side, e: MouseEvent) {
    e.preventDefault();
    dragging = side;
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;
    const x = e.clientX;
    setWidth(dragging, dragging === "contents" ? x : window.innerWidth - x);
  }

  function onMouseUp() {
    dragging = null;
  }

  function onResizeKey(side: Side, e: KeyboardEvent) {
    const grow = side === "contents" ? "ArrowRight" : "ArrowLeft";
    const shrink = side === "contents" ? "ArrowLeft" : "ArrowRight";
    if (e.key !== grow && e.key !== shrink) return;
    e.preventDefault();
    setWidth(side, widthPx(side) + (e.key === grow ? 16 : -16));
  }

  function resetWidth(side: Side) {
    columnWidths[side] = widthsForMode(focusMode)[side];
  }

  $effect(() => {
    if (dragging) {
      window.addEventListener("mousemove", onMouseMove);
      window.addEventListener("mouseup", onMouseUp);
      return () => {
        window.removeEventListener("mousemove", onMouseMove);
        window.removeEventListener("mouseup", onMouseUp);
      };
    }
  });

  // ── Keyboard shortcuts ───────────────────────────────────────────────
  function isTyping(e: KeyboardEvent): boolean {
    const t = e.target as HTMLElement | null;
    return !!t?.closest("input, textarea, select, [contenteditable]");
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape" && status === "error") {
      dismissError();
      return;
    }
    if (isTyping(e)) return;
    if (!(e.metaKey || e.ctrlKey || e.altKey)) {
      // [ / ] step through chapters
      if (e.key === "[" && prevChapter) selectChapter(prevChapter);
      else if (e.key === "]" && nextChapter) selectChapter(nextChapter);
      return;
    }
    if (e.altKey && !e.metaKey && !e.ctrlKey) return;
    if (e.key === "0") {
      e.preventDefault();
      setFocusMode(focusMode === "reader" ? lastMode : "reader");
      return;
    }
    const mode = SHORTCUTS[e.key];
    if (mode) {
      e.preventDefault();
      setFocusMode(mode);
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} onclick={handleWindowClick} bind:innerWidth />

<main class="app" data-mode={focusMode} class:dragging>
  <AppHeader
    {doc}
    {status}
    {focusMode}
    {library}
    {libraryLoading}
    bind:libraryOpen
    {currentPath}
    onOpen={handleOpen}
    onSetFocusMode={setFocusMode}
    onLoadBook={loadBundledBook}
  />

  {#if status === "error"}
    <div class="banner error" role="alert">
      <div class="banner-text">
        <strong>{errorTitle || "Something went wrong"}</strong>
        {#if errorMsg}
          <details>
            <summary>Details</summary>
            <pre>{errorMsg}</pre>
          </details>
        {/if}
      </div>
      <div class="banner-actions">
        {#if failedPath}
          <button class="banner-btn" onclick={retryError}>Retry</button>
        {/if}
        <button class="banner-btn" onclick={dismissError} aria-label="Dismiss error">Dismiss</button>
      </div>
    </div>
  {/if}

  <div class="three-col">
    {#if columnWidths.contents > 0}
      <aside class="col contents" style="width: {columnWidths.contents}%" transition:slide={colSlide}>
        <TocPanel
          chapters={doc?.chapters ?? []}
          currentChapterId={currentChapter?.id ?? null}
          annotationCount={annotations.length}
          totalChars={doc?.total_chars ?? 0}
          onSelect={selectChapter}
        />
      </aside>
      <div
        class="resize-handle"
        class:active={dragging === "contents"}
        onmousedown={(e) => onResizeMouseDown("contents", e)}
        onkeydown={(e) => onResizeKey("contents", e)}
        ondblclick={() => resetWidth("contents")}
        role="slider"
        aria-orientation="vertical"
        aria-label="Resize contents panel"
        aria-valuenow={widthPx("contents")}
        aria-valuemin={180}
        aria-valuemax={panelMaxPx}
        aria-valuetext="{widthPx("contents")}px"
        tabindex="0"
        title="Drag or use ← → · double-click to reset"
      ></div>
    {/if}

    <section class="col reader" style="flex: 1" aria-busy={status === "opening"}>
      <Reader
        {doc}
        {currentChapter}
        {currentBlocks}
        {annotations}
        {status}
        bind:pendingSelection
        bind:highlightedAnnotationId
        onHighlight={addHighlight}
        onNote={addNoteFromSelection}
        onCancelSelection={() => (pendingSelection = null)}
        onUpdateNote={updateNote}
        onDelete={deleteAnnotation}
      />
    </section>

    {#if columnWidths.notes > 0}
      <div
        class="resize-handle"
        class:active={dragging === "notes"}
        onmousedown={(e) => onResizeMouseDown("notes", e)}
        onkeydown={(e) => onResizeKey("notes", e)}
        ondblclick={() => resetWidth("notes")}
        role="slider"
        aria-orientation="vertical"
        aria-label="Resize notes panel"
        aria-valuenow={widthPx("notes")}
        aria-valuemin={180}
        aria-valuemax={panelMaxPx}
        aria-valuetext="{widthPx("notes")}px"
        tabindex="0"
        title="Drag or use ← → · double-click to reset"
      ></div>
      <aside class="col notes" style="width: {columnWidths.notes}%" transition:slide={colSlide}>
        <NotesPanel
          {doc}
          {annotations}
          bind:editingAnnotationId
          bind:noteDraft
          onSave={saveNote}
          onCancel={cancelNote}
          onDelete={deleteAnnotation}
          onJump={scrollToAnnotation}
        />
      </aside>
    {/if}
  </div>
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    color: var(--fg);
  }
  .app.dragging {
    cursor: col-resize;
    user-select: none;
  }

  /* ── Error banner ── */
  .banner.error {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-4);
    background: var(--danger-soft);
    border-bottom: 1px solid var(--danger);
    padding: var(--space-3) var(--space-5);
    color: var(--fg);
    font-size: var(--text-sm);
  }
  .banner-text {
    min-width: 0;
    flex: 1;
  }
  .banner-text strong {
    color: var(--danger);
    font-weight: var(--weight-semibold);
  }
  .banner-text details {
    margin-top: var(--space-1);
    color: var(--fg-muted);
    font-size: var(--text-xs);
  }
  .banner-text summary {
    cursor: pointer;
  }
  .banner-text pre {
    margin: var(--space-1) 0 0;
    max-height: 6rem;
    overflow: auto;
    font-family: var(--font-mono);
    white-space: pre-wrap;
  }
  .banner-actions {
    display: flex;
    gap: var(--space-2);
    flex-shrink: 0;
  }
  .banner-btn {
    background: transparent;
    border: 1px solid var(--border-strong);
    color: var(--fg);
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .banner-btn:hover {
    background: var(--bg-hover);
  }

  /* ── Three columns ── */
  .three-col {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
  .col {
    height: 100%;
    overflow-y: auto;
    background: var(--bg);
    padding: var(--space-4);
    min-width: 0;
  }
  .col.contents {
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
  }
  .col.notes {
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
  }
  .reader {
    background: var(--bg-reader);
    padding: var(--space-6) var(--space-8);
    display: flex;
    flex-direction: column;
  }
  .reader[aria-busy="true"] {
    opacity: 0.6;
    pointer-events: none;
  }

  /* ── Resize handle: 8px hit area, 1px visible line that turns accent ── */
  .resize-handle {
    position: relative;
    width: 8px;
    margin: 0 -4px; /* overlap the neighbours so the panels keep their border */
    z-index: 1;
    cursor: col-resize;
    flex-shrink: 0;
    outline: none;
  }
  .resize-handle::after {
    content: "";
    position: absolute;
    inset: 0 3px;
    background: transparent;
    transition: background var(--ease);
  }
  .resize-handle:hover::after,
  .resize-handle.active::after,
  .resize-handle:focus-visible::after {
    background: var(--accent);
  }
</style>
