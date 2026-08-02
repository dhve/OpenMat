// Thin wrapper around the notebook_save / notebook_open Tauri commands
// (app/src-tauri/src/files.rs). Tauri camelCases Rust parameter names by
// default when matching the JS invoke() payload, so files.rs's
// `notebook_save(content: String, path: Option<String>)` is reached here
// with a `{ content, path }` payload (already camelCase, no rename
// needed); same convention as src/engine/tauriEngine.ts.

import { invoke } from "@tauri-apps/api/core";

export function runningUnderTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Save `content` to `path`, or show a native save dialog when `path` is
 * null. Returns the path saved to, or null if the user cancelled the
 * dialog. */
export async function tauriSaveNotebook(content: string, path: string | null): Promise<string | null> {
  return invoke<string | null>("notebook_save", { content, path });
}

/** Show a native open dialog. Returns the chosen path and its file
 * contents, or null if the user cancelled. */
export async function tauriOpenNotebook(): Promise<[string, string] | null> {
  return invoke<[string, string] | null>("notebook_open");
}
