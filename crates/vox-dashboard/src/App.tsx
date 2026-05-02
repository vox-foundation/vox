import { ErrorBoundary } from './components/ErrorBoundary';
import { AppShellLive } from './components/islands/AppShellLive';

export default function App() {
  return (
    <ErrorBoundary>
      <AppShellLive />
    </ErrorBoundary>
  );
}
