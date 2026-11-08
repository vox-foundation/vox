import React, { useEffect, useRef, useState } from 'react';
import { discoveryHelp, discoveryRecord, type ActionHelp } from '../../../transport';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useLabel } from '../../../hooks/useLanguage';
import { Glass } from '../../ui/Glass';
import { useLocalStorage } from '../../../hooks/useLocalStorage';

interface Props {
  /** The action id currently under the cursor / top suggestion, or null. */
  actionId: string | null;
  /** Epoch ms (passed in so the component stays deterministic/testable). */
  nowMs: number;
  gamifyEnabled?: boolean;
  /** Apply the suggested example command to the console input. */
  onUseAction?: (example: string, actionId: string) => void;
}

const DISCOVERY_RAIL_COLLAPSED_KEY = 'gui.console.discovery_rail_collapsed.v1';

/**
 * Persistent right-hand rail (layout A). Resolves the active action to its help
 * and records a "seen" exposure (with dwell) so the spaced-repetition scheduler
 * learns what the user has been shown.
 */
export function DiscoveryRail({ actionId, nowMs, gamifyEnabled = false, onUseAction }: Props) {
  const [help, setHelp] = useState<ActionHelp | null>(null);
  const [collapsed, setCollapsed] = useLocalStorage<boolean>(DISCOVERY_RAIL_COLLAPSED_KEY, false);
  const shownAt = useRef<number>(nowMs);

const LOCAL_ACTION_HELP: Record<string, ActionHelp> = {
  'slash.help': {
    action_id: 'slash.help',
    about: 'List terminal slash commands or show help for one command.',
    args: [{ name: 'cmd', help: 'Optional command name', required: false }],
    example: '/help',
  },
  'slash.model': {
    action_id: 'slash.model',
    about: 'Inspect or switch the active session LLM model or route.',
    args: [{ name: 'id|route', help: 'Model identifier or routing tier', required: false }],
    example: '/model route',
  },
  'slash.nu': {
    action_id: 'slash.nu',
    about: 'Show Nushell structured shell idioms or run a structured pipeline.',
    args: [{ name: 'pipeline', help: 'Optional Nushell pipeline', required: false }],
    example: '/nu',
  },
  'nu.where': {
    action_id: 'nu.where',
    about: 'Filter rows of a structured Nushell stream by predicate.',
    args: [{ name: 'predicate', help: 'Row filter condition (e.g. size > 1kb)', required: true }],
    example: '!nu ls | where size > 1kb',
  },
  'nu.select': {
    action_id: 'nu.select',
    about: 'Project specific columns from a structured Nushell table.',
    args: [{ name: 'columns', help: 'Column names to select', required: true }],
    example: '!nu ls | select name size',
  },
  'nu.to_json': {
    action_id: 'nu.to_json',
    about: 'Serialize structured Nushell pipeline output as JSON.',
    args: [{ name: '--raw', help: 'Emit compact single-line JSON', required: false }],
    example: '!nu ls | to json --raw',
  },
};

  useEffect(() => {
    if (!actionId) {
      setHelp(null);
      return;
    }
    shownAt.current = nowMs;
    let live = true;
    discoveryHelp(actionId)
      .then((h) => {
        if (!live) return;
        setHelp(h ?? LOCAL_ACTION_HELP[actionId] ?? null);
      })
      .catch(() => {
        if (!live) return;
        setHelp(LOCAL_ACTION_HELP[actionId] ?? null);
      });
    return () => {
      live = false;
    };
    // Keyed on actionId only: nowMs changes on unrelated parent re-renders and
    // must not re-fetch help.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- keyed on actionId only
  }, [actionId]);

  // Record a "seen" exposure when the active action settles (debounced by 2s of
  // dwell — matches spec §discovery rail). Fire-and-forget.
  useEffect(() => {
    if (!actionId) return;
    const DWELL_MS = 2000;
    const t = setTimeout(() => {
      discoveryRecord(actionId, false, shownAt.current + DWELL_MS, DWELL_MS).catch(() => {});
    }, DWELL_MS);
    return () => clearTimeout(t);
    // Keyed on actionId only — see note above; restarting on nowMs would starve
    // the dwell timer.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- keyed on actionId only
  }, [actionId]);

  if (collapsed) {
    return (
      <aside aria-label="discovery" className="shrink-0" data-testid="discovery-rail">
        <Glass className="flex flex-col items-center gap-2 p-2">
          <button
            type="button"
            data-testid="discovery-rail-restore-button"
            aria-label="Expand discovery rail"
            aria-expanded={false}
            title="Expand discovery rail"
            onClick={() => setCollapsed(false)}
            className="rounded-lg border border-border-subtle p-2 text-text-muted transition hover:border-brass/40 hover:bg-overlay-hover hover:text-brass"
          >
            <span className="font-mono text-sm" aria-hidden="true">
              »
            </span>
          </button>
        </Glass>
      </aside>
    );
  }

  const handleUse = () => {
    if (!actionId || !help) return;
    void recordGamifyGuiEvent(
      'discovery_action_used',
      { action_id: actionId },
      { enabled: gamifyEnabled },
    );
    onUseAction?.(help.example, actionId);
  };

  return (
    <aside
      aria-label="discovery"
      aria-live="polite"
      className="w-[268px] shrink-0"
      data-testid="discovery-rail"
    >
      <Glass
        style={{
          backgroundImage:
            'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
        className="flex h-full flex-col gap-2.5 border-2 border-[#C59B27] p-3 text-xs text-[#1C140C] shadow-[0_6px_18px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)]"
      >
        <div className="flex items-center justify-between gap-2 border-b border-[#9E7B2F]/45 pb-1.5">
          <h2 className="font-display text-[10.5px] font-bold uppercase tracking-[0.16em] text-[#1C140C]">{useLabel('con-discovery')}</h2>
          <button
            type="button"
            data-testid="discovery-rail-hide-button"
            aria-label="Collapse discovery rail"
            aria-expanded={true}
            title="Collapse discovery rail"
            onClick={() => setCollapsed(true)}
            className="rounded-md border border-[#8C6825] bg-[#251C0E] px-1.5 py-0.5 text-[#F3E5AB] transition hover:border-[#D4AF37] hover:text-white"
          >
            <span className="font-mono text-xs" aria-hidden="true">
              «
            </span>
          </button>
        </div>

        {!help ? (
          <div className="space-y-2.5">
            <p className="font-serif text-[11.5px] font-medium text-[#3B2E1E]">
              Start typing a <code className="rounded bg-[#251C0E] px-1 py-0.5 text-[10px] text-[#F3E5AB]">vox</code> command to see live help and idioms.
            </p>
            <div className="space-y-1.5 pt-1">
              <div className="font-display text-[9px] font-bold uppercase tracking-widest text-[#6B4C12]">
                Quick Idioms
              </div>
              {Object.values(LOCAL_ACTION_HELP).slice(0, 4).map((item) => (
                <div
                  key={item.action_id}
                  className="rounded-lg border border-[#9E7B2F]/65 bg-[#241B11] p-2 text-[#F6F0E4]"
                >
                  <div className="font-mono text-[10.5px] font-semibold text-[#F3E5AB]">{item.example}</div>
                  <div className="mt-0.5 text-[10px] text-[#D8C7A8] line-clamp-2">{item.about}</div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <>
            <h3 className="font-mono text-[13px] font-bold text-[#1C140C]">{help.example}</h3>
            <p className="text-[#3B2E1E] font-medium">{help.about}</p>
            {help.args.length > 0 && (
              <ul className="list-disc pl-4 text-[#3B2E1E]">
                {help.args.map((a) => (
                  <li key={a.name}>
                    <code className="font-semibold text-[#1C140C]">{a.name}</code>
                    {a.required ? ' (required)' : ''} — {a.help}
                  </li>
                ))}
              </ul>
            )}
            <button
              type="button"
              onClick={handleUse}
              aria-label={`Use suggested action ${help.example}`}
              className="mt-2 self-start rounded-lg border border-[#8C6825] bg-[#251C0E] px-2.5 py-1.5 text-[11px] font-semibold text-[#F3E5AB] transition hover:border-[#D4AF37] hover:text-white"
            >
              Use
            </button>
          </>
        )}
      </Glass>
    </aside>
  );
}
