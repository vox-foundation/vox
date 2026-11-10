import React, { useEffect, useRef, useState } from 'react';
import type { CommandCatalog, CommandCatalogEntry, CommandCatalogArgument } from '../types/catalog';
import { voxTransport } from '../transport';
import { sanitizeErrorForToast } from '../lib/backendGuard';
import { confirmationRequiredId } from '../lib/capabilityRun';
import { ConfirmRunDialog } from './layout/ConfirmRunDialog';
import { RomanMedallion } from './ui/RomanTooltip';
import { Icon } from './ui/Icons';

export interface ExecuteOutput {
    exit_code: number;
    stdout: string;
    stderr: string;
}

/** Which input control a typed argument should render as. */
export type ArgControl = 'flag' | 'select' | 'count' | 'text';

/**
 * Map a clap-derived argument (SP-1 typed metadata) to its GUI control:
 * boolean flags → checkbox, repeatable counters → number, enumerated values
 * → dropdown, everything else → free text.
 */
export function argControl(arg: CommandCatalogArgument): ArgControl {
    if (arg.value_kind === 'flag') return 'flag';
    if (arg.value_kind === 'count') return 'count';
    if (arg.possible_values && arg.possible_values.length > 0) return 'select';
    return 'text';
}

export function CommandCatalogForm({
    catalog,
    initialPath,
    nonce,
}: {
    catalog: CommandCatalog;
    /** Palette handoff: command path to select; re-applied whenever `nonce` changes. */
    initialPath?: string[];
    nonce?: number;
}) {
    const [selectedPath, setSelectedPath] = useState<string | null>(null);
    const [argsInput, setArgsInput] = useState('');
    const [argValues, setArgValues] = useState<Record<string, string | boolean>>({});
    // D-03: a destructive refusal parks the exact args here until the user confirms.
    // It remembers the path it was created for, so Confirm can never retarget a different command.
    const [pendingConfirm, setPendingConfirm] = useState<{ id: string; path: string[]; args: Record<string, unknown> } | null>(null);
    // WR-08: every selection / run bumps the token; a result for an older token is dropped. `running`
    // blocks a second Run while one is pending (a side-effecting verb must not execute twice).
    const runToken = useRef(0);
    const [running, setRunning] = useState(false);
    const [output, setOutput] = useState<{ success?: boolean; data?: ExecuteOutput; error?: string } | null>(null);

    const commandList = catalog?.entries || [];

    const handleCommandSelect = (pathArray: string[]) => {
        const path = pathArray.join(' ');
        setSelectedPath(path);
        setArgsInput('');
        // Seed argument values from the CLI's real defaults (SP-1 default_values).
        const cmd = commandList.find((c: CommandCatalogEntry) => c.path.join(' ') === path);
        const seeded: Record<string, string | boolean> = {};
        for (const arg of cmd?.arguments ?? []) {
            const def = arg.default_values?.[0];
            if (def === undefined) continue;
            const fieldKey = arg.long || arg.name;
            seeded[fieldKey] = argControl(arg) === 'flag' ? def === 'true' : def;
        }
        setArgValues(seeded);
        setOutput(null);
        setPendingConfirm(null);
        runToken.current += 1; // results of a run started for another selection are stale now
        setRunning(false);
    };

    // Select the palette-chosen command (selecting only fills the form; Run stays explicit).
    useEffect(() => {
        if (!initialPath || nonce === undefined) return;
        handleCommandSelect(initialPath);
        document
            .querySelector(`[data-catalog-path="${CSS.escape(initialPath.join(' '))}"]`)
            ?.scrollIntoView?.({ block: 'center' });
        // eslint-disable-next-line react-hooks/exhaustive-deps -- keyed on nonce on purpose
    }, [nonce]);

    /**
     * D-16: positional arguments (neither long nor short) go out as `__positionals` in declaration
     * order; options keep their long name; `__argv` carries the raw extra arguments.
     */
    const buildArgs = (cmd: CommandCatalogEntry): Record<string, unknown> => {
        const argsObj: Record<string, unknown> = {};
        const positionals: string[] = [];
        for (const arg of cmd.arguments ?? []) {
            const fieldKey = arg.long || arg.name;
            const val = argValues[fieldKey];
            if (!arg.long && !arg.short) {
                if (typeof val === 'string' && val) positionals.push(val);
            } else if (typeof val === 'boolean') {
                if (val) argsObj[fieldKey] = true;
            } else if (val) {
                argsObj[fieldKey] = val;
            }
        }
        if (positionals.length) argsObj.__positionals = positionals;
        argsObj.__argv = argsInput.trim().split(/\s+/).filter(Boolean);
        return argsObj;
    };

    // Straight to the gated `execute_command` (never the ungated MCP route). `confirmed` is
    // true only on the retry the user approved in ConfirmRunDialog.
    const run = async (path: string[], args: Record<string, unknown>, confirmed?: boolean) => {
        if (running) return;
        const token = ++runToken.current;
        setRunning(true);
        try {
            const res = await voxTransport.executeCommand(path, args, { confirmed });
            if (token === runToken.current) setOutput({ success: true, data: res });
        } catch (err) {
            if (token !== runToken.current) return;
            const id = confirmationRequiredId(err);
            if (id) {
                setPendingConfirm({ id, path, args });
                return;
            }
            setOutput({ error: sanitizeErrorForToast(err) });
        } finally {
            if (token === runToken.current) setRunning(false);
        }
    };

    const handleExecute = () => {
        const cmd = commandList.find((c: CommandCatalogEntry) => c.path.join(' ') === selectedPath);
        if (!cmd) return;
        void run(cmd.path, buildArgs(cmd));
    };

    const currentCommand = commandList.find((c: CommandCatalogEntry) => c.path.join(' ') === selectedPath);

    return (
        <div className="flex h-full text-[#C8B99E] w-full rounded-xl border border-[#D4AF37]/40 bg-bg-surface overflow-hidden">
            <div className="w-72 border-r border-[#D4AF37]/30 bg-bg-surface/90 p-4 overflow-y-auto">
                <div className="flex items-center gap-2 mb-4">
                    <RomanMedallion active size="sm">
                        <Icon.command className="size-3.5 text-[#F3E5AB]" />
                    </RomanMedallion>
                    <h3 className="font-display text-sm font-bold tracking-widest text-brass uppercase">Command Catalog</h3>
                </div>
                <ul className="list-none p-0">
                    {commandList.map((cmd: CommandCatalogEntry) => {
                        const p = cmd.path.join(' ');
                        const isSelected = selectedPath === p;
                        return (
                            <li 
                                key={p}
                                data-catalog-path={p}
                                aria-current={isSelected ? 'true' : undefined}
                                onClick={() => handleCommandSelect(cmd.path)}
                                className={`cursor-pointer rounded-r-md p-2 mb-1 text-sm font-mono transition-colors border-l-2 ${
                                    isSelected ? 'bg-[#D4AF37]/15 border-[#D4AF37] text-[#F3E5AB]' : 'border-transparent text-[#C8B99E] hover:bg-[#D4AF37]/5'
                                }`}
                            >
                                <div className="flex justify-between items-center">
                                    <span>vox {p}</span>
                                    {cmd.has_subcommands && <span className="text-xs text-text-muted">▸</span>}
                                </div>
                                <div className="mt-1 flex gap-2">
                                    <span className="text-[10px] uppercase tracking-wider bg-bg-surface border border-[#D4AF37]/35 px-1.5 py-0.5 rounded-sm text-brass">
                                        {cmd.tier}
                                    </span>
                                    {cmd.aliases.length > 0 && (
                                        <span className="text-[10px] text-text-muted">
                                            alias: {cmd.aliases.join(', ')}
                                        </span>
                                    )}
                                </div>
                            </li>
                        );
                    })}
                </ul>
            </div>
            <div className="flex-1 p-6 overflow-y-auto bg-bg-surface">
                {currentCommand ? (
                    <div>
                        <div className="flex items-center gap-3 mb-2">
                            <RomanMedallion active size="md">
                                <Icon.command className="size-4 text-text-primary" />
                            </RomanMedallion>
                            <h2 className="text-2xl font-display text-brass">vox {selectedPath}</h2>
                        </div>
                        <p className="text-text-secondary font-mono text-sm mb-6">{currentCommand.about}</p>
                        
                        {currentCommand.arguments && currentCommand.arguments.length > 0 && (
                            <div className="space-y-4 mb-6">
                                <h3 className="text-sm font-bold tracking-widest text-brass uppercase border-b border-[#D4AF37]/30 pb-2">Arguments & Flags</h3>
                                <div className="grid grid-cols-1 gap-4">
                                    {currentCommand.arguments.map((arg: CommandCatalogArgument) => {
                                        const control = argControl(arg);
                                        const label = arg.long ? `--${arg.long}` : (arg.short ? `-${arg.short}` : arg.name);
                                        const fieldKey = arg.long || arg.name;
                                        const inputClass = "bg-bg-surface border border-[#D4AF37]/45 rounded-sm px-3 py-1.5 text-sm text-[#F3E5AB] font-mono focus:border-[#D4AF37] focus:outline-hidden transition-colors max-w-md";
                                        return (
                                            <div key={arg.name} className="flex flex-col">
                                                <div className="flex items-center gap-2 mb-1">
                                                    {control === 'flag' && (
                                                        <input
                                                            type="checkbox"
                                                            id={arg.name}
                                                            checked={!!argValues[fieldKey]}
                                                            onChange={e => setArgValues(prev => ({ ...prev, [fieldKey]: e.target.checked }))}
                                                            className="bg-bg-surface border border-[#D4AF37]/45 rounded-sm focus:border-[#D4AF37]"
                                                        />
                                                    )}
                                                    <label htmlFor={arg.name} className="text-xs font-bold tracking-widest text-[#C8B99E] font-mono">{label}</label>
                                                    {arg.required && <span className="text-[10px] text-red-400">REQUIRED</span>}
                                                </div>
                                                {arg.help && <div className="text-[10px] text-[#C8B99E]/70 mb-1">{arg.help}</div>}
                                                {control === 'select' && (
                                                    <select
                                                        id={arg.name}
                                                        value={(argValues[fieldKey] as string) ?? ''}
                                                        onChange={e => setArgValues(prev => ({ ...prev, [fieldKey]: e.target.value }))}
                                                        className={inputClass}
                                                    >
                                                        {!arg.required && <option value="">—</option>}
                                                        {arg.possible_values?.map(v => (
                                                            <option key={v} value={v}>{v}</option>
                                                        ))}
                                                    </select>
                                                )}
                                                {control === 'count' && (
                                                    <input
                                                        type="number"
                                                        min={0}
                                                        id={arg.name}
                                                        value={(argValues[fieldKey] as string) ?? ''}
                                                        onChange={e => setArgValues(prev => ({ ...prev, [fieldKey]: e.target.value }))}
                                                        placeholder={arg.name}
                                                        className={inputClass}
                                                    />
                                                )}
                                                {control === 'text' && (
                                                    <input
                                                        type="text"
                                                        id={arg.name}
                                                        value={(argValues[fieldKey] as string) ?? ''}
                                                        onChange={e => setArgValues(prev => ({ ...prev, [fieldKey]: e.target.value }))}
                                                        placeholder={arg.name}
                                                        className={inputClass}
                                                    />
                                                )}
                                            </div>
                                        );
                                    })}
                                </div>
                            </div>
                        )}

                        <div className="space-y-4 mb-6">
                            <div className="flex flex-col">
                                <label className="text-xs font-bold tracking-widest text-[#C8B99E] uppercase mb-1">Additional Raw Arguments</label>
                                <input 
                                    type="text" 
                                    placeholder="e.g. --json --force"
                                    value={argsInput}
                                    onChange={e => setArgsInput(e.target.value)}
                                    className="bg-bg-surface border border-[#D4AF37]/45 rounded-sm px-3 py-2 text-sm text-[#F3E5AB] font-mono focus:border-[#D4AF37] focus:outline-hidden transition-colors w-full"
                                />
                            </div>
                        </div>

                        <button 
                            type="button"
                            onClick={handleExecute}
                            disabled={running}
                            aria-busy={running}
                            className="bg-[#D4AF37] text-[#1C140C] font-bold px-4 py-2 rounded-sm uppercase tracking-widest hover:bg-[#F3E5AB] transition-colors disabled:opacity-60 disabled:cursor-wait"
                        >
                            {running ? 'Running…' : 'Run'}
                        </button>

                        {output && (
                            <div className="mt-8 p-4 bg-bg-surface border border-[#D4AF37]/45 rounded-sm">
                                <h4 className="text-sm font-bold tracking-widest text-[#C8B99E] uppercase mb-2">Execution Result</h4>
                                {output.error ? (
                                    <div className="text-red-500 font-mono text-sm">Error: {output.error}</div>
                                ) : (
                                    <>
                                        <div className={`text-xs font-mono mb-2 ${output.data?.exit_code === 0 ? 'text-green-500' : 'text-red-500'}`}>
                                            Exit Code: {output.data?.exit_code}
                                        </div>
                                        {output.data?.stdout && (
                                            <div className="mb-2">
                                                <div className="text-[10px] text-[#C8B99E]/70 uppercase">stdout</div>
                                                <pre className="text-xs font-mono text-[#F3E5AB] whitespace-pre-wrap">{output.data.stdout}</pre>
                                            </div>
                                        )}
                                        {output.data?.stderr && (
                                            <div>
                                                <div className="text-[10px] text-[#C8B99E]/70 uppercase">stderr</div>
                                                <pre className="text-xs font-mono text-red-400 whitespace-pre-wrap">{output.data.stderr}</pre>
                                            </div>
                                        )}
                                    </>
                                )}
                            </div>
                        )}
                    </div>
                ) : (
                    <div className="space-y-4">
                        <div className="flex items-center justify-center rounded-xl border border-dashed border-[#D4AF37]/40 bg-bg-surface/60 p-6 text-[#C8B99E] font-mono text-sm uppercase tracking-widest">
                            Select a command from the catalog to configure and execute.
                        </div>
                        <div
                            data-testid="catalog-overview-deck"
                            className="grid grid-cols-1 gap-3 sm:grid-cols-2"
                        >
                            <div className="rounded-xl border border-[#D4AF37]/40 bg-bg-surface/80 p-4">
                                <div className="flex items-center justify-between">
                                    <span className="font-display text-[10px] tracking-widest text-brass uppercase">
                                        Typed Clap Metadata
                                    </span>
                                    <span className="rounded border border-brass/30 bg-brass/10 px-1.5 py-0.5 font-mono text-[9px] text-brass">
                                        {commandList.length} commands
                                    </span>
                                </div>
                                <p className="mt-1.5 text-[11px] leading-relaxed text-[#C8B99E]">
                                    Every CLI subcommand in <code className="text-[#D4AF37]">vox-cli</code> compiles its flags, positional arguments, and default values directly into typed form controls.
                                </p>
                            </div>
                            <div className="rounded-xl border border-[#D4AF37]/40 bg-bg-surface/80 p-4">
                                <div className="flex items-center justify-between">
                                    <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">
                                        Destructive Run Gate
                                    </span>
                                    <span className="rounded border border-emerald-400/30 bg-emerald-400/10 px-1.5 py-0.5 font-mono text-[9px] text-emerald-300">
                                        WR-08 Guarded
                                    </span>
                                </div>
                                <p className="mt-1.5 text-[11px] leading-relaxed text-[#C8B99E]">
                                    Mutating or destructive verbs require explicit confirmation in <code className="text-brass">ConfirmRunDialog</code> and drop stale tokens on selection change.
                                </p>
                            </div>
                        </div>
                        {commandList.length > 0 && (
                            <div className="rounded-xl border border-[#D4AF37]/40 bg-bg-surface/80 p-4">
                                <div className="mb-3 flex items-center justify-between">
                                    <span className="font-display text-[10px] tracking-widest text-[#C8B99E] uppercase">
                                        Quick-Select Command Matrix
                                    </span>
                                    <span className="font-mono text-[10px] text-text-muted">
                                        Click a card to load its Clap argument schema
                                    </span>
                                </div>
                                <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-4">
                                    {commandList.slice(0, 8).map((cmd: CommandCatalogEntry) => {
                                        const p = cmd.path.join(' ');
                                        return (
                                            <button
                                                key={`quick-${p}`}
                                                type="button"
                                                onClick={() => handleCommandSelect(cmd.path)}
                                                className="flex flex-col items-start justify-between rounded-lg border border-[#D4AF37]/35 bg-bg-surface/90 p-2.5 text-left transition hover:border-brass/60 hover:bg-[#D4AF37]/10"
                                            >
                                                <div className="flex w-full items-center justify-between gap-1">
                                                    <span className="font-mono text-xs font-medium text-[#F3E5AB]">vox {p}</span>
                                                    <span className="rounded border border-brass/25 bg-brass/10 px-1 py-0.5 font-mono text-[9px] uppercase text-brass">
                                                        {cmd.source_group}
                                                    </span>
                                                </div>
                                                <div className="mt-1 line-clamp-1 font-mono text-[10px] text-[#C8B99E]">
                                                    {cmd.about || `Execute vox ${p}`}
                                                </div>
                                                <div className="mt-1.5 flex items-center gap-1.5 font-mono text-[9px] text-text-muted">
                                                    <span>{cmd.arguments?.length ?? 0} args</span>
                                                    <span>·</span>
                                                    <span className="text-emerald-300">{cmd.compiled_in ? 'compiled' : 'gated'}</span>
                                                </div>
                                            </button>
                                        );
                                    })}
                                </div>
                            </div>
                        )}
                    </div>
                )}
            </div>
            <ConfirmRunDialog
                open={pendingConfirm !== null}
                commandText={`vox ${pendingConfirm?.path.join(' ') ?? ''}`}
                rowId={pendingConfirm?.id ?? ''}
                onCancel={() => setPendingConfirm(null)}
                onConfirm={() => {
                    const p = pendingConfirm;
                    setPendingConfirm(null);
                    if (p) void run(p.path, p.args, true);
                }}
            />
        </div>
    );
}
