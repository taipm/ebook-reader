# Design system — ebook-reader

Tokens live in `src/app.css` (imported once by `src/routes/+layout.svelte`).
Components use `var(--token)` only — no hex in `.svelte` files. Dark mode is
automatic via `prefers-color-scheme`; nothing to toggle.

## Direction

A quiet desk, not a dashboard. The reading column is the only thing allowed to
be beautiful: serif, ~64 characters per line, generous leading, a page one step
lighter than the app around it. Chrome (header, TOC, notes) is sans, small,
low-contrast, and stays out of the way. One accent — verdigris — marks actions
and the current item; highlights are four flat marker colours, never the accent.

Not doing: cream + terracotta, pure-black + neon, all-caps eyebrow labels,
identical rounded cards everywhere, decorative gradients.

## Tokens

### Surfaces
| Token | Use |
|---|---|
| `--bg` | app background |
| `--bg-reader` | reading column |
| `--bg-panel` | TOC / notes sidebars |
| `--bg-raised` | popovers, menus, dialogs (+ `--shadow-md/lg`) |
| `--bg-hover`, `--bg-active` | row/button hover + pressed/selected |
| `--border`, `--border-strong` | dividers; inputs/focused containers |

### Text
`--fg` body · `--fg-muted` metadata, secondary · `--fg-faint` placeholders,
disabled · `--fg-on-accent` text on `--accent` fills.

### Accent & status
`--accent` / `--accent-hover` (buttons, links, active chapter) ·
`--accent-soft` tinted bg for active rows · `--accent-ring` focus ring
(already applied globally on `:focus-visible`) · `--danger` / `--danger-soft`.

### Highlights & notes
| Marker | Background | Chip/label text |
|---|---|---|
| yellow | `--hl-yellow` | `--hl-yellow-fg` |
| green | `--hl-green` | `--hl-green-fg` |
| blue | `--hl-blue` | `--hl-blue-fg` |
| rose | `--hl-rose` | `--hl-rose-fg` |

Annotation `color` string should be one of `yellow|green|blue|rose`; map to
`var(--hl-${color})`. Note cards: `--note-bg` + `--note-border`. Text selection
uses `--selection` (set globally).

### Type
| Token | Use |
|---|---|
| `--font-reading` | reader body (serif, system stack) |
| `--font-ui` | everything else |
| `--font-mono` | code blocks, file paths, format badge |
| `--text-xs … --text-2xl` | 12 / 14 / 16 / 20 / 24 / 32 px |
| `--reading-size`, `--reading-leading`, `--reading-measure` | 19px / 1.65 / 64ch — reader sets `font-size`, `line-height`, `max-width` from these; a future font-size control only changes `--reading-size` |
| `--leading-tight` (1.25) headings · `--leading-normal` (1.5) UI |
| `--weight-normal/medium/semibold` | 400 / 500 / 600 |

Reading block example:
```css
.reader-content {
  font-family: var(--font-reading);
  font-size: var(--reading-size);
  line-height: var(--reading-leading);
  max-width: var(--reading-measure);
  margin-inline: auto;
}
.reader-content p + p { margin-top: var(--reading-para-gap); }
```

### Spacing, radius, shadow, motion
`--space-1..6,8` = 4 / 8 / 12 / 16 / 24 / 32 / 48 px ·
`--radius-sm` chips, inputs · `--radius-md` buttons, cards · `--radius-lg` popovers ·
`--shadow-sm/md/lg` (only on raised surfaces) · `--ease` for hover/expand transitions.

### Layout
`--header-h` 44px · `--panel-w` 260px · `--panel-w-min` 180px.

## Rules for components
- Sidebars: `--bg-panel`, `--font-ui`, `--text-sm`, active item `--accent-soft` + `--accent` text. No uppercase section labels; use `--fg-muted` at `--text-xs` instead.
- Buttons: default = transparent + `--border`, hover `--bg-hover`; primary = `--accent` fill + `--fg-on-accent`. Radius `--radius-md`.
- Only raised surfaces get shadows; panels and the reader get borders or nothing.
- Copy: sentence case, verbs on buttons ("Open book", "Save note"), errors say what happened and what to do.
