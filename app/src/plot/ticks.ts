// Nice tick placement using the classic 1-2-5 ladder: pick a step size that
// is 1, 2, or 5 times a power of ten, closest to (but not below) the size
// implied by the target tick count. This is what gives axis labels like
// 0, 5, 10, 15, 20 instead of 0, 4.3, 8.6, ...

const LADDER = [1, 2, 5, 10];

export function niceStep(range: number, targetCount: number): number {
  if (range <= 0) return 1;
  const rawStep = range / Math.max(1, targetCount);
  const magnitude = Math.pow(10, Math.floor(Math.log10(rawStep)));
  const normalized = rawStep / magnitude;
  const niceNormalized = LADDER.find((v) => v >= normalized) ?? 10;
  return niceNormalized * magnitude;
}

/** Tick values within [min, max], spaced using the 1-2-5 ladder. */
export function niceTicks(min: number, max: number, targetCount = 5): number[] {
  if (!isFinite(min) || !isFinite(max) || min > max) return [];
  if (min === max) return [min];

  const step = niceStep(max - min, targetCount);
  const ticks: number[] = [];
  // Round the start up to the nearest multiple of step, guarding against
  // floating point noise (e.g. 0.1 + 0.2 landing just above a tick).
  const epsilon = step * 1e-9;
  let first = Math.ceil((min - epsilon) / step) * step;
  for (let v = first; v <= max + epsilon; v += step) {
    // Snap away from floating point drift like 1.9999999999999998.
    const snapped = Math.round(v / step) * step;
    ticks.push(Math.abs(snapped) < step * 1e-9 ? 0 : snapped);
  }
  return ticks;
}

/** Format a tick value without noisy trailing decimals. */
export function formatTick(value: number): string {
  const rounded = Math.round(value * 1e9) / 1e9;
  return rounded.toString();
}
