// Calls the Rust llm_complete Tauri command with whichever provider is
// currently configured in settings, and defensively strips markdown fences
// from the reply. Never executes the returned code; callers are
// responsible for showing it in an editable preview first (see
// AskAiModal.tsx) and only inserting it into a cell on explicit user
// action.
import { invoke } from "@tauri-apps/api/core";
import { loadSettings } from "../settings/store";
import { buildSystemPrompt } from "./systemPrompt";
import { stripCodeFences } from "./stripFences";

export class LlmGenerationError extends Error {}

export interface LlmCompleteArgs {
  provider: string;
  model: string;
  apiKey: string | null;
  system: string;
  prompt: string;
}

/**
 * Build the exact argument object that would be sent to the `llm_complete`
 * Tauri command for the currently saved settings and a given request.
 * Exported separately from generateOpenMatCode so provider dispatch is
 * testable without mocking invoke.
 */
export function buildLlmCompleteArgs(request: string): LlmCompleteArgs {
  const settings = loadSettings();
  const system = buildSystemPrompt();

  if (settings.provider === "ollama") {
    return { provider: "ollama", model: settings.ollamaModel, apiKey: null, system, prompt: request };
  }

  return {
    provider: "anthropic",
    model: settings.anthropicModel,
    apiKey: settings.anthropicApiKey.trim() === "" ? null : settings.anthropicApiKey,
    system,
    prompt: request,
  };
}

function describeInvokeError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "The model request failed.";
}

/**
 * Ask the configured LLM provider to translate `request`, an English
 * description of a math problem, into OpenMat linear syntax. Throws
 * LlmGenerationError (wrapping whatever llm_complete on the Rust side
 * returned, e.g. a refusal or a connection error) rather than executing or
 * silently swallowing failures.
 */
export async function generateOpenMatCode(request: string): Promise<string> {
  const args = buildLlmCompleteArgs(request);

  let raw: string;
  try {
    raw = await invoke<string>("llm_complete", { ...args });
  } catch (err) {
    throw new LlmGenerationError(describeInvokeError(err));
  }

  return stripCodeFences(raw);
}
