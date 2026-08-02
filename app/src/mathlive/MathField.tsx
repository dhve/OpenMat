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
  placeholder?: string;
  autoFocus?: boolean;
}

export interface MathFieldHandle {
  focus: () => void;
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
  { value, onChange, onEnter, onEvaluate, placeholder, autoFocus },
  ref,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mfRef = useRef<MathfieldElement | null>(null);

  useImperativeHandle(ref, () => ({
    focus: () => mfRef.current?.focus(),
  }));

  const onChangeRef = useRef(onChange);
  const onEnterRef = useRef(onEnter);
  const onEvaluateRef = useRef(onEvaluate);
  onChangeRef.current = onChange;
  onEnterRef.current = onEnter;
  onEvaluateRef.current = onEvaluate;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const mf = document.createElement("math-field") as MathfieldElement;
    mf.className = "openmat-mathfield";
    mf.value = value;
    mf.mathVirtualKeyboardPolicy = "manual";
    mf.smartFence = true;
    mf.smartSuperscript = true;
    if (placeholder) mf.setAttribute("placeholder", placeholder);

    const handleInput = () => onChangeRef.current(mf.value);

    // Registered with capture so it runs before MathLive's own internal key
    // handling (which lives on an element inside its shadow DOM), so
    // preventDefault reliably wins for the keys we intercept.
    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key !== "Enter") return;
      e.preventDefault();
      e.stopPropagation();
      if (e.shiftKey) {
        onEvaluateRef.current?.();
      } else {
        onEnterRef.current?.();
      }
    };

    mf.addEventListener("input", handleInput);
    mf.addEventListener("keydown", handleKeydown, { capture: true });
    container.appendChild(mf);
    mfRef.current = mf;

    if (autoFocus) {
      requestAnimationFrame(() => mf.focus());
    }

    return () => {
      mf.removeEventListener("input", handleInput);
      mf.removeEventListener("keydown", handleKeydown, { capture: true });
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
