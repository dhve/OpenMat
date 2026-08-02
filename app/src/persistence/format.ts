// Serialize/parse the .omnb envelope. Kept separate from the window-global
// wiring in index.ts so it can be unit tested without touching window,
// Tauri's invoke bridge, or the DOM.

import { CURRENT_SCHEMA_VERSION, type NotebookSnapshot, type OmnbFile } from "./types";

export class UnsupportedSchemaVersionError extends Error {
  readonly version: unknown;

  constructor(version: unknown) {
    super(`Unsupported .omnb schema_version: ${JSON.stringify(version)}`);
    this.name = "UnsupportedSchemaVersionError";
    this.version = version;
  }
}

export class InvalidNotebookFileError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidNotebookFileError";
  }
}

/** Wrap a notebook snapshot in the versioned .omnb envelope and serialize
 * it to JSON text. Fixed key order and indentation (via the OmnbFile field
 * order and a constant indent) so a save-load-save round trip is
 * byte-stable, per specs/m0-milestone.md acceptance row 8. */
export function serializeNotebook(notebook: NotebookSnapshot): string {
  const file: OmnbFile = { schema_version: CURRENT_SCHEMA_VERSION, notebook };
  return JSON.stringify(file, null, 2) + "\n";
}

/** Parse .omnb JSON text back into a notebook snapshot, throwing on
 * malformed JSON, a missing envelope, or a schema_version this build does
 * not understand. */
export function parseNotebookFile(text: string): NotebookSnapshot {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (err) {
    throw new InvalidNotebookFileError(`Not valid .omnb JSON: ${(err as Error).message}`);
  }

  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed) || !("schema_version" in parsed)) {
    throw new InvalidNotebookFileError("Not a valid .omnb file: missing schema_version.");
  }

  const file = parsed as OmnbFile;
  if (file.schema_version !== CURRENT_SCHEMA_VERSION) {
    throw new UnsupportedSchemaVersionError(file.schema_version);
  }

  if (!("notebook" in file)) {
    throw new InvalidNotebookFileError("Not a valid .omnb file: missing notebook field.");
  }

  return file.notebook;
}
