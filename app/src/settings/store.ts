// Persistence for the LLM provider configuration (see ../llm/ for the
// natural-language-input feature this configures).
//
// SECURITY NOTE: localStorage is not secure credential storage. The
// Anthropic API key entered in the settings pane is stored here in
// plaintext, readable by any script running in this webview and by anyone
// with access to the browser profile on disk. A proper OS keychain
// integration (e.g. a Tauri keyring plugin) is future work; this is a
// stopgap so the key survives across app restarts during development.

export type LlmProvider = "anthropic" | "ollama";

export interface LlmSettings {
  provider: LlmProvider;
  anthropicModel: string;
  anthropicApiKey: string;
  ollamaModel: string;
}

const STORAGE_KEY = "openmat.llm.settings";

export const DEFAULT_ANTHROPIC_MODEL = "claude-opus-5";

const DEFAULT_SETTINGS: LlmSettings = {
  provider: "anthropic",
  anthropicModel: DEFAULT_ANTHROPIC_MODEL,
  anthropicApiKey: "",
  ollamaModel: "",
};

interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

// Tauri's webview provides a working localStorage. Some non-browser test
// runners do not: in particular, Node's own built-in Web Storage global can
// shadow the jsdom-provided one and expose a localStorage object whose
// methods are missing or inert rather than throwing outright. Feature-check
// it and fall back to an in-memory store rather than assume it works, so
// settings persistence degrades gracefully instead of crashing wherever
// localStorage happens to be unusable (private browsing, this Node quirk,
// or an opaque-origin document that throws on access).
function createMemoryStore(): KeyValueStore {
  const data = new Map<string, string>();
  return {
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => void data.set(key, value),
    removeItem: (key) => void data.delete(key),
  };
}

const memoryStoreFallback = createMemoryStore();

function resolveStore(): KeyValueStore {
  try {
    const candidate = globalThis.localStorage as KeyValueStore | undefined;
    if (candidate && typeof candidate.getItem === "function" && typeof candidate.setItem === "function") {
      return candidate;
    }
  } catch {
    // Accessing localStorage itself can throw; fall through below.
  }
  return memoryStoreFallback;
}

function coerceSettings(value: unknown): LlmSettings {
  if (typeof value !== "object" || value === null) {
    return { ...DEFAULT_SETTINGS };
  }
  const raw = value as Record<string, unknown>;
  return {
    provider: raw.provider === "ollama" ? "ollama" : "anthropic",
    anthropicModel:
      typeof raw.anthropicModel === "string" && raw.anthropicModel.trim() !== ""
        ? raw.anthropicModel
        : DEFAULT_ANTHROPIC_MODEL,
    anthropicApiKey: typeof raw.anthropicApiKey === "string" ? raw.anthropicApiKey : "",
    ollamaModel: typeof raw.ollamaModel === "string" ? raw.ollamaModel : "",
  };
}

/** Read the persisted settings, falling back to defaults on missing or malformed data. */
export function loadSettings(): LlmSettings {
  try {
    const raw = resolveStore().getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    return coerceSettings(JSON.parse(raw));
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

/** Persist settings. Callers own validation; this only serializes. */
export function saveSettings(settings: LlmSettings): void {
  resolveStore().setItem(STORAGE_KEY, JSON.stringify(settings));
}

/** Test-only: clear whatever storage backend (real or in-memory fallback) is in use. */
export function resetSettingsForTests(): void {
  try {
    resolveStore().removeItem(STORAGE_KEY);
  } catch {
    // Best-effort; nothing sensible to do if even removal throws.
  }
}

/** Test-only: write a raw string under the settings key, to exercise malformed-data handling. */
export function writeRawSettingsForTests(raw: string): void {
  resolveStore().setItem(STORAGE_KEY, raw);
}
