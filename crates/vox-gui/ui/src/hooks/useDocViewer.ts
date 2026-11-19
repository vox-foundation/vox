import { useCallback, useState } from 'react';

export interface ActiveDoc {
  path: string;
  title: string;
  initialContent?: string;
  initialMode?: 'read' | 'split' | 'edit';
}

export interface OpenDocOptions {
  initialContent?: string;
  initialMode?: 'read' | 'split' | 'edit';
}

function titleFromPath(path: string): string {
  return path.split('/').pop()?.replace(/\.(md|mdx|markdown|adoc|txt|json|yaml|yml|toml|csv|tsv|vox|rs|ts|tsx|sql)$/i, '') ?? path;
}

/**
 * Doc-viewer state: at most one open doc, presented as a non-modal docked pane
 * (see DocViewerDrawer.tsx). Supports optional initialContent / initialMode so
 * Research syntheses or + New Doc templates can open directly in Split/Edit mode.
 */
export function useDocViewer() {
  const [activeDoc, setActiveDoc] = useState<ActiveDoc | null>(null);

  const openDoc = useCallback((path: string, title?: string, options?: OpenDocOptions) => {
    setActiveDoc({
      path,
      title: title ?? titleFromPath(path),
      ...(options?.initialContent !== undefined ? { initialContent: options.initialContent } : {}),
      ...(options?.initialMode !== undefined ? { initialMode: options.initialMode } : {}),
    });
  }, []);

  const closeDoc = useCallback(() => {
    setActiveDoc(null);
  }, []);

  return { activeDoc, openDoc, closeDoc };
}
