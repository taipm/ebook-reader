<script lang="ts">
  import type { PendingSelection } from "$lib/types";

  let {
    pendingSelection,
    onHighlight,
    onNote,
    onCancel,
  }: {
    pendingSelection: PendingSelection;
    onHighlight: () => void;
    onNote: () => void;
    onCancel: () => void;
  } = $props();

  const GAP = 8;
  const HEIGHT = 40;
  // Thiếu chỗ phía trên (selection sát đỉnh nội dung) → lật xuống dưới selection.
  const below = $derived(pendingSelection.rect.y < HEIGHT + GAP);
  const top = $derived(
    below
      ? (pendingSelection.rect_bottom ?? pendingSelection.rect.y) + GAP
      : pendingSelection.rect.y - HEIGHT - GAP
  );
</script>

<div
  class="selection-toolbar"
  class:below
  role="toolbar"
  aria-label="Selection"
  style="left: {pendingSelection.rect.x}px; top: {top}px"
>
  <button class="tool-btn" onclick={onHighlight}>Highlight</button>
  <button class="tool-btn" onclick={onNote}>Add note</button>
  <button class="tool-btn cancel" onclick={onCancel} aria-label="Dismiss">✕</button>
</div>

<style>
  .selection-toolbar {
    position: absolute;
    transform: translateX(-50%);
    display: flex;
    gap: var(--space-1);
    padding: var(--space-1);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    font-family: var(--font-ui);
    z-index: 100;
  }
  .tool-btn {
    background: transparent;
    color: var(--fg);
    border: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .tool-btn:hover {
    background: var(--bg-hover);
  }
  .tool-btn.cancel {
    color: var(--fg-muted);
  }
  @media (prefers-reduced-motion: no-preference) {
    .tool-btn {
      transition: background var(--ease);
    }
  }
</style>
