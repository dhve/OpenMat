import { afterEach, describe, expect, it } from "vitest";
import { DEFAULT_ANTHROPIC_MODEL, loadSettings, resetSettingsForTests, saveSettings, writeRawSettingsForTests } from "./store";

afterEach(() => {
  resetSettingsForTests();
});

describe("loadSettings", () => {
  it("returns anthropic defaults when nothing has been saved", () => {
    const settings = loadSettings();
    expect(settings.provider).toBe("anthropic");
    expect(settings.anthropicModel).toBe(DEFAULT_ANTHROPIC_MODEL);
    expect(settings.anthropicApiKey).toBe("");
    expect(settings.ollamaModel).toBe("");
  });

  it("round-trips a saved anthropic configuration", () => {
    saveSettings({
      provider: "anthropic",
      anthropicModel: "claude-opus-5",
      anthropicApiKey: "sk-ant-test-key",
      ollamaModel: "",
    });
    expect(loadSettings()).toEqual({
      provider: "anthropic",
      anthropicModel: "claude-opus-5",
      anthropicApiKey: "sk-ant-test-key",
      ollamaModel: "",
    });
  });

  it("round-trips a saved ollama configuration", () => {
    saveSettings({
      provider: "ollama",
      anthropicModel: DEFAULT_ANTHROPIC_MODEL,
      anthropicApiKey: "",
      ollamaModel: "llama3.2:1b",
    });
    expect(loadSettings()).toEqual({
      provider: "ollama",
      anthropicModel: DEFAULT_ANTHROPIC_MODEL,
      anthropicApiKey: "",
      ollamaModel: "llama3.2:1b",
    });
  });

  it("falls back to defaults when the stored value is malformed JSON", () => {
    writeRawSettingsForTests("{not json");
    expect(loadSettings()).toEqual({
      provider: "anthropic",
      anthropicModel: DEFAULT_ANTHROPIC_MODEL,
      anthropicApiKey: "",
      ollamaModel: "",
    });
  });

  it("falls back field by field when the stored value has the wrong shape", () => {
    writeRawSettingsForTests(JSON.stringify({ provider: "carrier-pigeon", anthropicModel: 42 }));
    const settings = loadSettings();
    expect(settings.provider).toBe("anthropic");
    expect(settings.anthropicModel).toBe(DEFAULT_ANTHROPIC_MODEL);
  });
});
