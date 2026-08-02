// Transport for free-form natural language interpretation: on desktop the
// Rust llm_complete Tauri command (keys never touch page JS), in a plain
// browser a direct provider call. Strips markdown fences defensively; the
// reply is parsed by notebookSpec.ts, and generated code lands in ordinary
// editable cells before/while evaluating, never hidden.
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

function runningUnderTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Browser transport: the Tauri command is unavailable outside the desktop
 * shell, so the plain-browser build talks to the provider directly. Ollama
 * allows localhost origins by default; Anthropic allows browser calls when
 * the dangerous-direct-browser-access header acknowledges the key is the
 * user's own, entered locally. Mirrors src-tauri/src/llm.rs request shapes. */
async function completeDirect(args: LlmCompleteArgs): Promise<string> {
  if (args.provider === "ollama") {
    let res: Response;
    try {
      res = await fetch("http://localhost:11434/api/chat", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          model: args.model,
          stream: false,
          // Ollama's default num_predict truncates long replies mid-JSON; a
          // generated multi-cell notebook needs room to finish.
          options: { num_predict: 4096 },
          messages: [
            { role: "system", content: args.system },
            { role: "user", content: args.prompt },
          ],
        }),
      });
    } catch {
      throw new LlmGenerationError("Could not reach Ollama at localhost:11434. Is it running?");
    }
    if (!res.ok) throw new LlmGenerationError(`Ollama request failed (${res.status}).`);
    const data = (await res.json()) as { message?: { content?: string } };
    return data.message?.content ?? "";
  }

  if (!args.apiKey) {
    throw new LlmGenerationError("Natural language input needs a model: add an Anthropic API key or pick a local Ollama model in Settings.");
  }
  const res = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-api-key": args.apiKey,
      "anthropic-version": "2023-06-01",
      "anthropic-dangerous-direct-browser-access": "true",
    },
    body: JSON.stringify({
      model: args.model,
      max_tokens: 4096,
      system: args.system,
      messages: [{ role: "user", content: args.prompt }],
    }),
  });
  if (!res.ok) {
    let message = `The model request failed (${res.status}).`;
    try {
      const body = (await res.json()) as { error?: { message?: string } };
      if (body.error?.message) message = body.error.message;
    } catch {
      // keep the status-code message
    }
    throw new LlmGenerationError(message);
  }
  const data = (await res.json()) as { stop_reason?: string; content?: Array<{ type: string; text?: string }> };
  if (data.stop_reason === "refusal") {
    throw new LlmGenerationError("The model declined this request.");
  }
  return (data.content ?? [])
    .filter((block) => block.type === "text")
    .map((block) => block.text ?? "")
    .join("");
}

/**
 * Ask the configured provider to interpret `request`, an English
 * description, into notebook cells (see systemPrompt.ts for the reply
 * contract). Throws LlmGenerationError (wrapping whatever the transport
 * returned, e.g. a refusal or a connection error) rather than executing or
 * silently swallowing failures.
 */
export async function generateOpenMatCode(request: string): Promise<string> {
  const args = buildLlmCompleteArgs(request);

  let raw: string;
  if (runningUnderTauri()) {
    try {
      raw = await invoke<string>("llm_complete", { ...args });
    } catch (err) {
      throw new LlmGenerationError(describeInvokeError(err));
    }
  } else {
    raw = await completeDirect(args);
  }

  return stripCodeFences(raw);
}
