import { useState } from "react";
import { Notebook } from "./notebook/Notebook";
import { createDefaultNotebook } from "./demo/defaultNotebook";
import "./App.css";

// These modules self-register their window integrations (save/open,
// settings) as an import side effect; the calls below go through optional
// chaining so the app still renders if one is absent (e.g. some tests).
import "./persistence";
import "./settings";

interface OpenMatIntegrations {
  __openmat_save?: () => void;
  __openmat_open?: () => void;
  __openmat_settings?: () => void;
}

function integrations(): OpenMatIntegrations {
  return window as unknown as OpenMatIntegrations;
}

function App() {
  // Created once: the Notebook component owns cell state from here on.
  const [initialCells] = useState(() => createDefaultNotebook());

  return (
    <div className="app-shell">
      <header className="app-header">
        <span className="app-title">OpenMat</span>
        <div className="app-header-actions">
          <button type="button" className="app-header-button" onClick={() => integrations().__openmat_save?.()}>
            Save
          </button>
          <button type="button" className="app-header-button" onClick={() => integrations().__openmat_open?.()}>
            Open
          </button>
          <button type="button" className="app-header-button" onClick={() => integrations().__openmat_settings?.()}>
            Settings
          </button>
        </div>
      </header>
      <Notebook initialCells={initialCells} />
      {/* Save/Open/Settings dialogs portal into this mount point rather
          than each owning a modal root of its own. */}
      <div id="openmat-modals" />
    </div>
  );
}

export default App;
