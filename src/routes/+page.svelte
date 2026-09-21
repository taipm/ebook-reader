<script lang="ts">
  // MVP 1 placeholder — PR #3 sẽ build 3-column UI:
  // [Contents | Reader | Notes]
  //
  // Phase 1 (this PR): just verify Tauri boots and Svelte renders.
  let booted = $state(false);
  $effect(() => {
    booted = true;
  });
</script>

<main class="splash">
  <h1>ebook-reader</h1>
  <p class="subtitle">Personal knowledge workspace</p>
  {#if booted}
    <p class="status">✓ Tauri + Svelte booted successfully</p>
  {/if}
  <p class="hint">PR #2 sẽ mở được EPUB · PR #3 sẽ có UI 3 cột</p>
</main>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    background: #0f1419;
    color: #e6e6e6;
    font-family:
      -apple-system,
      BlinkMacSystemFont,
      "Segoe UI",
      sans-serif;
    height: 100%;
  }
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
  }
  h1 {
    font-size: 2rem;
    font-weight: 600;
    margin: 0;
    color: #fafafa;
  }
  .subtitle {
    color: #94a3b8;
    margin: 0;
    font-size: 0.95rem;
  }
  .status {
    color: #4ade80;
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 0.9rem;
    margin-top: 1rem;
  }
  .hint {
    color: #64748b;
    font-size: 0.85rem;
    margin-top: 0.5rem;
  }
</style>
