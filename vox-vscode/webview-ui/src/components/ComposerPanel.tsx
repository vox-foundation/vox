import React, { useEffect, useMemo, useState } from 'react';
import { Files, Sparkles, Check, X, RefreshCcw } from 'lucide-react';
import { getVsCodeApi } from '../utils/vscode';
import type { ComposerState } from '../../../src/types';
import { CodeBlock } from './CodeBlock';

const vscode = getVsCodeApi();

function langFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'rs': return 'rust';
    case 'ts': case 'tsx': return 'typescript';
    case 'js': case 'jsx': return 'javascript';
    case 'json': return 'json';
    case 'toml': return 'toml';
    case 'md': return 'markdown';
    case 'sql': return 'sql';
    case 'sh': case 'bat': case 'ps1': return 'bash';
    default: return 'text';
  }
}

function diffStat(original: string, proposed: string): string {
  const oldLines = original.split('\n').length;
  const newLines = proposed.split('\n').length;
  const delta = newLines - oldLines;
  return `${oldLines} -> ${newLines} lines (${delta >= 0 ? '+' : ''}${delta})`;
}

export function ComposerPanel({
  composerState,
}: {
  composerState: ComposerState | null;
}) {
  const available = composerState?.availableFiles ?? [];
  const [prompt, setPrompt] = useState('');
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);

  useEffect(() => {
    if (available.length > 0 && selectedFiles.length === 0) {
      setSelectedFiles(available.slice(0, Math.min(3, available.length)));
    }
  }, [available, selectedFiles.length]);

  const draftPaths = useMemo(
    () => new Set((composerState?.drafts ?? []).map((draft) => draft.path)),
    [composerState?.drafts],
  );

  const toggleFile = (file: string) => {
    setSelectedFiles((prev) => (prev.includes(file) ? prev.filter((entry) => entry !== file) : [...prev, file]));
  };

  return (
    <div className="h-full overflow-y-auto p-4 flex flex-col gap-4">
      <section className="glass rounded-2xl border border-white/10 p-4">
        <div className="flex items-center gap-2 mb-3">
          <Sparkles size={16} className="text-blue-500" />
          <h3 className="text-sm font-bold">Composer Review</h3>
        </div>
        <p className="text-xs text-zinc-400 leading-relaxed mb-4">
          Generate full-file edit proposals for several files, inspect them before apply, and keep a rollback point via the existing snapshot workflow.
        </p>
        <textarea
          className="w-full min-h-[92px] rounded-xl border border-white/10 bg-black/20 p-3 text-sm resize-y outline-none"
          placeholder="Describe the multi-file change you want..."
          value={prompt}
          onChange={(event) => setPrompt(event.target.value)}
        />
        <div className="mt-4">
          <div className="flex items-center gap-2 mb-2">
            <Files size={14} className="text-zinc-400" />
            <span className="text-xs font-bold uppercase tracking-widest text-zinc-500">Target files</span>
          </div>
          <div className="flex flex-wrap gap-2">
            {available.length === 0 ? (
              <span className="text-xs text-zinc-500">Open a few files in the editor to target them here.</span>
            ) : (
              available.map((file) => {
                const selected = selectedFiles.includes(file);
                return (
                  <button
                    key={file}
                    type="button"
                    onClick={() => toggleFile(file)}
                    className={`px-2 py-1 rounded-full border text-[11px] ${
                      selected ? 'bg-blue-500/15 border-blue-500/30 text-blue-300' : 'bg-black/20 border-white/10 text-zinc-400'
                    }`}
                  >
                    {file}
                  </button>
                );
              })
            )}
          </div>
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
          <button
            type="button"
            className="px-3 py-2 rounded-xl bg-blue-600 text-white text-xs font-semibold disabled:opacity-50"
            disabled={composerState?.isGenerating || !prompt.trim() || selectedFiles.length === 0}
            onClick={() => vscode.postMessage({ type: 'composerGenerate', prompt, files: selectedFiles })}
          >
            {composerState?.isGenerating ? 'Generating...' : 'Stage Review'}
          </button>
          <button
            type="button"
            className="px-3 py-2 rounded-xl border border-white/10 text-xs font-semibold text-zinc-300 disabled:opacity-50"
            disabled={(composerState?.drafts?.length ?? 0) === 0}
            onClick={() => vscode.postMessage({ type: 'composerApply', paths: [] })}
          >
            Apply All Drafts
          </button>
          <button
            type="button"
            className="px-3 py-2 rounded-xl border border-white/10 text-xs font-semibold text-zinc-300 disabled:opacity-50"
            disabled={(composerState?.drafts?.length ?? 0) === 0}
            onClick={() => vscode.postMessage({ type: 'composerDiscardAll' })}
          >
            Discard Drafts
          </button>
        </div>
        {composerState?.lastError ? (
          <p className="mt-3 text-xs text-amber-400">{composerState.lastError}</p>
        ) : null}
        {composerState?.snapshotRequested ? (
          <p className="mt-3 text-xs text-zinc-500">
            Applying composer drafts requests a rollback point first so the existing snapshot/undo tools remain your safety net.
          </p>
        ) : null}
      </section>

      <section className="glass rounded-2xl border border-white/10 p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-bold">Draft Queue</h3>
          <span className="text-xs text-zinc-500">{composerState?.drafts?.length ?? 0} staged</span>
        </div>
        {(composerState?.drafts?.length ?? 0) === 0 ? (
          <p className="text-xs text-zinc-500">No staged composer drafts yet.</p>
        ) : (
          <div className="flex flex-col gap-3">
            {composerState?.drafts.map((draft) => (
              <article key={draft.path} className="rounded-xl border border-white/10 bg-black/20 p-3">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="text-sm font-semibold">{draft.path}</div>
                    <div className="text-[11px] text-zinc-500 mt-1">
                      {diffStat(draft.original, draft.proposed)}
                      {draft.model_used ? ` · ${draft.model_used}` : ''}
                      {typeof draft.tokens === 'number' ? ` · ${draft.tokens} tok` : ''}
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      className="p-2 rounded-lg bg-emerald-500/10 text-emerald-300 border border-emerald-500/20"
                      onClick={() => vscode.postMessage({ type: 'composerApply', paths: [draft.path] })}
                    >
                      <Check size={14} />
                    </button>
                    <button
                      type="button"
                      className="p-2 rounded-lg bg-rose-500/10 text-rose-300 border border-rose-500/20"
                      onClick={() => vscode.postMessage({ type: 'composerDiscard', path: draft.path })}
                    >
                      <X size={14} />
                    </button>
                  </div>
                </div>
                {draft.explanation ? (
                  <p className="mt-3 text-xs text-zinc-400 leading-relaxed">{draft.explanation}</p>
                ) : null}
                <details className="mt-3">
                  <summary className="cursor-pointer text-xs text-blue-300 flex items-center gap-2">
                    <RefreshCcw size={12} />
                    Review exact replacement
                  </summary>
                  <div className="grid grid-cols-2 gap-3 mt-3">
                    <div className="rounded-xl bg-black/30 border border-white/10 overflow-auto">
                      <CodeBlock code={draft.original} lang={langFromPath(draft.path)} />
                    </div>
                    <div className="rounded-xl bg-blue-500/5 border border-blue-500/20 overflow-auto">
                      <CodeBlock code={draft.proposed} lang={langFromPath(draft.path)} />
                    </div>
                  </div>
                </details>
                {draftPaths.has(draft.path) ? null : null}
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
