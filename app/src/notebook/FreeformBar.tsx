import { useState } from "react";
import "./FreeformBar.css";

interface FreeformBarProps {
  /** Interpret one plain-language request. Resolves when the generated
   * cells have been inserted and evaluated; the bar disables itself while
   * this runs so requests cannot interleave. */
  onSubmit: (request: string) => Promise<void>;
}

/**
 * The always-visible natural language box, docked at the bottom of the
 * window: type what you want shown or created ("plot sin x from 0 to 10",
 * "make me a notebook exploring the damped harmonic oscillator") and the
 * cells appear in the notebook. The same interpreter also lives inline:
 * typing = at the start of an empty cell makes that cell free-form, and
 * submitting here records the request as exactly such a cell, so the
 * notebook keeps a visible trace of what was asked.
 */
export function FreeformBar({ onSubmit }: FreeformBarProps) {
  const [request, setRequest] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    const trimmed = request.trim();
    if (trimmed === "" || busy) return;
    setBusy(true);
    setRequest("");
    try {
      await onSubmit(trimmed);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="freeform-bar">
      <div className="freeform-bar-inner">
        <span className="freeform-bar-marker" aria-hidden="true">
          =
        </span>
        <input
          type="text"
          className="freeform-bar-input"
          value={request}
          disabled={busy}
          placeholder={busy ? "Working…" : "Ask in plain language: plot sin x from 0 to 10, make a notebook about…"}
          aria-label="Natural language input"
          onChange={(e) => setRequest(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void submit();
            }
          }}
        />
        <button type="button" className="freeform-bar-go" disabled={busy || request.trim() === ""} onClick={() => void submit()}>
          {busy ? "…" : "Create"}
        </button>
      </div>
    </div>
  );
}
