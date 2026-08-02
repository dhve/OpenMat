import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import "mathlive";
import type { MathfieldElement } from "mathlive";
import "./MathField.css";

interface MathFieldProps {
  value: string;
  onChange: (latex: string) => void;
  /** Enter without a modifier: notebook cell navigation, not evaluation. */
  onEnter?: () => void;
  /** Shift+Enter: evaluate the cell. */
  onEvaluate?: () => void;
  /** The field gained focus (used to track which cell is "selected"). */
  onFocus?: () => void;
  /** Up arrow pressed with the caret already at the top of the field
   * (MathLive's moveUp had nowhere to go): move to the previous cell. */
  onNavigateUp?: () => void;
  /** Down arrow pressed with the caret already at the bottom of the field:
   * move to the next cell. */
  onNavigateDown?: () => void;
  placeholder?: string;
  autoFocus?: boolean;
}

export interface MathFieldHandle {
  /** Focuses the field. `position` places the caret at the very start or
   * end first, for Up/Down cell navigation landing somewhere sensible. */
  focus: (position?: "start" | "end") => void;
}

/**
 * A thin React wrapper around MathLive's <math-field> custom element.
 *
 * The element is created and managed imperatively rather than rendered
 * through JSX. MathLive owns a shadow DOM with its own internal key
 * handling, and syncing it through React's synthetic prop diffing is more
 * fragile than just setting DOM properties directly once and updating them
 * in effects. This is the same pattern React itself recommends for wrapping
 * complex custom elements.
 */
export const MathField = forwardRef<MathFieldHandle, MathFieldProps>(function MathField(
  { value, onChange, onEnter, onEvaluate, onFocus, onNavigateUp, onNavigateDown, placeholder, autoFocus },
  ref,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mfRef = useRef<MathfieldElement | null>(null);

  useImperativeHandle(ref, () => ({
    focus: (position) => {
      const mf = mfRef.current;
      if (!mf) return;
      if (position === "start") mf.executeCommand("moveToMathfieldStart");
      else if (position === "end") mf.executeCommand("moveToMathfieldEnd");
      mf.focus();
    },
  }));

  const onChangeRef = useRef(onChange);
  const onEnterRef = useRef(onEnter);
  const onEvaluateRef = useRef(onEvaluate);
  const onFocusRef = useRef(onFocus);
  const onNavigateUpRef = useRef(onNavigateUp);
  const onNavigateDownRef = useRef(onNavigateDown);
  onChangeRef.current = onChange;
  onEnterRef.current = onEnter;
  onEvaluateRef.current = onEvaluate;
  onFocusRef.current = onFocus;
  onNavigateUpRef.current = onNavigateUp;
  onNavigateDownRef.current = onNavigateDown;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const mf = document.createElement("math-field") as MathfieldElement;
    mf.className = "openmat-mathfield";
    mf.value = value;
    // Setting value can leave the whole expression selected, which reads as
    // highlighted (or invisible under dark-mode selection colors). Collapse
    // the selection to the end so a prefilled cell shows plain content.
    mf.executeCommand("moveToMathfieldEnd");
    mf.mathVirtualKeyboardPolicy = "manual";
    mf.smartFence = true;
    mf.smartSuperscript = true;
    if (placeholder) mf.setAttribute("placeholder", placeholder);

    const handleInput = () => onChangeRef.current(mf.value);
    const handleFocusIn = () => onFocusRef.current?.();

    // Registered with capture so it runs before MathLive's own internal key
    // handling (which lives on an element inside its shadow DOM), so
    // preventDefault reliably wins for the keys we intercept.
    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        e.stopPropagation();
        if (e.shiftKey) {
          onEvaluateRef.current?.();
        } else {
          onEnterRef.current?.();
        }
        return;
      }

      // Up/Down move within the field first (e.g. out of a fraction's
      // numerator); only once that has nowhere left to go do they move to
      // the previous/next cell. executeCommand returns false when the
      // command made no change, which is MathLive's own boundary signal.
      if (e.key === "ArrowUp" && onNavigateUpRef.current) {
        e.preventDefault();
        e.stopPropagation();
        if (!mf.executeCommand("moveUp")) onNavigateUpRef.current();
        return;
      }
      if (e.key === "ArrowDown" && onNavigateDownRef.current) {
        e.preventDefault();
        e.stopPropagation();
        if (!mf.executeCommand("moveDown")) onNavigateDownRef.current();
      }
    };

    mf.addEventListener("input", handleInput);
    mf.addEventListener("keydown", handleKeydown, { capture: true });
    // "focusin" bubbles through the shadow DOM boundary (composed: true), so
    // this fires whenever MathLive's internal editable surface is focused,
    // with no cooperation needed from the element itself.
    container.addEventListener("focusin", handleFocusIn);
    container.appendChild(mf);
    mfRef.current = mf;

    if (autoFocus) {
      requestAnimationFrame(() => mf.focus());
    }

    return () => {
      mf.removeEventListener("input", handleInput);
      mf.removeEventListener("keydown", handleKeydown, { capture: true });
      container.removeEventListener("focusin", handleFocusIn);
      if (container.contains(mf)) container.removeChild(mf);
      mfRef.current = null;
    };
    // Intentionally mount once: `value` is kept in sync by the effect below
    // so the field isn't torn down and recreated (and re-focused) on every
    // keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const mf = mfRef.current;
    if (mf && mf.value !== value) {
      mf.value = value;
    }
  }, [value]);

  return <div ref={containerRef} className="mathfield-host" />;
});
