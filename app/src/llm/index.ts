// Importing this module registers window.__openmat_askai as a side
// effect, so any part of the app can open the natural-language-input pane
// without a direct import cycle (e.g. a toolbar button owned by another
// workstream).
import { openAskAi } from "./AskAiModal";

declare global {
  interface Window {
    __openmat_askai?: () => void;
    // Provided by the notebook workstream. Ask AI hands it the generated
    // code as plain text for the notebook to insert as an editable cell;
    // it never executes the code itself.
    __openmat_insert_cell?: (code: string) => void;
  }
}

window.__openmat_askai = openAskAi;

export { openAskAi };
export { generateOpenMatCode, LlmGenerationError, buildLlmCompleteArgs } from "./generate";
export { stripCodeFences } from "./stripFences";
export { buildSystemPrompt } from "./systemPrompt";
export { GRAMMAR_SUMMARY } from "./grammar";
