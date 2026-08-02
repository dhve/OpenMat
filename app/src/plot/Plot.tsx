import { useMemo } from "react";
import { niceTicks, formatTick } from "./ticks";
import { smoothPathD } from "./smoothPath";
import type { Curve } from "../engine/types";
import "./Plot.css";

interface PlotProps {
  curves: Curve[];
  xRange: [number, number];
  yRange: [number, number];
  xLabel?: string;
  yLabel?: string;
  width?: number;
  height?: number;
}

const CURVE_COLORS = ["var(--accent)", "var(--accent-2)", "var(--accent-3)"];

const MARGIN = { top: 18, right: 22, bottom: 40, left: 54 };

export function Plot({ curves, xRange, yRange, xLabel = "t", yLabel = "x", width = 640, height = 340 }: PlotProps) {
  const plotWidth = Math.max(1, width - MARGIN.left - MARGIN.right);
  const plotHeight = Math.max(1, height - MARGIN.top - MARGIN.bottom);

  const [xMin, xMax] = xRange;
  const [yMin, yMax] = yRange;
  const xSpan = xMax - xMin || 1;
  const ySpan = yMax - yMin || 1;

  const scaleX = (t: number) => MARGIN.left + ((t - xMin) / xSpan) * plotWidth;
  const scaleY = (v: number) => MARGIN.top + plotHeight - ((v - yMin) / ySpan) * plotHeight;

  const xTicks = useMemo(() => niceTicks(xMin, xMax, 6), [xMin, xMax]);
  const yTicks = useMemo(() => niceTicks(yMin, yMax, 5), [yMin, yMax]);

  const paths = useMemo(
    () =>
      curves.map((curve) => ({
        d: smoothPathD(curve.points.map(([t, v]) => ({ x: scaleX(t), y: scaleY(v) }))),
        label: curve.label,
      })),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [curves, xMin, xMax, yMin, yMax, plotWidth, plotHeight],
  );

  const showLegend = curves.length > 1 && curves.some((c) => c.label);

  return (
    <svg
      className="openmat-plot"
      viewBox={`0 0 ${width} ${height}`}
      width="100%"
      style={{ maxWidth: width }}
      role="img"
      aria-label={`Plot of ${curves.map((c) => c.label ?? "curve").join(", ")}`}
    >
      <rect x={0} y={0} width={width} height={height} className="plot-bg" />

      {/* Gridlines */}
      {xTicks.map((t) => (
        <line
          key={`gx-${t}`}
          x1={scaleX(t)}
          x2={scaleX(t)}
          y1={MARGIN.top}
          y2={MARGIN.top + plotHeight}
          className="plot-grid"
        />
      ))}
      {yTicks.map((v) => (
        <line
          key={`gy-${v}`}
          x1={MARGIN.left}
          x2={MARGIN.left + plotWidth}
          y1={scaleY(v)}
          y2={scaleY(v)}
          className="plot-grid"
        />
      ))}

      {/* Frame */}
      <rect x={MARGIN.left} y={MARGIN.top} width={plotWidth} height={plotHeight} className="plot-frame" />

      {/* Ticks + labels */}
      {xTicks.map((t) => (
        <g key={`tx-${t}`}>
          <line
            x1={scaleX(t)}
            x2={scaleX(t)}
            y1={MARGIN.top + plotHeight}
            y2={MARGIN.top + plotHeight + 5}
            className="plot-tick"
          />
          <text x={scaleX(t)} y={MARGIN.top + plotHeight + 19} className="plot-tick-label" textAnchor="middle">
            {formatTick(t)}
          </text>
        </g>
      ))}
      {yTicks.map((v) => (
        <g key={`ty-${v}`}>
          <line x1={MARGIN.left - 5} x2={MARGIN.left} y1={scaleY(v)} y2={scaleY(v)} className="plot-tick" />
          <text x={MARGIN.left - 9} y={scaleY(v) + 4} className="plot-tick-label" textAnchor="end">
            {formatTick(v)}
          </text>
        </g>
      ))}

      {/* Axis labels */}
      <text x={MARGIN.left + plotWidth / 2} y={height - 6} className="plot-axis-label" textAnchor="middle">
        {xLabel}
      </text>
      <text
        x={14}
        y={MARGIN.top + plotHeight / 2}
        className="plot-axis-label"
        textAnchor="middle"
        transform={`rotate(-90 14 ${MARGIN.top + plotHeight / 2})`}
      >
        {yLabel}
      </text>

      {/* Curves */}
      {paths.map((p, i) => (
        <path key={i} d={p.d} className="plot-curve" style={{ stroke: CURVE_COLORS[i % CURVE_COLORS.length] }} />
      ))}

      {/* Legend */}
      {showLegend && (
        <g transform={`translate(${MARGIN.left + plotWidth - 118}, ${MARGIN.top + 10})`}>
          <rect x={0} y={0} width={112} height={curves.length * 18 + 8} className="plot-legend-bg" />
          {curves.map((c, i) => (
            <g key={i} transform={`translate(8, ${14 + i * 18})`}>
              <line x1={0} y1={0} x2={16} y2={0} style={{ stroke: CURVE_COLORS[i % CURVE_COLORS.length] }} strokeWidth={2.5} />
              <text x={22} y={4} className="plot-legend-label">
                {c.label ?? `curve ${i + 1}`}
              </text>
            </g>
          ))}
        </g>
      )}
    </svg>
  );
}
