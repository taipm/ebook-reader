import type { FocusMode } from "$lib/types";

/** ⌘<key> → focus mode. ⌘0 is handled separately: toggle reader ↔ last mode. */
export const SHORTCUTS: Record<string, FocusMode> = {
  "1": "normal",
  "2": "reader",
  "3": "research",
  "4": "notes",
};

/** Button labels — e2e/tests/basic.spec.ts binds to these exact strings. */
export const MODE_LABELS: Record<FocusMode, string> = {
  normal: "Normal",
  reader: "Reader",
  research: "Research",
  notes: "Notes",
};
