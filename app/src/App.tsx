import { useState } from "react";
import { Notebook } from "./notebook/Notebook";
import { createDefaultNotebook } from "./demo/defaultNotebook";
import "./App.css";

// The other sprint agents (persistence, LLM, settings) self-register these
// on window; they may not exist outside the full app, so every call goes
// through optional chaining. Typed locally rather than in notebook/types.ts
// since this app does not own their exact signatures.
interface OpenMatIntegrations {
  __openmat_save?: () => void;
  __openmat_open?: () => void;
  __openmat_settings?: () => void;
  __openmat_askai?: () => void;
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
          <button type="button" className="app-header-button app-header-button-accent" onClick={() => integrations().__openmat_askai?.()}>
            Ask AI
          </button>
        </div>
      </header>
      <Notebook initialCells={initialCells} />
      {/* Other agents' UI (Save/Open/Settings dialogs, the Ask AI panel)
          portals into this mount point rather than each owning a modal
          root of its own. */}
      <div id="openmat-modals" />
    </div>
  );
}

export default App;
