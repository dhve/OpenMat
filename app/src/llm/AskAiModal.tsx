// Natural-language input: describe a math problem in English, get back
// OpenMat code in an editable preview, and insert it into a cell only on
// explicit user action. Generated code is NEVER auto-executed here or
// anywhere in this module; window.__openmat_insert_cell (owned by the
// notebook workstream) is expected to place it as ordinary editable cell
// content, not run it.
import { useState } from "react";
import { generateOpenMatCode, LlmGenerationError } from "./generate";
import { mountModal } from "./modalHost";
import "./AskAiModal.css";

type Status = "editing" | "loading" | "error" | "preview";

interface AskAiModalProps {
  onClose: () => void;
}

function AskAiModal({ onClose }: AskAiModalProps) {
  const [request, setRequest] = useState("");
  const [status, setStatus] = useState<Status>("editing");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [previewCode, setPreviewCode] = useState("");

  const handleGenerate = async () => {
    if (request.trim() === "" || status === "loading") return;
    setStatus("loading");
    setErrorMessage(null);
    try {
      const code = await generateOpenMatCode(request.trim());
      setPreviewCode(code);
      setStatus("preview");
    } catch (err) {
      setErrorMessage(err instanceof LlmGenerationError ? err.message : "The model request failed.");
      setStatus("error");
    }
  };

  const handleDiscard = () => {
    setPreviewCode("");
    setErrorMessage(null);
    setStatus("editing");
  };

  const handleInsert = () => {
    window.__openmat_insert_cell?.(previewCode);
    onClose();
  };

  return (
    <div className="askai-overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="askai-modal" role="dialog" aria-modal="true" aria-label="Ask AI">
        <div className="askai-header">
          <h2>Ask AI</h2>
          <button type="button" className="askai-close" aria-label="Close" onClick={onClose}>
            x
          </button>
        </div>

        {status !== "preview" && (
          <>
            <p className="askai-hint">Describe a math problem in English. The generated code is never run automatically, you review it first.</p>
            <textarea
              className="askai-textarea"
              placeholder="e.g. the derivative of sin(x) times x squared"
              value={request}
              autoFocus
              disabled={status === "loading"}
              onChange={(e) => setRequest(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault();
                  void handleGenerate();
                }
              }}
            />
            {status === "error" && errorMessage && <p className="askai-status askai-status-error">{errorMessage}</p>}
            <div className="askai-footer">
              <button type="button" className="askai-button askai-button-secondary" onClick={onClose}>
                Cancel
              </button>
              <button
                type="button"
                className="askai-button askai-button-primary"
                onClick={() => void handleGenerate()}
                disabled={request.trim() === "" || status === "loading"}
              >
                {status === "loading" ? "Generating..." : "Generate"}
              </button>
            </div>
          </>
        )}

        {status === "preview" && (
          <>
            <p className="askai-hint">Review and edit the generated code, then insert it as a new cell. Nothing is evaluated yet.</p>
            <textarea
              className="askai-textarea askai-textarea-code"
              value={previewCode}
              autoFocus
              onChange={(e) => setPreviewCode(e.target.value)}
            />
            <div className="askai-footer">
              <button type="button" className="askai-button askai-button-secondary" onClick={handleDiscard}>
                Discard
              </button>
              <button type="button" className="askai-button askai-button-primary" onClick={handleInsert} disabled={previewCode.trim() === ""}>
                Insert
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/** Open the Ask AI modal. Self-registered on window.__openmat_askai by ./index.ts. */
export function openAskAi(): void {
  mountModal((close) => <AskAiModal onClose={close} />);
}

export { AskAiModal };
