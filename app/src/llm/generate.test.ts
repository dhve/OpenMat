import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { resetSettingsForTests, saveSettings } from "../settings/store";
import { buildLlmCompleteArgs, generateOpenMatCode, LlmGenerationError } from "./generate";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

// These tests exercise the desktop transport (the mocked Tauri invoke), so
// mark the environment as Tauri; without the flag generateOpenMatCode takes
// the direct-fetch browser path instead.
(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};

afterEach(() => {
  resetSettingsForTests();
  mockedInvoke.mockReset();
});

describe("buildLlmCompleteArgs", () => {
  it("builds anthropic args from saved settings", () => {
    saveSettings({ provider: "anthropic", anthropicModel: "claude-opus-5", anthropicApiKey: "sk-ant-test", ollamaModel: "" });

    const args = buildLlmCompleteArgs("integrate x^2");

    expect(args.provider).toBe("anthropic");
    expect(args.model).toBe("claude-opus-5");
    expect(args.apiKey).toBe("sk-ant-test");
    expect(args.prompt).toBe("integrate x^2");
    expect(args.system).toContain("Sin");
  });

  it("sends a null apiKey for anthropic when no key is saved", () => {
    saveSettings({ provider: "anthropic", anthropicModel: "claude-opus-5", anthropicApiKey: "", ollamaModel: "" });

    expect(buildLlmCompleteArgs("x").apiKey).toBeNull();
  });

  it("builds ollama args from saved settings, always with a null apiKey", () => {
    saveSettings({
      provider: "ollama",
      anthropicModel: "claude-opus-5",
      anthropicApiKey: "unused-for-ollama",
      ollamaModel: "llama3.2:1b",
    });

    const args = buildLlmCompleteArgs("differentiate x^3");

    expect(args.provider).toBe("ollama");
    expect(args.model).toBe("llama3.2:1b");
    expect(args.apiKey).toBeNull();
  });
});

describe("generateOpenMatCode", () => {
  it("calls llm_complete for the anthropic provider and strips fences from the reply", async () => {
    saveSettings({ provider: "anthropic", anthropicModel: "claude-opus-5", anthropicApiKey: "sk-ant-test", ollamaModel: "" });
    mockedInvoke.mockResolvedValueOnce("```\nD[x^2, x]\n```");

    const result = await generateOpenMatCode("derivative of x^2");

    expect(result).toBe("D[x^2, x]");
    expect(mockedInvoke).toHaveBeenCalledWith("llm_complete", {
      provider: "anthropic",
      model: "claude-opus-5",
      apiKey: "sk-ant-test",
      system: expect.stringContaining("Sin"),
      prompt: "derivative of x^2",
    });
  });

  it("calls llm_complete for the ollama provider and strips fences from the reply", async () => {
    saveSettings({ provider: "ollama", anthropicModel: "claude-opus-5", anthropicApiKey: "", ollamaModel: "qwen2.5:0.5b" });
    mockedInvoke.mockResolvedValueOnce("Integrate[x^2, x]");

    const result = await generateOpenMatCode("integral of x^2");

    expect(result).toBe("Integrate[x^2, x]");
    expect(mockedInvoke).toHaveBeenCalledWith("llm_complete", {
      provider: "ollama",
      model: "qwen2.5:0.5b",
      apiKey: null,
      system: expect.stringContaining("Sin"),
      prompt: "integral of x^2",
    });
  });

  it("surfaces a refusal error from the Rust side as an LlmGenerationError", async () => {
    saveSettings({ provider: "anthropic", anthropicModel: "claude-opus-5", anthropicApiKey: "sk-ant-test", ollamaModel: "" });
    mockedInvoke.mockRejectedValueOnce("Anthropic declined to respond to this request (refusal).");

    let caught: unknown;
    try {
      await generateOpenMatCode("something the model refuses");
    } catch (err) {
      caught = err;
    }

    expect(caught).toBeInstanceOf(LlmGenerationError);
    expect((caught as Error).message).toMatch(/refus/i);
  });

  it("surfaces an Ollama connection error from the Rust side as an LlmGenerationError", async () => {
    saveSettings({ provider: "ollama", anthropicModel: "claude-opus-5", anthropicApiKey: "", ollamaModel: "qwen2.5:0.5b" });
    mockedInvoke.mockRejectedValueOnce(
      "Could not connect to Ollama at localhost:11434. Ollama does not appear to be running - start it and try again.",
    );

    let caught: unknown;
    try {
      await generateOpenMatCode("x");
    } catch (err) {
      caught = err;
    }

    expect(caught).toBeInstanceOf(LlmGenerationError);
    expect((caught as Error).message).toMatch(/Ollama/);
  });

  it("never executes or evaluates the returned code, only returns it as a string", async () => {
    saveSettings({ provider: "anthropic", anthropicModel: "claude-opus-5", anthropicApiKey: "sk-ant-test", ollamaModel: "" });
    mockedInvoke.mockResolvedValueOnce("Plot[Sin[x], {x, 0, 10}]");

    const result = await generateOpenMatCode("plot sin");

    expect(typeof result).toBe("string");
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    expect(mockedInvoke).not.toHaveBeenCalledWith("evaluate", expect.anything());
  });
});
