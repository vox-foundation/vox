import React from 'react';
import { ErrorBoundary } from './components/ErrorBoundary';
import { AppShell } from '../app/src/generated/AppShell';

// Transport bridge wired in Phase 2 via @island strategy.
// For now AppShell provides the full tab layout with local state only.
export default function App() {
  return (
    <ErrorBoundary>
      <AppShell />
    </ErrorBoundary>
  );
}
