<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";

  // Response shape từ Rust `open_document` command
  type Chapter = {
    id: string;
    parent_id: string | null;
    title: string;
    level: number;
    position: number;
  };

  type OpenDocumentResponse = {
    meta: {
      id: string;
      title: string;
      author: string | null;
      format: string;
      content_hash: string;
      created_at: string;
      updated_at: string;
    };
    chapters: Chapter[];
    block_count: number;
    total_chars: number;
  };

  let status: "idle" | "opening" | "ready" | "error" = $state("idle");
  let errorMsg = $state("");
  let doc: OpenDocumentResponse | null = $state(null);

  async function handleOpen() {
    status = "opening";
    errorMsg = "";
    try {
      // Step 1: mở native file dialog
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "ebook / markdown",
            extensions: ["epub", "md", "markdown"],
          },
        ],
      });
      if (!selected) {
        status = "idle";
        return;
      }
      const path = selected as string;

      // Step 2: gọi Rust command
      const result = await invoke<OpenDocumentResponse>("open_document", { path });
      doc = result;
      status = "ready";
    } catch (e) {
      status = "error";
      errorMsg = String(e);
    }
  }

  function formatHash(s: string): string {
    return s.length > 12 ? `${s.slice(0, 8)}…${s.slice(-4)}` : s;
  }

  function formatChars(n: number): string {
    if (n < 1000) return `${n}`;
    if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`;
    return `${(n / 1_000_000).toFixed(1)}M`;
  }
</script>

<main>
  <header>
    <h1>ebook-reader</h1>
    <p class="tagline">Personal knowledge workspace</p>
  </header>

  <section class="actions">
    <button onclick={handleOpen} disabled={status === "opening"}>
      {status === "opening" ? "Opening…" : "📂 Open EPUB / Markdown"}
    </button>
  </section>

  {#if status === "error"}
    <section class="error">
      <strong>Error:</strong>
      <pre>{errorMsg}</pre>
    </section>
  {/if}

  {#if doc}
    <section class="result">
      <h2>{doc.meta.title}</h2>
      {#if doc.meta.author}
        <p class="author">by {doc.meta.author}</p>
      {/if}

      <div class="stats">
        <div class="stat">
          <span class="label">Format</span>
          <span class="value">{doc.meta.format.toUpperCase()}</span>
        </div>
        <div class="stat">
          <span class="label">Chapters</span>
          <span class="value">{doc.chapters.length}</span>
        </div>
        <div class="stat">
          <span class="label">Blocks</span>
          <span class="value">{doc.block_count}</span>
        </div>
        <div class="stat">
          <span class="label">Total chars</span>
          <span class="value">{formatChars(doc.total_chars)}</span>
        </div>
        <div class="stat">
          <span class="label">Hash</span>
          <span class="value mono">{formatHash(doc.meta.content_hash)}</span>
        </div>
      </div>

      <h3>Table of contents ({doc.chapters.length})</h3>
      <ol class="toc">
        {#each doc.chapters.slice(0, 50) as ch (ch.id)}
          <li style="margin-left: {(ch.level - 1) * 16}px">
            <span class="lvl">L{ch.level}</span>
            <span class="title">{ch.title}</span>
          </li>
        {/each}
        {#if doc.chapters.length > 50}
          <li class="more">… and {doc.chapters.length - 50} more</li>
        {/if}
      </ol>

      <p class="hint">
        🚧 MVP 1 Phase A: EPUB & Markdown loaded successfully. UI 3-column + highlight +
        note sync coming in Phase B.
      </p>
    </section>
  {:else if status === "idle"}
    <section class="empty">
      <p>Chọn 1 file EPUB hoặc Markdown để bắt đầu.</p>
      <p class="hint">Test file available: <code>~/GitHub/ebook-reader/test-data/dracula.epub</code></p>
    </section>
  {/if}
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
    max-width: 880px;
    margin: 0 auto;
    padding: 3rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  header h1 {
    margin: 0;
    font-size: 2.2rem;
    font-weight: 600;
    color: #fafafa;
    letter-spacing: -0.02em;
  }
  .tagline {
    margin: 0.25rem 0 0;
    color: #94a3b8;
    font-size: 0.95rem;
  }
  .actions button {
    background: #2563eb;
    color: #fff;
    border: 0;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    font-size: 0.95rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s;
  }
  .actions button:hover:not(:disabled) {
    background: #1d4ed8;
  }
  .actions button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .error {
    background: #7f1d1d;
    border: 1px solid #b91c1c;
    padding: 1rem;
    border-radius: 8px;
  }
  .error pre {
    margin: 0.5rem 0 0;
    white-space: pre-wrap;
    font-size: 0.85rem;
    color: #fecaca;
  }
  .result {
    background: #1a1f2e;
    border: 1px solid #2d3748;
    padding: 1.5rem;
    border-radius: 12px;
  }
  .result h2 {
    margin: 0 0 0.25rem;
    font-size: 1.8rem;
    color: #fafafa;
  }
  .author {
    margin: 0 0 1.25rem;
    color: #94a3b8;
    font-style: italic;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }
  .stat {
    background: #0f172a;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    border: 1px solid #1e293b;
  }
  .stat .label {
    display: block;
    font-size: 0.7rem;
    text-transform: uppercase;
    color: #64748b;
    letter-spacing: 0.05em;
    margin-bottom: 0.25rem;
  }
  .stat .value {
    display: block;
    font-size: 1.1rem;
    font-weight: 600;
    color: #e2e8f0;
  }
  .stat .value.mono {
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 0.85rem;
  }
  .toc {
    list-style: none;
    padding: 0.75rem 1rem;
    margin: 0.5rem 0 1.5rem;
    max-height: 320px;
    overflow-y: auto;
    background: #0f172a;
    border-radius: 8px;
  }
  .toc li {
    padding: 0.25rem 0;
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
  }
  .toc li .lvl {
    color: #475569;
    font-size: 0.7rem;
    font-family: ui-monospace, SFMono-Regular, monospace;
    min-width: 24px;
  }
  .toc li .title {
    color: #cbd5e1;
    font-size: 0.9rem;
  }
  .toc li.more {
    color: #64748b;
    font-style: italic;
    padding-top: 0.5rem;
  }
  .empty {
    text-align: center;
    padding: 3rem 1rem;
    color: #94a3b8;
  }
  .hint {
    color: #64748b;
    font-size: 0.85rem;
    margin-top: 1rem;
    text-align: center;
  }
  .hint code {
    background: #1e293b;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  h3 {
    margin: 1.5rem 0 0.5rem;
    font-size: 0.95rem;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
  }
</style>
