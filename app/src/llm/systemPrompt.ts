// System prompt for the natural-language-to-OpenMat translation feature.
// Split out from generate.ts so prompt construction is directly testable
// (see systemPrompt.test.ts) without mocking the Tauri bridge.
import { GRAMMAR_SUMMARY } from "./grammar";

export function buildSystemPrompt(): string {
  return `You translate an English description of a math problem into OpenMat's linear syntax, a Wolfram-Language-like notation.

${GRAMMAR_SUMMARY}

Output ONLY the OpenMat code for the request: no prose, no explanation, no markdown code fences, no leading phrase like "Here is the code:". Your entire response must be exactly one OpenMat expression or statement, ready to paste directly into a notebook cell.`;
}
