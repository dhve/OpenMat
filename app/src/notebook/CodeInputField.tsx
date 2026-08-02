import { forwardRef, useImperativeHandle, useLayoutEffect, useRef } from "react";
import "./CodeInputField.css";

interface CodeInputFieldProps {
  value: string;
  autoFocus?: boolean;
  placeholder?: string;
  onChange: (value: string) => void;
  onFocus?: () => void;
  /** Shift+Enter: evaluate the cell. */
  onEvaluate?: () => void;
  /** Enter: no evaluation, just move to (or create) the next cell. */
  onCommit?: () => void;
  onNavigateUp?: () => void;
  onNavigateDown?: () => void;
}

export interface CodeInputHandle {
  focus: (position?: "start" | "end") => void;
}

/**
 * A plain monospace textarea for Input cells whose source is already WL
 * linear syntax (InputCell.sourceKind === "linear": see notebook/types.ts),
 * used instead of MathField for those cells.
 *
 * MathLive's <math-field> is a LaTeX editor: curly braces are invisible
 * grouping syntax there, not literal glyphs, so WL's `{a, b, c}` lists and
 * `{t, 0, 20}` iterator specs (which appear throughout the grammar: Table,
 * NDSolve, Integrate, ...) would silently lose their braces on a LaTeX
 * round trip. A plain textarea has no such interpretation: it shows, and
 * keeps, exactly the characters the model generated.
 */
export const CodeInputField = forwardRef<CodeInputHandle, CodeInputFieldProps>(function CodeInputField(
  { value, autoFocus, placeholder, onChange, onFocus, onEvaluate, onCommit, onNavigateUp, onNavigateDown },
  ref,
) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useImperativeHandle(ref, () => ({
    focus: (position) => {
      const el = textareaRef.current;
      if (!el) return;
      el.focus();
      const offset = position === "start" ? 0 : el.value.length;
      el.setSelectionRange(offset, offset);
    },
  }));

  // Grows with content instead of scrolling internally. Recomputed from
  // `value` itself, not just on keystrokes, so a cell inserted or loaded
  // programmatically (window.__openmat_insert_cell, __openmat_set_notebook)
  // is sized correctly right away.
  useLayoutEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [value]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) onEvaluate?.();
      else onCommit?.();
      return;
    }
    if (e.key === "ArrowUp") {
      const before = e.currentTarget.value.slice(0, e.currentTarget.selectionStart ?? 0);
      if (!before.includes("\n")) {
        e.preventDefault();
        onNavigateUp?.();
      }
      return;
    }
    if (e.key === "ArrowDown") {
      const after = e.currentTarget.value.slice(e.currentTarget.selectionEnd ?? e.currentTarget.value.length);
      if (!after.includes("\n")) {
        e.preventDefault();
        onNavigateDown?.();
      }
    }
  };

  return (
    <textarea
      ref={textareaRef}
      className="code-input-field"
      value={value}
      placeholder={placeholder}
      rows={1}
      autoFocus={autoFocus}
      onChange={(e) => onChange(e.target.value)}
      onFocus={() => onFocus?.()}
      onKeyDown={handleKeyDown}
    />
  );
});
