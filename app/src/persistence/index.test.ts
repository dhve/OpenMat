import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock @tauri-apps/api/core's invoke before importing anything from this
// directory, so tauriBridge.ts picks up the mock instead of the real
// bridge (which would fail outside an actual Tauri shell).
const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

// Not `Window & {...}`: notebook/types.ts already globally declares
// `Window.__openmat_get_notebook` (etc.) with the real NotebookDoc type, so
// intersecting here would fight that declaration. `as unknown as` sidesteps
// the merge the same way src/persistence/index.ts itself does.
interface TestWindowProps {
  __TAURI_INTERNALS__?: unknown;
  __openmat_get_notebook?: () => unknown;
  __openmat_set_notebook?: (notebook: unknown) => void;
  __openmat_save?: (path?: string | null) => Promise<void>;
  __openmat_open?: () => Promise<void>;
}

function win(): TestWindowProps {
  return window as unknown as TestWindowProps;
}

// Each test gets a fresh copy of the module (fresh `lastPath` /
// `initialized` module state) via vi.resetModules(), so save/open path
// memory from one test can never leak into the next.
async function freshPersistence() {
  vi.resetModules();
  return import("./index");
}

describe("persistence/index, Tauri commands", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    win().__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    delete win().__TAURI_INTERNALS__;
    delete win().__openmat_get_notebook;
    delete win().__openmat_set_notebook;
  });

  it("registers window.__openmat_save and window.__openmat_open on import", async () => {
    await freshPersistence();
    expect(typeof win().__openmat_save).toBe("function");
    expect(typeof win().__openmat_open).toBe("function");
  });

  it("saves with path=null on first save and serializes the .omnb envelope", async () => {
    const { saveNotebook, CURRENT_SCHEMA_VERSION } = await freshPersistence();
    const notebook = { title: "Demo", cells: [] };
    win().__openmat_get_notebook = () => notebook;
    invokeMock.mockResolvedValueOnce("/tmp/Demo.omnb");

    await saveNotebook();

    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as [string, { content: string; path: string | null }];
    expect(command).toBe("notebook_save");
    expect(args.path).toBeNull();
    expect(JSON.parse(args.content)).toEqual({ schema_version: CURRENT_SCHEMA_VERSION, notebook });
  });

  it("remembers the returned path and reuses it silently on the next save", async () => {
    const { saveNotebook } = await freshPersistence();
    win().__openmat_get_notebook = () => ({ title: "Demo" });

    invokeMock.mockResolvedValueOnce("/tmp/Demo.omnb");
    await saveNotebook();

    invokeMock.mockResolvedValueOnce("/tmp/Demo.omnb");
    await saveNotebook();

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(invokeMock.mock.calls[1][1].path).toBe("/tmp/Demo.omnb");
  });

  it("forces a fresh save dialog when path is explicitly null, ignoring the remembered path", async () => {
    const { saveNotebook } = await freshPersistence();
    win().__openmat_get_notebook = () => ({ title: "Demo" });

    invokeMock.mockResolvedValueOnce("/tmp/Demo.omnb");
    await saveNotebook();

    invokeMock.mockResolvedValueOnce("/tmp/Demo-2.omnb");
    await saveNotebook(null);

    expect(invokeMock.mock.calls[1][1].path).toBeNull();
  });

  it("leaves the remembered path unset when the save dialog is cancelled", async () => {
    const { saveNotebook } = await freshPersistence();
    win().__openmat_get_notebook = () => ({ title: "Demo" });

    invokeMock.mockResolvedValueOnce(null);
    await saveNotebook();

    invokeMock.mockResolvedValueOnce("/tmp/Demo.omnb");
    await saveNotebook();

    expect(invokeMock.mock.calls[1][1].path).toBeNull();
  });

  it("rejects when window.__openmat_get_notebook is not registered", async () => {
    const { saveNotebook } = await freshPersistence();
    await expect(saveNotebook()).rejects.toThrow(/__openmat_get_notebook/);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("opens a notebook, validates schema_version, and restores it via __openmat_set_notebook", async () => {
    const { openNotebook, CURRENT_SCHEMA_VERSION } = await freshPersistence();
    const notebook = { title: "Demo", cells: [{ id: "cell-1", kind: "title", text: "Demo" }] };
    const fileText = JSON.stringify({ schema_version: CURRENT_SCHEMA_VERSION, notebook });
    invokeMock.mockResolvedValueOnce(["/tmp/Demo.omnb", fileText]);
    const setNotebook = vi.fn();
    win().__openmat_set_notebook = setNotebook;

    await openNotebook();

    expect(invokeMock).toHaveBeenCalledWith("notebook_open");
    expect(setNotebook).toHaveBeenCalledWith(notebook);
  });

  it("does nothing when the open dialog is cancelled", async () => {
    const { openNotebook } = await freshPersistence();
    invokeMock.mockResolvedValueOnce(null);
    const setNotebook = vi.fn();
    win().__openmat_set_notebook = setNotebook;

    await openNotebook();

    expect(setNotebook).not.toHaveBeenCalled();
  });

  it("rejects an unknown schema_version and does not restore the notebook", async () => {
    const { openNotebook } = await freshPersistence();
    const fileText = JSON.stringify({ schema_version: 999, notebook: {} });
    invokeMock.mockResolvedValueOnce(["/tmp/Demo.omnb", fileText]);
    const setNotebook = vi.fn();
    win().__openmat_set_notebook = setNotebook;

    await expect(openNotebook()).rejects.toThrow(/schema_version/);
    expect(setNotebook).not.toHaveBeenCalled();
  });

  it("rejects when window.__openmat_set_notebook is not registered, without opening a dialog", async () => {
    const { openNotebook } = await freshPersistence();
    await expect(openNotebook()).rejects.toThrow(/__openmat_set_notebook/);
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("persistence/index, non-Tauri fallback selection", () => {
  afterEach(() => {
    delete win().__TAURI_INTERNALS__;
    delete win().__openmat_get_notebook;
  });

  it("does not call the Tauri invoke bridge when __TAURI_INTERNALS__ is absent", async () => {
    delete win().__TAURI_INTERNALS__;
    invokeMock.mockReset();
    const { saveNotebook } = await freshPersistence();
    win().__openmat_get_notebook = () => ({ title: "Demo" });

    // jsdom has no URL.createObjectURL; stub it so the browser-download
    // fallback path doesn't throw. This test only cares that Tauri's
    // invoke bridge is never reached outside Tauri.
    const originalCreateObjectURL = URL.createObjectURL;
    const originalRevokeObjectURL = URL.revokeObjectURL;
    (URL as unknown as { createObjectURL: () => string }).createObjectURL = vi.fn(() => "blob:mock");
    (URL as unknown as { revokeObjectURL: () => void }).revokeObjectURL = vi.fn();
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});

    await saveNotebook();

    expect(invokeMock).not.toHaveBeenCalled();

    clickSpy.mockRestore();
    URL.createObjectURL = originalCreateObjectURL;
    URL.revokeObjectURL = originalRevokeObjectURL;
  });
});
