import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

// A single static import: this file's whole point is exercising the
// keydown listener index.ts registers on import (init() self-invokes at
// the bottom of that module), so unlike index.test.ts it must not
// reimport the module or a second listener would end up attached.
import "./index";

// See index.test.ts for why this isn't `Window & {...}`.
interface TestWindowProps {
  __TAURI_INTERNALS__?: unknown;
  __openmat_get_notebook?: () => unknown;
  __openmat_set_notebook?: (notebook: unknown) => void;
}

function win(): TestWindowProps {
  return window as unknown as TestWindowProps;
}

describe("Cmd+S / Cmd+O keyboard shortcuts", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    win().__TAURI_INTERNALS__ = {};
    win().__openmat_get_notebook = () => ({ title: "Demo" });
    win().__openmat_set_notebook = vi.fn();
  });

  // Path-remembering behavior (first save passes path=null, later saves
  // reuse the returned path) is covered in index.test.ts; these two tests
  // share one module instance with the rest of this file (see the comment
  // above the `import "./index"` at the top), so `lastPath` legitimately
  // carries over between them, same as it would across two real Cmd+S
  // presses in one running app. Only the command name is asserted here.

  it("Cmd+S saves and prevents the browser's default save-page dialog", async () => {
    invokeMock.mockResolvedValueOnce("/tmp/Demo.omnb");

    const event = new KeyboardEvent("keydown", { key: "s", metaKey: true, cancelable: true });
    window.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(invokeMock).toHaveBeenCalledWith("notebook_save", expect.anything()));
  });

  it("Ctrl+S saves too, for non-Mac keyboards", async () => {
    invokeMock.mockResolvedValueOnce("/tmp/Demo.omnb");

    const event = new KeyboardEvent("keydown", { key: "s", ctrlKey: true, cancelable: true });
    window.dispatchEvent(event);

    await vi.waitFor(() => expect(invokeMock).toHaveBeenCalledWith("notebook_save", expect.anything()));
  });

  it("Cmd+O opens and prevents the browser's default open dialog", async () => {
    const fileText = JSON.stringify({ schema_version: 1, notebook: { title: "Demo" } });
    invokeMock.mockResolvedValueOnce(["/tmp/Demo.omnb", fileText]);

    const event = new KeyboardEvent("keydown", { key: "o", metaKey: true, cancelable: true });
    window.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(invokeMock).toHaveBeenCalledWith("notebook_open"));
  });

  it("plain 's' with no modifier key does not trigger a save", async () => {
    const event = new KeyboardEvent("keydown", { key: "s", cancelable: true });
    window.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(false);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("Cmd+Shift+S does not trigger the plain save shortcut", async () => {
    const event = new KeyboardEvent("keydown", { key: "s", metaKey: true, shiftKey: true, cancelable: true });
    window.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(false);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
