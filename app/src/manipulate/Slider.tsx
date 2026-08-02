import { useEffect, useRef, useState } from "react";
import "./Slider.css";

interface SliderProps {
  name: string;
  label?: string;
  min: number;
  max: number;
  step: number;
  value: number;
  /** Called (debounced) with the new value while dragging, and immediately on release. */
  onChange: (value: number) => void;
  debounceMs?: number;
}

function decimalsForStep(step: number): number {
  const s = step.toString();
  const dot = s.indexOf(".");
  return dot === -1 ? 0 : s.length - dot - 1;
}

export function Slider({ name, label, min, max, step, value, onChange, debounceMs = 90 }: SliderProps) {
  // The displayed value updates on every input event so dragging feels
  // instant. The actual onChange (which triggers a re-evaluate) is
  // debounced so a fast drag does not flood the engine with requests.
  const [displayValue, setDisplayValue] = useState(value);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const decimals = decimalsForStep(step);

  useEffect(() => {
    setDisplayValue(value);
  }, [value]);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const handleInput = (next: number) => {
    setDisplayValue(next);
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => onChange(next), debounceMs);
  };

  const handleCommit = (next: number) => {
    if (timerRef.current) clearTimeout(timerRef.current);
    onChange(next);
  };

  return (
    <div className="manipulate-slider">
      <div className="manipulate-row">
        <label className="manipulate-label" htmlFor={`manipulate-${name}`}>
          {label ?? name}
        </label>
        <input
          id={`manipulate-${name}`}
          className="manipulate-range"
          type="range"
          min={min}
          max={max}
          step={step}
          value={displayValue}
          onChange={(e) => handleInput(parseFloat(e.target.value))}
          onPointerUp={(e) => handleCommit(parseFloat((e.target as HTMLInputElement).value))}
          onKeyUp={(e) => handleCommit(parseFloat((e.target as HTMLInputElement).value))}
        />
        <span className="manipulate-readout">{displayValue.toFixed(decimals)}</span>
      </div>
      <div className="manipulate-bounds">
        <span>{min}</span>
        <span>{max}</span>
      </div>
    </div>
  );
}
