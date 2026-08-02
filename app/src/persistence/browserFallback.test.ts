import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { downloadNotebookFile, pickNotebookFile } from "./browserFallback";

// jsdom does not implement URL.createObjectURL / revokeObjectURL, so stub
// them for the tests that exercise the download path.
describe("downloadNotebookFile", () => {
  let originalCreateObjectURL: typeof URL.createObjectURL;
  let originalRevokeObjectURL: typeof URL.revokeObjectURL;

  beforeEach(() => {
    originalCreateObjectURL = URL.createObjectURL;
    originalRevokeObjectURL = URL.revokeObjectURL;
    URL.createObjectURL = vi.fn(() => "blob:mock-url");
    URL.revokeObjectURL = vi.fn();
  });

  afterEach(() => {
    URL.createObjectURL = originalCreateObjectURL;
    URL.revokeObjectURL = originalRevokeObjectURL;
    vi.restoreAllMocks();
  });

  it("creates an object URL, clicks a download anchor with the given file name, and revokes the URL", () => {
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    const appendSpy = vi.spyOn(document.body, "appendChild");

    downloadNotebookFile('{"schema_version":1}', "Demo.omnb");

    expect(URL.createObjectURL).toHaveBeenCalledTimes(1);
    expect(appendSpy).toHaveBeenCalledTimes(1);
    const anchor = appendSpy.mock.calls[0][0] as HTMLAnchorElement;
    expect(anchor.tagName).toBe("A");
    expect(anchor.download).toBe("Demo.omnb");
    expect(anchor.href).toContain("blob:mock-url");
    expect(clickSpy).toHaveBeenCalledTimes(1);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:mock-url");
  });
});

describe("pickNotebookFile", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("resolves with the chosen file's name and text content", async () => {
    vi.spyOn(HTMLInputElement.prototype, "click").mockImplementation(function (this: HTMLInputElement) {
      const file = new File(['{"schema_version":1,"notebook":{}}'], "Saved.omnb", { type: "application/json" });
      Object.defineProperty(this, "files", { value: [file], configurable: true });
      this.dispatchEvent(new Event("change"));
    });

    const result = await pickNotebookFile();

    expect(result).toEqual({ fileName: "Saved.omnb", content: '{"schema_version":1,"notebook":{}}' });
  });

  it("resolves null when the user cancels without choosing a file", async () => {
    vi.spyOn(HTMLInputElement.prototype, "click").mockImplementation(function (this: HTMLInputElement) {
      this.dispatchEvent(new Event("change"));
    });

    const result = await pickNotebookFile();

    expect(result).toBeNull();
  });
});
