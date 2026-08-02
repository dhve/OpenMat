import { useRef, useState } from "react";
import "./FreeformBar.css";

interface FreeformBarProps {
  /** Interpret one plain-language request. Resolves when the generated
   * cells have been inserted and evaluated; the bar disables itself while
   * this runs so requests cannot interleave. */
  onSubmit: (request: string) => Promise<void>;
}

/** Symbols worth one click when describing math in plain language. The
 * interpreter reads the unicode directly. */
const SYMBOLS = ["π", "∞", "²", "³", "√", "∫", "Σ", "≤", "≥", "≠", "×", "÷", "→", "α", "β", "γ", "θ", "λ", "ω", "φ", "Δ", "°"];

/**
 * The always-visible natural language box, docked at the bottom of the
 * window: type what you want shown or created ("plot sin x from 0 to 10",
 * "make me a notebook exploring the damped harmonic oscillator") and the
 * cells appear in the notebook. The same interpreter also lives inline:
 * typing = at the start of an empty cell makes that cell free-form, and
 * submitting here records the request as exactly such a cell, so the
 * notebook keeps a visible trace of what was asked. The π button opens a
 * palette of math symbols that insert at the caret; math cells have their
 * own full symbol keyboard via MathLive's keyboard toggle.
 */
export function FreeformBar({ onSubmit }: FreeformBarProps) {
  const [request, setRequest] = useState("");
  const [busy, setBusy] = useState(false);
  const [showSymbols, setShowSymbols] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const submit = async () => {
    const trimmed = request.trim();
    if (trimmed === "" || busy) return;
    setBusy(true);
    setRequest("");
    setShowSymbols(false);
    try {
      await onSubmit(trimmed);
    } finally {
      setBusy(false);
    }
  };

  const insertSymbol = (symbol: string) => {
    const input = inputRef.current;
    const start = input?.selectionStart ?? request.length;
    const end = input?.selectionEnd ?? request.length;
    const next = request.slice(0, start) + symbol + request.slice(end);
    setRequest(next);
    requestAnimationFrame(() => {
      input?.focus();
      input?.setSelectionRange(start + symbol.length, start + symbol.length);
    });
  };

  return (
    <div className="freeform-bar">
      <div className="freeform-bar-stack">
        {showSymbols && (
          <div className="freeform-bar-symbols" role="toolbar" aria-label="Math symbols">
            {SYMBOLS.map((symbol) => (
              <button key={symbol} type="button" className="freeform-bar-symbol" onClick={() => insertSymbol(symbol)}>
                {symbol}
              </button>
            ))}
          </div>
        )}
        <div className="freeform-bar-inner">
          <span className="freeform-bar-marker" aria-hidden="true">
            =
          </span>
          <input
            ref={inputRef}
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
          <button
            type="button"
            className={`freeform-bar-symbols-toggle${showSymbols ? " freeform-bar-symbols-toggle-open" : ""}`}
            aria-label="Math symbols"
            aria-pressed={showSymbols}
            title="Math symbols"
            onClick={() => setShowSymbols((open) => !open)}
          >
            π
          </button>
          <button type="button" className="freeform-bar-go" disabled={busy || request.trim() === ""} onClick={() => void submit()}>
            {busy ? "…" : "Create"}
          </button>
        </div>
      </div>
    </div>
  );
}
