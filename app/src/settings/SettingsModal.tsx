// LLM provider settings: pick Anthropic (bring your own API key) or a
// local Ollama install, and confirm the connection works before leaving
// the pane. See ./store.ts for what gets persisted and why localStorage is
// not a real credential store.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { mountModal } from "./modalHost";
import { DEFAULT_ANTHROPIC_MODEL, loadSettings, saveSettings, type LlmProvider, type LlmSettings } from "./store";
import "./SettingsModal.css";

type TestStatus = "idle" | "testing" | "success" | "error";

interface SettingsModalProps {
  onClose: () => void;
}

function describeError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "Something went wrong.";
}

function SettingsModal({ onClose }: SettingsModalProps) {
  const [settings, setSettings] = useState<LlmSettings>(() => loadSettings());
  const [ollamaModels, setOllamaModels] = useState<string[]>([]);
  const [ollamaModelsError, setOllamaModelsError] = useState<string | null>(null);
  const [ollamaModelsLoading, setOllamaModelsLoading] = useState(false);
  const [testStatus, setTestStatus] = useState<TestStatus>("idle");
  const [testMessage, setTestMessage] = useState<string | null>(null);

  const refreshOllamaModels = async () => {
    setOllamaModelsLoading(true);
    setOllamaModelsError(null);
    try {
      const models = await invoke<string[]>("llm_list_ollama_models");
      setOllamaModels(models);
      setSettings((prev) => (prev.ollamaModel || models.length === 0 ? prev : { ...prev, ollamaModel: models[0] }));
    } catch (err) {
      setOllamaModelsError(describeError(err));
    } finally {
      setOllamaModelsLoading(false);
    }
  };

  useEffect(() => {
    void refreshOllamaModels();
    // Load once on open; the refresh button covers later refreshes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const setProvider = (provider: LlmProvider) => setSettings((prev) => ({ ...prev, provider }));

  const handleSave = () => {
    saveSettings(settings);
    onClose();
  };

  const handleTestConnection = async () => {
    setTestStatus("testing");
    setTestMessage(null);
    try {
      const args =
        settings.provider === "anthropic"
          ? {
              provider: "anthropic",
              model: settings.anthropicModel || DEFAULT_ANTHROPIC_MODEL,
              apiKey: settings.anthropicApiKey.trim() === "" ? null : settings.anthropicApiKey,
              system: "",
              prompt: "Reply with the single word OK and nothing else.",
            }
          : {
              provider: "ollama",
              model: settings.ollamaModel,
              apiKey: null,
              system: "",
              prompt: "Reply with the single word OK and nothing else.",
            };
      const reply = await invoke<string>("llm_complete", args);
      setTestStatus("success");
      setTestMessage(reply.trim() === "" ? "Connected. The model returned an empty reply." : `Connected. Model said: ${reply.trim()}`);
    } catch (err) {
      setTestStatus("error");
      setTestMessage(describeError(err));
    }
  };

  const canTest =
    settings.provider === "anthropic"
      ? settings.anthropicApiKey.trim() !== "" && settings.anthropicModel.trim() !== ""
      : settings.ollamaModel.trim() !== "";

  return (
    <div className="settings-overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="settings-modal" role="dialog" aria-modal="true" aria-label="LLM provider settings">
        <div className="settings-header">
          <h2>LLM Provider Settings</h2>
          <button type="button" className="settings-close" aria-label="Close settings" onClick={onClose}>
            x
          </button>
        </div>

        <fieldset className="settings-field">
          <legend>Provider</legend>
          <label className="settings-radio">
            <input
              type="radio"
              name="openmat-llm-provider"
              checked={settings.provider === "anthropic"}
              onChange={() => setProvider("anthropic")}
            />
            Anthropic API
          </label>
          <label className="settings-radio">
            <input
              type="radio"
              name="openmat-llm-provider"
              checked={settings.provider === "ollama"}
              onChange={() => setProvider("ollama")}
            />
            Ollama (local)
          </label>
        </fieldset>

        {settings.provider === "anthropic" ? (
          <div className="settings-section">
            <label className="settings-label" htmlFor="openmat-anthropic-model">
              Model
            </label>
            <input
              id="openmat-anthropic-model"
              type="text"
              className="settings-input"
              value={settings.anthropicModel}
              placeholder={DEFAULT_ANTHROPIC_MODEL}
              onChange={(e) => setSettings((prev) => ({ ...prev, anthropicModel: e.target.value }))}
            />

            <label className="settings-label" htmlFor="openmat-anthropic-key">
              API key
            </label>
            <input
              id="openmat-anthropic-key"
              type="password"
              className="settings-input"
              autoComplete="off"
              value={settings.anthropicApiKey}
              placeholder="sk-ant-..."
              onChange={(e) => setSettings((prev) => ({ ...prev, anthropicApiKey: e.target.value }))}
            />
          </div>
        ) : (
          <div className="settings-section">
            <label className="settings-label" htmlFor="openmat-ollama-model">
              Model
            </label>
            <div className="settings-row">
              <select
                id="openmat-ollama-model"
                className="settings-input"
                value={settings.ollamaModel}
                onChange={(e) => setSettings((prev) => ({ ...prev, ollamaModel: e.target.value }))}
              >
                <option value="" disabled>
                  {ollamaModelsLoading ? "Loading models..." : "Select a model"}
                </option>
                {ollamaModels.map((name) => (
                  <option key={name} value={name}>
                    {name}
                  </option>
                ))}
              </select>
              <button
                type="button"
                className="settings-button settings-button-secondary"
                onClick={() => void refreshOllamaModels()}
                disabled={ollamaModelsLoading}
              >
                Refresh
              </button>
            </div>
            {ollamaModelsError && <p className="settings-status settings-status-error">{ollamaModelsError}</p>}
          </div>
        )}

        <div className="settings-section settings-test">
          <button
            type="button"
            className="settings-button settings-button-secondary"
            onClick={() => void handleTestConnection()}
            disabled={!canTest || testStatus === "testing"}
          >
            {testStatus === "testing" ? "Testing..." : "Test connection"}
          </button>
          {testMessage && (
            <p className={`settings-status ${testStatus === "error" ? "settings-status-error" : "settings-status-success"}`}>
              {testMessage}
            </p>
          )}
        </div>

        <div className="settings-footer">
          <button type="button" className="settings-button settings-button-secondary" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="settings-button settings-button-primary" onClick={handleSave}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

/** Open the settings modal. Self-registered on window.__openmat_settings by ./index.ts. */
export function openSettings(): void {
  mountModal((close) => <SettingsModal onClose={close} />);
}

export { SettingsModal };
