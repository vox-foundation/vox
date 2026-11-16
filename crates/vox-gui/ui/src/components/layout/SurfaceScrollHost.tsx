import React from 'react';

export function SurfaceScrollHost({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-1 min-h-0 min-w-0 flex-col overflow-hidden" data-testid="surface-scroll-host">
      <div
        className="@container/surface h-full min-h-0 min-w-0 overflow-x-hidden overflow-y-auto custom-scrollbar"
        data-testid="surface-scroll-viewport"
        role="region"
        aria-label="Surface content"
        tabIndex={0}
      >
        {children}
      </div>
    </div>
  );
}
