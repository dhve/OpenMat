interface InsertBarProps {
  onInsert: () => void;
  /** The trailing bar at the very end of the notebook stays visible so
   * there is always an obvious way to add a cell, not just on hover. */
  alwaysVisible?: boolean;
}

/**
 * The thin strip between two cells that, on hover, shows a line-and-plus
 * affordance for inserting a new Input cell at that spot (Mathematica-style
 * "insert cell here" bar).
 */
export function InsertBar({ onInsert, alwaysVisible }: InsertBarProps) {
  return (
    <div className={`insert-bar${alwaysVisible ? " insert-bar-always" : ""}`}>
      <button type="button" className="insert-bar-button" onClick={onInsert} aria-label="Insert cell here">
        <span className="insert-bar-line" />
        <span className="insert-bar-plus">+</span>
        <span className="insert-bar-line" />
      </button>
    </div>
  );
}
