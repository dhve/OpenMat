// The .omnb on-disk format: a thin, versioned envelope around whatever
// opaque notebook snapshot window.__openmat_get_notebook() returns. This
// module never inspects the notebook shape itself, only the envelope
// (schema_version) around it, so the notebook editor implementation and
// the persistence format can evolve independently. Format documented
// before M1 per specs/m0-milestone.md acceptance row 8.

export const CURRENT_SCHEMA_VERSION = 1;

/** Opaque notebook snapshot: whatever shape window.__openmat_get_notebook()
 * returns and window.__openmat_set_notebook() accepts (cells, slider
 * parameter values, evaluation counter, notebook title, and each cell's
 * last KernelResult-derived output view). Persistence treats this as a
 * serializable black box and only wraps/unwraps it. */
export type NotebookSnapshot = unknown;

export interface OmnbFile {
  schema_version: number;
  notebook: NotebookSnapshot;
}

/** The window-global contract this module reads from and writes to.
 * `__openmat_get_notebook` / `__openmat_set_notebook` are implemented
 * elsewhere (the notebook/editor workstream); `__openmat_save` /
 * `__openmat_open` are registered by this module's init(). Declared as a
 * local interface (not a global augmentation) so this file cannot collide
 * with a conflicting declaration elsewhere in the app. */
export interface OpenMatWindow {
  __openmat_get_notebook?: () => NotebookSnapshot;
  __openmat_set_notebook?: (notebook: NotebookSnapshot) => void;
  __openmat_save?: (path?: string | null) => Promise<void>;
  __openmat_open?: () => Promise<void>;
}
