let counter = 0;

/** A short, sufficiently-unique id for a notebook cell. */
export function nextCellId(): string {
  counter += 1;
  return `cell-${Date.now().toString(36)}-${counter}`;
}
