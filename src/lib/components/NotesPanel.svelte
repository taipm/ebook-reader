<script lang="ts">
  import { tick } from "svelte";
  import type { Annotation, Chapter, OpenDocumentResponse } from "$lib/types";

  let {
    doc,
    annotations,
    editingAnnotationId = $bindable(),
    noteDraft = $bindable(),
    onSave,
    onCancel,
    onDelete,
    onJump,
  }: {
    doc: OpenDocumentResponse | null;
    annotations: Annotation[];
    editingAnnotationId: string | null;
    noteDraft: string;
    onSave: () => void;
    onCancel: () => void;
    onDelete: (id: string) => void;
    onJump: (ann: Annotation) => void;
  } = $props();

  const allNotes = $derived(annotations);

  // ── Pending delete with undo (P0-3) ──────────────────────────────────
  // ponytail: one pending delete at a time; a second delete commits the first.
  const UNDO_MS = 5000;
  let pendingDeleteId = $state<string | null>(null);
  let pendingTimer: ReturnType<typeof setTimeout> | null = null;

  function commitPendingDelete() {
    if (pendingTimer) clearTimeout(pendingTimer);
    pendingTimer = null;
    if (pendingDeleteId) onDelete(pendingDeleteId);
    pendingDeleteId = null;
  }

  function requestDelete(id: string) {
    commitPendingDelete();
    if (editingAnnotationId === id) onCancel();
    pendingDeleteId = id;
    pendingTimer = setTimeout(commitPendingDelete, UNDO_MS);
  }

  function undoDelete() {
    if (pendingTimer) clearTimeout(pendingTimer);
    pendingTimer = null;
    pendingDeleteId = null;
  }

  // ── Group by chapter, in document order (P1-10) ──────────────────────
  const groups = $derived.by(() => {
    const visible = allNotes.filter((a) => a.id !== pendingDeleteId);
    const order = new Map((doc?.chapters ?? []).map((c, i) => [c.id, i]));
    const byChapter = new Map<string, Annotation[]>();
    for (const a of visible) {
      const list = byChapter.get(a.location.chapter_id) ?? [];
      list.push(a);
      byChapter.set(a.location.chapter_id, list);
    }
    return [...byChapter.entries()]
      .sort(([a], [b]) => (order.get(a) ?? 1e9) - (order.get(b) ?? 1e9))
      .map(([chapterId, items]) => ({
        chapter: doc?.chapters.find((c) => c.id === chapterId) as Chapter | undefined,
        items: items.sort((x, y) => x.location.char_start - y.location.char_start),
      }));
  });

  // ── Editor: autofocus, ⌘Enter save, Esc cancel (P1-5/6) ──────────────
  let textareaEl = $state<HTMLTextAreaElement | null>(null);
  $effect(() => {
    if (editingAnnotationId) tick().then(() => textareaEl?.focus());
  });

  // editing id points at a removed annotation (P2-12): drop the stale editor
  $effect(() => {
    if (editingAnnotationId && !allNotes.some((a) => a.id === editingAnnotationId)) onCancel();
  });

  const canSave = $derived(noteDraft.trim().length > 0);

  function save() {
    if (canSave) onSave();
    else onCancel();
  }

  function onEditorKey(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      e.stopPropagation();
      save();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onCancel();
    }
  }

  function startEdit(ann: Annotation) {
    editingAnnotationId = ann.id;
    noteDraft = ann.note;
  }

  const MARKERS = new Set(["yellow", "green", "blue", "rose"]);
  function markerOf(ann: Annotation): string {
    if (MARKERS.has(ann.color)) return ann.color;
    return ann.kind === "note" ? "rose" : "yellow";
  }

  function clip(s: string, n: number): string {
    return s.length > n ? s.slice(0, n) + "…" : s;
  }
</script>

