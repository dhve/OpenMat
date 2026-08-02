// Importing this module registers window.__openmat_settings as a side
// effect, so any part of the app can open the LLM settings pane without a
// direct import cycle (e.g. a menu button owned by another workstream).
import { openSettings } from "./SettingsModal";

declare global {
  interface Window {
    __openmat_settings?: () => void;
  }
}

window.__openmat_settings = openSettings;

export { openSettings };
export type { LlmProvider, LlmSettings } from "./store";
export { loadSettings, saveSettings, DEFAULT_ANTHROPIC_MODEL } from "./store";
