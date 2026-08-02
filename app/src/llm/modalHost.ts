// Mounts a modal into #openmat-modals when the app shell provides that
// layer, or document.body otherwise. Used to open this pane imperatively
// from window.__openmat_askai(), outside of any existing React tree, so
// there is no parent component to portal from; this creates its own React
// root at the target instead, which has the same effect.
import { createRoot, type Root } from "react-dom/client";
import type { ReactElement } from "react";

export function mountModal(render: (close: () => void) => ReactElement): () => void {
  const host = document.getElementById("openmat-modals") ?? document.body;
  const container = document.createElement("div");
  host.appendChild(container);

  const root: Root = createRoot(container);

  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    root.unmount();
    container.remove();
  };

  root.render(render(close));
  return close;
}
