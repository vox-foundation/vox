import React, { useEffect, useRef, useState } from 'react';
import { discoveryHelp, discoveryRecord, type ActionHelp } from '../../../transport';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useLabel } from '../../../hooks/useLanguage';
import { useLocalStorage } from '../../../hooks/useLocalStorage';
import { Button } from '../../ui/Button';
import { Glass } from '../../ui/Glass';
import { Icon } from '../../ui/Icons';
import { Tooltip } from '../../ui/Tooltip';
import { RunbookPanel } from './RunbookPanel';
import { TERMINAL_MONO_STYLE } from './terminalTheme';

export interface DiscoveryRailProps {
  actionId: string | null;
  nowMs: number;
  gamifyEnabled?: boolean;
  onUseAction?: (example: string, actionId: string) => void;
  activeTab?: 'help' | 'runbooks';
  onTabChange?: (tab: 'help' | 'runbooks') => void;
  selectedRunbookId?: string | null;
  cwd?: string;
  onExecuteRunbook?: (command: string) => void;
  onInsertRunbook?: (command: string) => void;
}

const DISCOVERY_RAIL_COLLAPSED_KEY = 'gui.console.discovery_rail_collapsed.v1';

const LOCAL_ACTION_HELP: Record<string, ActionHelp> = {
  'slash.help': { action_id: 'slash.help', about: 'List terminal slash commands or show help for one command.', args: [{ name: 'cmd', help: 'Optional command name', required: false }], example: '/help' },
  'slash.model': { action_id: 'slash.model', about: 'Inspect or switch the active session LLM model or route.', args: [{ name: 'id|route', help: 'Model identifier or routing tier', required: false }], example: '/model route' },
  'slash.nu': { action_id: 'slash.nu', about: 'Show Nushell structured shell idioms or run a structured pipeline.', args: [{ name: 'pipeline', help: 'Optional Nushell pipeline', required: false }], example: '/nu' },
  'nu.where': { action_id: 'nu.where', about: 'Filter rows of a structured Nushell stream by predicate.', args: [{ name: 'predicate', help: 'Row filter condition (e.g. size > 1kb)', required: true }], example: '!nu ls | where size > 1kb' },
  'nu.select': { action_id: 'nu.select', about: 'Project specific columns from a structured Nushell table.', args: [{ name: 'columns', help: 'Column names to select', required: true }], example: '!nu ls | select name size' },
  'nu.to_json': { action_id: 'nu.to_json', about: 'Serialize structured Nushell pipeline output as JSON.', args: [{ name: '--raw', help: 'Emit compact single-line JSON', required: false }], example: '!nu ls | to json --raw' },
};

