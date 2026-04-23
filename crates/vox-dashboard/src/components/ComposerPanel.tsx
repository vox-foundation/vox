import React, { useEffect, useMemo, useState } from 'react';
import { Files, Sparkles, Check, X, RefreshCcw } from 'lucide-react';
import { voxTransport } from '../transport';
import type { ComposerState } from '../../../src/types';
import { CodeBlock } from './CodeBlock';

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
    <div className="h-full overflow-y-auto p-4 flex flex-col gap-4 bg-void/50 custom-scrollbar">
      <section className="glass-panel rounded-lg border border-border p-4 shadow-[inset_0_0_10px_rgba(0,0,0,0.5)]">
        <div className="flex items-center gap-2 mb-3">
          <Sparkles size={16} className="text-secondary-foreground" />
          <h3 className="text-sm font-rajdhani font-bold uppercase tracking-widest text-secondary-foreground">Composer Review</h3>
        </div>
        <p className="text-[11px] text-steel leading-relaxed mb-4 font-mono">
          Generate full-file edit proposals for several files, inspect them before apply, and keep a rollback point via the existing snapshot workflow.
        </p>
        <textarea
          className="w-full min-h-[92px] rounded border border-border bg-machine p-3 text-sm resize-y outline-none focus:border-cyan focus:shadow-[0_0_8px_var(--vox-cyan-glow)] text-foreground placeholder-steel opacity-80 focus:opacity-100 transition-all font-mono"
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
                    className={`px-2 py-1 rounded border text-[10px] uppercase font-bold tracking-widest transition-colors ${
                      selected ? 'bg-cyan bg-opacity-10 border-cyan text-cyan shadow-[0_0_5px_var(--vox-cyan-glow)]' : 'bg-machine border-border text-steel hover:text-foreground hover:bg-surface'
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
            className="px-4 py-1.5 rounded bg-primary text-black text-[11px] font-bold uppercase tracking-widest disabled:opacity-30 disabled:bg-machine disabled:text-steel hover:bg-amber-400 transition-colors shadow-[0_0_5px_var(--vox-amber-glow)]"
            disabled={composerState?.isGenerating || !prompt.trim() || selectedFiles.length === 0}
            onClick={() => voxTransport.callTool('vox_submit_task', { description: `Generate multi-file change: ${prompt}`, files: selectedFiles.map(f => ({ path: f, access: 'read' })) })}
          >
            {composerState?.isGenerating ? 'GENERATING...' : 'STAGE REVIEW'}
          </button>
          <button
            type="button"
            className="px-3 py-1.5 rounded border border-border bg-machine text-[10px] font-bold uppercase tracking-widest text-brass disabled:opacity-30 hover:border-copper hover:text-primary transition-colors"
            disabled={(composerState?.drafts?.length ?? 0) === 0}
            onClick={() => voxTransport.callTool('vox_apply_structured_edit', { paths: [] })}
          >
            APPLY ALL
          </button>
          <button
            type="button"
            className="px-3 py-1.5 rounded border border-border bg-void text-[10px] font-bold uppercase tracking-widest text-steel disabled:opacity-30 hover:border-destructive hover:text-destructive transition-colors"
            disabled={(composerState?.drafts?.length ?? 0) === 0}
            onClick={() => voxTransport.callTool('vox_undo', {})} // Discard all drafts via undo
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

      <section className="glass-panel rounded-lg border border-border p-4 shadow-[inset_0_0_10px_rgba(0,0,0,0.5)] mt-4">
        <div className="flex items-center justify-between mb-3 border-b border-border border-opacity-30 pb-2">
          <h3 className="text-sm font-rajdhani font-bold uppercase tracking-widest text-brass">Draft Queue</h3>
          <span className="text-[10px] font-mono text-cyan">{composerState?.drafts?.length ?? 0} STAGED</span>
        </div>
        {(composerState?.drafts?.length ?? 0) === 0 ? (
          <p className="text-[11px] text-steel font-mono">No staged composer drafts yet.</p>
        ) : (
          <div className="flex flex-col gap-3">
            {composerState?.drafts.map((draft) => (
              <article key={draft.path} className="rounded border border-border bg-machine p-3 hover:border-cyan transition-colors group">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="text-xs font-mono text-primary truncate max-w-[200px]" title={draft.path}>{draft.path.split(/[/\\]/).pop()}</div>
                    <div className="text-[9px] text-steel mt-1 font-mono uppercase tracking-widest">
                      {diffStat(draft.original, draft.proposed)}
                      {draft.model_used ? ` · ${draft.model_used}` : ''}
                      {typeof draft.tokens === 'number' ? ` · ${draft.tokens} tok` : ''}
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      className="p-1.5 rounded bg-machine text-emerald-400 border border-emerald-400/30 hover:bg-emerald-400 hover:text-black transition-colors"
                      onClick={() => voxTransport.callTool('vox_apply_structured_edit', { paths: [draft.path] })}
                      title="Apply Draft"
                    >
                      <Check size={14} />
                    </button>
                    <button
                      type="button"
                      className="p-1.5 rounded bg-machine text-destructive border border-destructive/30 hover:bg-destructive hover:text-white transition-colors"
                      onClick={() => voxTransport.callTool('vox_undo', { path: draft.path })} // Discard single draft
                      title="Discard Draft"
                    >
                      <X size={14} />
                    </button>
                  </div>
                </div>
                {draft.explanation ? (
                  <p className="mt-3 text-xs text-zinc-400 leading-relaxed">{draft.explanation}</p>
                ) : null}
                <details className="mt-3">
                  <summary className="cursor-pointer text-[10px] uppercase font-bold tracking-widest text-cyan flex items-center gap-2 select-none hover:text-white transition-colors">
                    <RefreshCcw size={12} />
                    DIFF PREVIEW
                  </summary>
                  <div className="grid grid-cols-2 gap-3 mt-3">
                    <div className="rounded bg-void border border-border/50 overflow-auto">
                      <CodeBlock code={draft.original} lang={langFromPath(draft.path)} />
                    </div>
                    <div className="rounded bg-cyan/5 border border-cyan/30 overflow-auto shadow-[0_0_8px_var(--vox-cyan-glow)]">
                      <CodeBlock code={draft.proposed} lang={langFromPath(draft.path)} />
                    </div>
                  </div>
                </details>

              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
