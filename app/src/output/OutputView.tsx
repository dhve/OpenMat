import { useMemo } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";
import type { EvalResult } from "../engine/types";
import { Plot } from "../plot/Plot";
import "./OutputView.css";

interface OutputViewProps {
  result: EvalResult | null;
  status: "idle" | "evaluating" | "done" | "error";
}

function TypesetLatex({ latex }: { latex: string }) {
  const html = useMemo(() => {
    try {
      return katex.renderToString(latex, { throwOnError: false, displayMode: true });
    } catch {
      return null;
    }
  }, [latex]);

  if (html === null) {
    return <div className="output-error">Could not typeset the result.</div>;
  }
  // eslint-disable-next-line react/no-danger
  return <div className="output-latex" dangerouslySetInnerHTML={{ __html: html }} />;
}

export function OutputView({ result, status }: OutputViewProps) {
  if (status === "idle") return null;

  if (status === "evaluating") {
    return (
      <div className="output-cell">
        <div className="output-evaluating">Evaluating…</div>
      </div>
    );
  }

  if (!result) return null;

  if (result.error) {
    return (
      <div className="output-cell">
        <div className="output-error" role="alert">
          <span className="output-error-label">Error</span>
          <span className="output-error-message">{result.error}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="output-cell">
      {result.latex && <TypesetLatex latex={result.latex} />}
      {result.plot && (
        <div className="output-plot">
          <Plot curves={result.plot.curves} xRange={result.plot.x_range} yRange={result.plot.y_range} />
        </div>
      )}
    </div>
  );
}