export function DiscoveryRail({
  actionId, nowMs, gamifyEnabled = false, onUseAction,
  activeTab, onTabChange, selectedRunbookId, cwd, onExecuteRunbook, onInsertRunbook,
}: DiscoveryRailProps) {
  const headingLabel = useLabel('con-discovery');
  const [help, setHelp] = useState<ActionHelp | null>(null);
  const [collapsed, setCollapsed] = useLocalStorage<boolean>(
    DISCOVERY_RAIL_COLLAPSED_KEY,
    typeof window !== 'undefined' && window.innerWidth > 0 && window.innerWidth < 980,
  );
  const manualExpandNarrowRef = useRef(false);
  const [internalTab, setInternalTab] = useState<'help' | 'runbooks'>('help');
  const shownAt = useRef<number>(nowMs);
  const currentTab = activeTab ?? internalTab;

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const onResize = () => {
      if (window.innerWidth > 0 && window.innerWidth < 980) {
        if (!manualExpandNarrowRef.current) setCollapsed(true);
      } else {
        manualExpandNarrowRef.current = false;
      }
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, [setCollapsed]);

  const selectTab = (next: 'help' | 'runbooks') => {
    setInternalTab(next);
    onTabChange?.(next);
  };

  useEffect(() => {
    if (!actionId) {
      setHelp(null);
      return;
    }
    shownAt.current = nowMs;
    let live = true;
    discoveryHelp(actionId)
      .then((h) => { if (live) setHelp(h ?? LOCAL_ACTION_HELP[actionId] ?? null); })
      .catch(() => { if (live) setHelp(LOCAL_ACTION_HELP[actionId] ?? null); });
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- keyed on actionId only
  }, [actionId]);

  useEffect(() => {
    if (!actionId) return;
    const DWELL_MS = 2000;
    const t = setTimeout(() => {
      discoveryRecord(actionId, false, shownAt.current + DWELL_MS, DWELL_MS).catch(() => {});
    }, DWELL_MS);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- keyed on actionId only
  }, [actionId]);

  if (collapsed) {
    return (
      <aside aria-label="discovery" className="w-8 shrink-0" data-testid="discovery-rail">
        <Glass padding="p-0" className="flex flex-col items-center gap-2 bg-bg-surface py-1">
          <Tooltip content="Expand discovery rail" side="left">
            <button
              type="button" data-testid="discovery-rail-restore-button"
              aria-label="Expand discovery rail" aria-expanded={false}
              onClick={() => {
                if (typeof window !== 'undefined' && window.innerWidth > 0 && window.innerWidth < 980) {
                  manualExpandNarrowRef.current = true;
                }
                setCollapsed(false);
              }}
              className="inline-flex size-6 items-center justify-center rounded border border-border-subtle bg-bg-elevated text-text-muted transition-colors hover:border-border-strong hover:bg-overlay-hover hover:text-text-primary"
            >
              <Icon.chevL className="size-3" aria-hidden="true" />
            </button>
          </Tooltip>
        </Glass>
      </aside>
    );
  }

  const handleUse = () => {
    if (!actionId || !help) return;
    void recordGamifyGuiEvent('discovery_action_used', { action_id: actionId }, { enabled: gamifyEnabled });
    onUseAction?.(help.example, actionId);
  };

  const fallbackInsert = (cmd: string) => onUseAction?.(cmd, 'runbook');
  const tabCls = (active: boolean) =>
    `rounded px-2 py-1 text-[11px] leading-[1.25] font-semibold transition-colors ${
      active ? 'bg-bg-surface text-accent shadow-xs' : 'text-text-secondary hover:text-text-primary'
    }`;

  return (
    <aside
      aria-label="discovery"
      aria-live="polite"
      className="flex w-60 xl:w-64 shrink-0 flex-col border-l border-border-subtle bg-bg-surface text-text-primary max-[1099px]:absolute max-[1099px]:right-0 max-[1099px]:top-0 max-[1099px]:bottom-0 max-[1099px]:z-20 max-[1099px]:shadow-xl"
      data-testid="discovery-rail"
    >
      <Glass padding="p-3" className="flex h-full min-h-0 flex-col gap-2 overflow-y-auto border border-border-strong bg-bg-surface text-xs text-text-primary">
        <div className="flex items-center justify-between gap-2 border-b border-border-subtle pb-2">
          <h2 className="font-display text-[11px] leading-[1.25] font-semibold uppercase tracking-wider text-text-primary">
            {headingLabel}
          </h2>
          <Tooltip content="Collapse discovery rail" side="left">
            <button
              type="button" data-testid="discovery-rail-hide-button"
              aria-label="Collapse discovery rail" aria-expanded={true}
              onClick={() => { manualExpandNarrowRef.current = false; setCollapsed(true); }}
              className="inline-flex size-6 items-center justify-center rounded border border-border-subtle bg-bg-elevated text-text-secondary transition-colors hover:border-border-strong hover:text-text-primary"
            >
              <Icon.chevR className="size-3" aria-hidden="true" />
            </button>
          </Tooltip>
        </div>

        <div role="tablist" aria-label="Discovery rail mode" className="grid grid-cols-2 gap-1 rounded border border-border-subtle bg-bg-elevated p-1">
          <button type="button" role="tab" data-testid="discovery-tab-help" aria-selected={currentTab === 'help'} onClick={() => selectTab('help')} className={tabCls(currentTab === 'help')}>
            Action Help
          </button>
          <button type="button" role="tab" data-testid="discovery-tab-runbooks" aria-selected={currentTab === 'runbooks'} onClick={() => selectTab('runbooks')} className={tabCls(currentTab === 'runbooks')}>
            Runbooks
          </button>
        </div>

        {currentTab === 'runbooks' ? (
          <RunbookPanel
            cwd={cwd} selectedRunbookId={selectedRunbookId}
            onInsert={onInsertRunbook ?? fallbackInsert}
            onRun={onExecuteRunbook ?? onInsertRunbook ?? fallbackInsert}
          />
        ) : !help ? (
          <div className="flex flex-col gap-2">
            <p className="text-xs leading-[1.45] font-normal text-text-secondary">
              Start typing a <code data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="rounded border border-border-subtle bg-bg-elevated px-1 text-[11px] font-semibold text-text-primary">vox</code> command to see live help and idioms.
            </p>
            <div className="flex flex-col gap-2 pt-1">
              <div className="font-display text-[11px] leading-[1.25] font-semibold uppercase tracking-wider text-text-muted">Quick Idioms</div>
              {Object.values(LOCAL_ACTION_HELP).slice(0, 4).map((item) => (
                <div key={item.action_id} className="flex flex-col gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-left text-xs text-text-primary hover:border-brass">
                  <div data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="text-xs leading-[1.45] font-semibold text-text-primary">{item.example}</div>
                  <div className="line-clamp-2 text-[11px] leading-[1.25] font-normal text-text-secondary">{item.about}</div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            <h3 data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="text-sm leading-[1.3] font-semibold text-text-primary">{help.example}</h3>
            <p className="text-xs leading-[1.45] font-normal text-text-secondary">{help.about}</p>
            {help.args.length > 0 && (
              <ul className="list-disc pl-4 text-xs leading-[1.45] font-normal text-text-secondary">
                {help.args.map((a) => (
                  <li key={a.name}>
                    <code data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="font-semibold text-text-primary">{a.name}</code>
                    {a.required ? ' (required)' : ''} - {a.help}
                  </li>
                ))}
              </ul>
            )}
            <Button
              size="xs" variant="secondary" onClick={handleUse}
              aria-label={`Use Suggested Action ${help.example}`}
              className="mt-1 h-6 self-start rounded border border-brass bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-primary hover:border-border-strong"
            >
              Use Suggested Action
            </Button>
          </div>
        )}
      </Glass>
    </aside>
  );
}
