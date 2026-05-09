import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { ErrorBoundary } from './ErrorBoundary';

// Install Vox-runtime globals (str, len, Speech, std, mobile) BEFORE importing
// the codegen output, since the emitted modules reference them at top level.
import "./runtime";
import { registerServiceWorker } from './sync';

import { voxRoutes, type VoxRoute } from "../dist/routes.manifest";

/**
 * Minimal client-side router for Vox-emitted apps.
 *
 * Maps `window.location.pathname` to one of the entries in `voxRoutes` (the
 * Vox-emitted routes manifest). Intercepts intra-app `<a>` clicks so they
 * push history state instead of forcing a full page navigation. No external
 * router dep — the app's flat route shape (Home / Timeline / Weekly /
 * Export / Voice) doesn't justify pulling in a 30 KB router.
 */
function findRoute(path: string): VoxRoute | undefined {
  const exact = voxRoutes.find((r) => r.path === path);
  if (exact) return exact;
  return voxRoutes.find((r) => r.index);
}

function App(): React.ReactElement {
  const [path, setPath] = useState<string>(window.location.pathname);

  useEffect(() => {
    const onPop = () => setPath(window.location.pathname);
    window.addEventListener("popstate", onPop);

    const onClick = (e: MouseEvent) => {
      // Only intercept plain left-clicks on intra-app links.
      if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
      const a = (e.target as HTMLElement).closest?.("a");
      if (!a) return;
      const href = a.getAttribute("href");
      if (!href || href.startsWith("http") || href.startsWith("//") || href.startsWith("#")) return;
      e.preventDefault();
      window.history.pushState(null, "", href);
      setPath(href);
    };
    document.addEventListener("click", onClick);

    return () => {
      window.removeEventListener("popstate", onPop);
      document.removeEventListener("click", onClick);
    };
  }, []);

  const route = findRoute(path);
  if (!route) {
    return (
      <div className="mh-root">
        <h1>404</h1>
        <p>
          No route for <code>{path}</code>.
        </p>
        <a href="/">Go home</a>
      </div>
    );
  }
  const Component = route.component;
  return <Component />;
}

registerServiceWorker();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
