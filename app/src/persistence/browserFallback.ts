// Non-Tauri fallback (vitest, or plain `vite dev` in a browser tab, with
// no __TAURI_INTERNALS__ on window): save triggers a normal browser
// download, open triggers a hidden <input type="file"> upload. Keeps the
// persistence module testable and usable outside the Tauri shell, mirroring
// the Tauri-vs-fallback split in src/engine/index.ts.

export function downloadNotebookFile(content: string, fileName: string): void {
  const blob = new Blob([content], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

export interface PickedFile {
  fileName: string;
  content: string;
}

/** Open a hidden file input, resolve with the chosen file's name and text
 * contents, or null if the user cancelled (dismissed the dialog without
 * choosing a file). */
export function pickNotebookFile(): Promise<PickedFile | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".omnb,application/json";
    input.style.display = "none";

    input.addEventListener("change", () => {
      const file = input.files?.[0];
      input.remove();

      if (!file) {
        resolve(null);
        return;
      }

      const reader = new FileReader();
      reader.onload = () => resolve({ fileName: file.name, content: String(reader.result ?? "") });
      reader.onerror = () => resolve(null);
      reader.readAsText(file);
    });

    document.body.appendChild(input);
    input.click();
  });
}
