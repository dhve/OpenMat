// Notebook save/open wiring: registers window.__openmat_save and
// window.__openmat_open, and the Cmd+S / Cmd+O keyboard shortcuts. Save
// reads the current notebook via window.__openmat_get_notebook (the
// notebook/editor workstream's contract), wraps it in the versioned .omnb
// envelope (see format.ts), and hands it to the Tauri notebook_save
// command, or downloads it as a file outside Tauri. Open is the mirror:
// read a file, validate schema_version, and hand the notebook back through
// window.__openmat_set_notebook.
//
// Self-initializes on import (see init() call at the bottom), so the
// integrator only needs `import "./persistence"` somewhere in the app's
// entry point; this directory does not wire itself into main.tsx or
// App.tsx.

import { parseNotebookFile, serializeNotebook } from "./format";
import { runningUnderTauri, tauriOpenNotebook, tauriSaveNotebook } from "./tauriBridge";
import { downloadNotebookFile, pickNotebookFile } from "./browserFallback";
import type { OpenMatWindow } from "./types";

export { serializeNotebook, parseNotebookFile, UnsupportedSchemaVersionError, InvalidNotebookFileError } from "./format";
export { CURRENT_SCHEMA_VERSION } from "./types";
export type { NotebookSnapshot, OmnbFile } from "./types";

const DEFAULT_FILE_NAME = "Untitled.omnb";

function openmatWindow(): OpenMatWindow {
  return window as unknown as OpenMatWindow;
}

// Remembers the path a notebook was last saved to or opened from, so a
// plain Cmd+S re-saves silently instead of showing the save dialog every
// time. Lives for the lifetime of the page/app session; a fresh reload has
// no remembered path yet, same as most desktop apps losing "current file"
// state on relaunch until something is opened or saved again.
let lastPath: string | null = null;

function baseFileName(path: string | null): string {
  if (!path) return DEFAULT_FILE_NAME;
  const segments = path.split(/[\\/]/);
  return segments[segments.length - 1] || DEFAULT_FILE_NAME;
}

/** Save the current notebook (read via window.__openmat_get_notebook).
 * Pass `path` to force a specific destination, or `null` to force a fresh
 * "Save As" dialog; omit it to reuse the remembered last path, falling
 * back to a save dialog the first time. */
export async function saveNotebook(path?: string | null): Promise<void> {
  const win = openmatWindow();
  const getNotebook = win.__openmat_get_notebook;
  if (!getNotebook) {
    throw new Error("window.__openmat_get_notebook is not registered; nothing to save.");
  }

  const content = serializeNotebook(getNotebook());
  const targetPath = path === undefined ? lastPath : path;

  if (runningUnderTauri()) {
    const savedPath = await tauriSaveNotebook(content, targetPath);
    if (savedPath !== null) {
      lastPath = savedPath;
    }
    return;
  }

  downloadNotebookFile(content, baseFileName(targetPath));
}

/** Open a notebook: show a file picker, validate schema_version, and hand
 * the result to window.__openmat_set_notebook. No-op if the user cancels. */
export async function openNotebook(): Promise<void> {
  const win = openmatWindow();
  const setNotebook = win.__openmat_set_notebook;
  if (!setNotebook) {
    throw new Error("window.__openmat_set_notebook is not registered; cannot open.");
  }

  if (runningUnderTauri()) {
    const opened = await tauriOpenNotebook();
    if (opened === null) return;
    const [path, content] = opened;
    setNotebook(parseNotebookFile(content));
    lastPath = path;
    return;
  }

  const picked = await pickNotebookFile();
  if (picked === null) return;
  setNotebook(parseNotebookFile(picked.content));
  lastPath = picked.fileName;
}

function isSaveShortcut(event: KeyboardEvent): boolean {
  return (event.metaKey || event.ctrlKey) && !event.shiftKey && event.key.toLowerCase() === "s";
}

function isOpenShortcut(event: KeyboardEvent): boolean {
  return (event.metaKey || event.ctrlKey) && !event.shiftKey && event.key.toLowerCase() === "o";
}

function handleKeydown(event: KeyboardEvent): void {
  if (isSaveShortcut(event)) {
    event.preventDefault();
    void saveNotebook();
  } else if (isOpenShortcut(event)) {
    event.preventDefault();
    void openNotebook();
  }
}

let initialized = false;

/** Register window.__openmat_save / window.__openmat_open and the Cmd+S /
 * Cmd+O keyboard shortcuts. Idempotent: only wires up once even if called,
 * or this module is imported, more than once. */
export function init(): void {
  if (initialized) return;
  initialized = true;

  const win = openmatWindow();
  win.__openmat_save = saveNotebook;
  win.__openmat_open = openNotebook;

  if (typeof window !== "undefined") {
    window.addEventListener("keydown", handleKeydown);
  }
}

init();
