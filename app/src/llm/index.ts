// Natural language interpretation for free-form input cells (type = at the
// start of an empty cell): Notebook.tsx calls generateOpenMatCode with the
// request and parseGeneratedNotebook on the reply. Provider configuration
// (Anthropic key or local Ollama model) lives in the Settings dialog.
export { generateOpenMatCode, LlmGenerationError, buildLlmCompleteArgs } from "./generate";
export { parseGeneratedNotebook } from "./notebookSpec";
export type { GeneratedCellSpec, GeneratedInputCell, GeneratedTextCell } from "./notebookSpec";
export { stripCodeFences } from "./stripFences";
export { buildSystemPrompt } from "./systemPrompt";
export { GRAMMAR_SUMMARY } from "./grammar";
