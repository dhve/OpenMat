import { forwardRef, useImperativeHandle, useLayoutEffect, useRef } from "react";
import type { TextCellData } from "./types";
import "./TextCellView.css";

interface TextCellViewProps {
  cell: TextCellData;
  autoFocus?: boolean;
  onChange: (text: string) => void;
  onFocus?: () => void;
  /** Commit and move on: Enter for Title/Section (single line), Shift+Enter
   * for Text (multi-line; plain Enter there just inserts a newline). */
  onCommit?: () => void;
  onNavigateUp?: () => void;
  onNavigateDown?: () => void;
}

export interface TextCellHandle {
  focus: (position?: "start" | "end") => void;
}

const PLACEHOLDER: Record<TextCellData["kind"], string> = {
  title: "Untitled",
  section: "Section",
  text: "Text",
};

/**
 * The editable Title/Section/Text cell. A single <textarea> styled per
 * `cell.kind` (see TextCellView.css) rather than a rich-text editor: plain,
 * predictable, and consistent with the rest of this codebase's "no more
 * machinery than the demo needs" approach.
 */
export const TextCellView = forwardRef<TextCellHandle, TextCellViewProps>(function TextCellView(
  { cell, autoFocus, onChange, onFocus, onCommit, onNavigateUp, onNavigateDown },
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

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter") {
      // A Text cell is free-form and multi-line: a plain Enter inserts a
      // newline, only Shift+Enter commits and moves on. Title/Section are
      // one-liners, so either Enter or Shift+Enter commits.
      if (cell.kind === "text" && !e.shiftKey) return;
      e.preventDefault();
      onCommit?.();
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

  // Grows with content instead of scrolling internally, like Mathematica's
  // text cells: no fixed height to fight with. Recomputed from `cell.text`
  // itself (not just on user keystrokes) so a cell loaded programmatically,
  // e.g. via window.__openmat_set_notebook, is sized correctly right away
  // instead of only after the user next types into it.
  useLayoutEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [cell.text]);

  return (
    <textarea
      ref={textareaRef}
      className={`text-cell-field text-cell-field-${cell.kind}`}
      value={cell.text}
      placeholder={PLACEHOLDER[cell.kind]}
      rows={1}
      autoFocus={autoFocus}
      onChange={(e) => onChange(e.target.value)}
      onFocus={() => onFocus?.()}
      onKeyDown={handleKeyDown}
    />
  );
});
