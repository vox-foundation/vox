import React, { useState, type JSX } from 'react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from '../../ui/Dialog';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { executeSandboxProbe, type SandboxProbeOutcome } from './researchActions';

export interface SandboxReplModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialCode?: string;
  initialLanguage?: string;
}

const SUPPORTED_LANGUAGES = [
  { value: 'rust', label: 'Rust' },
  { value: 'python', label: 'Python' },
  { value: 'typescript', label: 'TypeScript' },
  { value: 'sql', label: 'SQL' },
  { value: 'vox', label: 'Vox' },
] as const;

export function SandboxReplModal({
  isOpen,
  onClose,
  initialCode = '',
  initialLanguage = 'rust',
}: SandboxReplModalProps): JSX.Element | null {
  const [language, setLanguage] = useState<string>(initialLanguage);
  const [code, setCode] = useState<string>(initialCode);
  const [running, setRunning] = useState<boolean>(false);
  const [outcome, setOutcome] = useState<SandboxProbeOutcome | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleRunProbe = async () => {
    setRunning(true);
    setError(null);
    try {
      const res = await executeSandboxProbe(code, language);
      setOutcome(res);
    } catch (err) {
      setError(sanitizeErrorForToast(err));
    } finally {
      setRunning(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent className="max-w-2xl max-h-[90vh] flex flex-col p-6 bg-[#121216] border border-border-subtle rounded-xl text-text-primary">
        <div className="flex items-start justify-between pb-3 border-b border-border-subtle">
          <div>
            <DialogTitle className="font-display text-base font-semibold text-text-primary tracking-wide">
              Sandbox REPL Probe
            </DialogTitle>
            <DialogDescription className="mt-0.5 text-xs text-text-muted">
              Verify claims and test polyglot code in an isolated compiler sandbox.
            </DialogDescription>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="text-text-muted hover:text-text-secondary text-lg leading-none px-2 py-1 rounded"
          >
            ✕
          </button>
        </div>

        <div className="mt-4 space-y-3 flex-1 overflow-y-auto">
          <div className="flex items-center gap-3">
            <label htmlFor="sandbox-lang-select" className="text-xs font-medium text-text-secondary">
              Language:
            </label>
            <select
              id="sandbox-lang-select"
              aria-label="Language"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              className="rounded border border-border-subtle bg-black/50 px-2 py-1 text-xs text-text-primary outline-none focus:border-brass/50"
            >
              {SUPPORTED_LANGUAGES.map((lang) => (
                <option key={lang.value} value={lang.value}>
                  {lang.label}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label htmlFor="sandbox-code-editor" className="sr-only">
              Code Editor
            </label>
            <textarea
              id="sandbox-code-editor"
              aria-label="Code"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              rows={8}
              placeholder="// Write code here to probe in the sandbox..."
              className="w-full rounded-lg border border-border-subtle bg-black/60 p-3 font-mono text-xs text-text-primary outline-none focus:border-brass/40 resize-y"
            />
          </div>

          <div className="flex items-center justify-between pt-1">
            <button
              type="button"
              onClick={handleRunProbe}
              disabled={running}
              className="rounded-lg border border-brass/40 bg-brass/10 px-4 py-2 text-xs font-medium text-brass hover:bg-brass/20 disabled:opacity-50 transition-colors"
            >
              {running ? 'Running Probe…' : 'Run Compiler Probe'}
            </button>
          </div>

          {error && (
            <div className="p-3 rounded-lg border border-red-500/30 bg-red-500/10 text-xs text-red-400">
              Execution Error: {error}
            </div>
          )}

          {outcome && (
            <div className="mt-4 space-y-2 rounded-lg border border-border-subtle bg-black/40 p-3">
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold text-text-secondary">Probe Result:</span>
                <span
                  className={`px-2 py-0.5 rounded text-[11px] font-mono font-semibold uppercase ${
                    outcome.passed
                      ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                      : 'bg-rose-500/20 text-rose-400 border border-rose-500/30'
                  }`}
                >
                  {outcome.passed ? 'Passed' : 'Failed'}
                </span>
              </div>

              {outcome.stdout && (
                <div>
                  <div className="text-[11px] font-medium text-text-muted mb-1">Standard Output:</div>
                  <pre className="max-h-36 overflow-auto rounded bg-black/60 p-2 font-mono text-[11px] text-text-secondary">
                    {outcome.stdout}
                  </pre>
                </div>
              )}

              {outcome.stderr && (
                <div>
                  <div className="text-[11px] font-medium text-rose-400 mb-1">Standard Error:</div>
                  <pre className="max-h-36 overflow-auto rounded bg-red-950/20 border border-red-900/30 p-2 font-mono text-[11px] text-rose-300">
                    {outcome.stderr}
                  </pre>
                </div>
              )}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