<div class="panel">
  <div class="panel-head">
    <span>Notes</span>
    {#if allNotes.length}<span class="count">{allNotes.length}</span>{/if}
  </div>

  {#if pendingDeleteId}
    <div class="toast" role="status">
      <span>Note deleted</span>
      <button class="link" onclick={undoDelete}>Undo</button>
    </div>
  {/if}

  {#if allNotes.length === 0}
    <div class="empty">
      <p class="empty-title">No notes yet</p>
      <p>Select a passage in the book, then choose Highlight or Note. ⌘3 widens this panel.</p>
    </div>
  {:else}
    {#each groups as group (group.chapter?.id ?? "?")}
      <section class="group">
        <h4 class="group-title">
          <span class="group-name">{group.chapter?.title ?? "Unknown chapter"}</span>
          <span class="count">{group.items.length}</span>
        </h4>

        {#each group.items as ann (ann.id)}
          {#if ann.id === editingAnnotationId}
            <div class="card editor" data-marker={markerOf(ann)}>
              <blockquote class="quote">{clip(ann.highlighted_text, 140)}</blockquote>
              <textarea
                class="input"
                placeholder="Write a note…"
                bind:this={textareaEl}
                bind:value={noteDraft}
                onkeydown={onEditorKey}
                rows="4"
              ></textarea>
              <div class="actions">
                <button class="btn primary" onclick={save} disabled={!canSave}>Save note</button>
                <button class="btn" onclick={onCancel}>Cancel</button>
                <span class="hint">⌘↩ save · Esc cancel</span>
              </div>
            </div>
          {:else}
            <article class="card" data-marker={markerOf(ann)}>
              <div class="card-head">
                <span class="kind">{ann.note ? "Note" : "Highlight"}</span>
                <button
                  class="delete"
                  aria-label="Delete note"
                  title="Delete"
                  onclick={() => requestDelete(ann.id)}
                >×</button>
              </div>
              <blockquote class="quote">{clip(ann.highlighted_text, 140)}</blockquote>
              {#if ann.note}
                <p class="body">{ann.note}</p>
              {/if}
              {#if ann.tags.length}
                <div class="tags">
                  {#each ann.tags as tag}<span class="tag">{tag}</span>{/each}
                </div>
              {/if}
              <div class="actions">
                <button class="link" onclick={() => onJump(ann)}>Jump to source</button>
                <button class="link" onclick={() => startEdit(ann)}>
                  {ann.note ? "Edit" : "Add note"}
                </button>
              </div>
            </article>
          {/if}
        {/each}
      </section>
    {/each}
  {/if}
</div>

<style>
  .panel {
    font-family: var(--font-ui);
    font-size: var(--text-sm);
    color: var(--fg);
  }
  .panel-head {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    color: var(--fg-muted);
    font-size: var(--text-xs);
  }
  .count {
    color: var(--fg-muted);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  .toast {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-raised);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    font-size: var(--text-xs);
  }

  .empty {
    padding: var(--space-5) var(--space-3);
    text-align: center;
    color: var(--fg-muted);
    font-size: var(--text-xs);
  }
  .empty p {
    margin: 0;
  }
  .empty-title {
    color: var(--fg);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    margin-bottom: var(--space-1);
  }

  .group + .group {
    margin-top: var(--space-4);
  }
  .group-title {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    margin: 0 0 var(--space-2);
    padding: var(--space-1) 0;
    background: var(--bg-panel);
    color: var(--fg-muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }
  .group-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card {
    position: relative;
    padding: var(--space-3);
    background: var(--note-bg);
    border: 1px solid var(--note-border);
    border-left: 3px solid var(--hl-yellow-fg);
    border-radius: var(--radius-md);
    transition: border-color var(--ease);
  }
  .card + .card {
    margin-top: var(--space-2);
  }
  .card[data-marker="rose"] { border-left-color: var(--hl-rose-fg); }
  .card[data-marker="green"] { border-left-color: var(--hl-green-fg); }
  .card[data-marker="blue"] { border-left-color: var(--hl-blue-fg); }
  .card.editor {
    border-color: var(--border-strong);
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-1);
  }
  .kind {
    font-size: var(--text-xs);
    color: var(--fg-muted);
  }
  .delete {
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--fg-muted);
    font-size: var(--text-md);
    line-height: 1;
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--ease), background var(--ease);
  }
  .card:hover .delete,
  .card:focus-within .delete {
    opacity: 1;
  }
  .delete:hover,
  .delete:focus-visible {
    opacity: 1;
    background: var(--danger-soft);
    color: var(--danger);
  }

  .quote {
    margin: 0 0 var(--space-2);
    padding-left: var(--space-2);
    border-left: 2px solid var(--border);
    color: var(--fg-muted);
    font-family: var(--font-reading);
    font-style: italic;
  }
  .body {
    margin: 0 0 var(--space-2);
    white-space: pre-wrap;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    margin-bottom: var(--space-2);
  }
  .tag {
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-xs);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .hint {
    margin-left: auto;
    color: var(--fg-muted);
    font-size: var(--text-xs);
  }
  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: var(--text-xs);
    text-decoration: underline;
    text-decoration-style: dotted;
    cursor: pointer;
  }
  .link:hover {
    color: var(--accent-hover);
  }

  .input {
    width: 100%;
    margin-bottom: var(--space-2);
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-raised);
    resize: vertical;
  }
  .input:focus-visible {
    border-color: var(--accent);
  }
  .btn {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: transparent;
    font-size: var(--text-xs);
    cursor: pointer;
    transition: background var(--ease);
  }
  .btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  .btn.primary {
    border-color: transparent;
    background: var(--accent);
    color: var(--fg-on-accent);
  }
  .btn.primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
