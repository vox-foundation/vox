import React, { Component, ErrorInfo, ReactNode } from 'react';
import { AlertCircle, RotateCcw } from 'lucide-react';

interface Props {
  children?: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught error:', error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div className="h-full w-full bg-bg-base text-text-primary flex flex-col items-center justify-center p-10">
          <div className="glass bg-bg-surface border border-(--color-status-fail)/40 rounded-3xl p-8 max-w-lg w-full flex flex-col items-center text-center relative overflow-hidden">
            <div className="absolute inset-0 bg-(--color-status-fail)/5 animate-pulse pointer-events-none" />
            <AlertCircle size={48} className="text-(--color-status-fail) mb-6" />
            <h2 className="text-xl font-bold uppercase tracking-widest text-(--color-status-fail) mb-2">Display Runtime Error</h2>
            <p className="text-text-muted text-sm mb-6 max-w-sm">
              The interface encountered an unexpected payload from the Orchestrator. 
            </p>
            <div className="bg-bg-elevated p-4 rounded-xl border border-border-subtle w-full text-left overflow-x-auto mb-8">
              <code className="text-[10px] font-mono text-text-secondary">
                {this.state.error?.message || "Unknown rendering exception"}
              </code>
            </div>
            <button 
              onClick={() => this.setState({ hasError: false })}
              className="flex items-center gap-2 bg-bg-elevated border border-border-strong text-text-primary px-6 py-3 rounded-xl text-xs font-bold uppercase tracking-widest hover:border-(--color-accent-default) transition-colors"
            >
              <RotateCcw size={14} /> Recover State
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
