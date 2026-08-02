import { useState } from "react";
import { Notebook } from "./notebook/Notebook";
import { createDefaultNotebook } from "./demo/defaultNotebook";
import "./App.css";

function App() {
  // Created once: the Notebook component owns cell state from here on.
  const [initialCells] = useState(() => createDefaultNotebook());

  return (
    <div className="app-shell">
      <header className="app-header">
        <span className="app-title">OpenMat</span>
      </header>
      <Notebook initialCells={initialCells} />
    </div>
  );
}

export default App;
