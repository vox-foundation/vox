import { ErrorBoundary } from './components/ErrorBoundary';
import { AppShellLive } from './components/shell/AppShellLive';

export default function App() {
  return (
    <ErrorBoundary>
      <AppShellLive />
    </ErrorBoundary>
  );
}
