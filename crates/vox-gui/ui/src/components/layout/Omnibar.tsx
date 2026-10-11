import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { voxTransport } from '../../transport';
import { Icon } from '../ui/Icons';
import { CommandCatalogEntry } from '../../types/catalog';
import { Agent } from '../../types/dashboard';
import { UnifiedHit } from '../surfaces/Search/searchHelpers';
import { parsePaletteQuery, type PalettePrefixMode } from './paletteSources';
import type { FederatedIndexKind } from '../../lib/federatedSearchIndex';
import { useSearchController } from '../../hooks/useSearchController';
import { useFederatedSearchIndex } from '../../hooks/useFederatedSearchIndex';
import { useCapabilityIndex } from '../../hooks/useCapabilityIndex';
import { kindsForMode, noteRecentCapability, rankCapabilities, recentCapabilities, toRankEntries } from '../../lib/capabilityRank';
import { useContentManifest } from '../../hooks/useContentManifest';
import { querySearchableRegistry } from '../../lib/searchableRegistry';
import {
  buildOmnibarFacets,
  omnibarRowsInOrder,
  queryForFederatedSearch,
  type GraphNeighbor,
  type OmnibarGraphResult,
  type OmnibarRow,
  type RowKind,
} from '../../lib/omnibarFacets';
import { recordGamifyGuiEvent } from '../../lib/gamifyGuiEvents';
import { unwrapMcpEnvelope } from '../../lib/mcpToolResult';
import { decideCliActivation, type CliActivation } from '../../lib/capabilityRun';

const SETTINGS_SEED_KEY = 'vox_settings_seed';

/**
 * Graph-discover MCP tool. `vox_graphify_query` was renamed to the `vox_search_*`
 * family (dispatch.rs registers vox_search_{status,structural,neighbors,path,
 * callers,callees,compare,rebuild}); this constant was left pointing at the old
 * name, so the facet errored on every keystroke. `vox_search_structural` is the
 * only graph tool that takes a lexical query string.
 */
const GRAPH_DISCOVER_TOOL = 'vox_search_structural';

/** VG-1-owned neighbor-expansion MCP tool (umbrella spec §3.1: { corpus, node_ids, max_depth }). */
const GRAPH_NEIGHBORS_TOOL = 'vox_search_neighbors';

/** Row kind -> icon (aria-hidden; the label carries the meaning). */
const KIND_ICON: Partial<Record<RowKind, keyof typeof Icon>> = {
  cli: 'terminal',
  mcp_tool: 'puzzle',
  crate: 'cpu',
  skill: 'spark',
  doc: 'book',
  surface: 'dashboard',
  setting: 'settings',
  policy: 'shield',
  agent: 'users',
  action: 'command',
};

/** D-01: the trailing hint on a cli row, from the same decision Enter uses. */
const CLI_HINT: Record<CliActivation['kind'], string> = {
  run: '↵ runs',
  form: '↵ opens form',
  explain: '↵ explains',
};

/** D-08a: shown under a capped Commands / Tools facet; the Explorer link arrives in phase 06.4. */
const NARROW_HINT = 'Keep typing to narrow, or use > (commands) / # (crates)';

/**
 * finding #4: preserve CommandPalette's prefix-mode → kind restriction. Maps
 * `/`=docs+skills, `@`/`#`/`>`= no federated kinds, so the federated lane actually narrows.
 * D-05: surfaces and commands come from the capability index, so they are not federated kinds
 * any more; the federated lane keeps settings, policies, docs and installed skills.
 */
function federatedKindsForMode(mode: PalettePrefixMode, indexDown = false): FederatedIndexKind[] {
  // WR-06: with the index chunk missing, built-in surfaces come back through the federated lane.
  const base = federatedKindsForModeBase(mode);
  return indexDown && (mode === 'default' || mode === 'commands') ? [...base, 'surface'] : base;
}

function federatedKindsForModeBase(mode: PalettePrefixMode): FederatedIndexKind[] {
  switch (mode) {
    case 'skills':
      return ['doc', 'skill'];
    case 'commands':
    case 'agents':
    case 'crates':
      return [];
    default:
      return ['setting', 'policy', 'doc', 'skill'];
  }
}

/**
 * Parse a graph-tool response into rows. `invoke_mcp_tool` returns the daemon's
 * whole `ToolResult` under `result`, so the payload of both
 * `vox_search_structural` and `vox_search_neighbors` lives at `result.data`, and
 * the hits at `result.data.hits[]`. `results[]` is accepted as a legacy
 * fallback. Reading `result.hits` (no envelope unwrap) made the facet silently
 * return zero rows on every keystroke.
 */
export function parseDiscoverResults(res: unknown): GraphNeighbor[] {
  const r = res as { is_error?: boolean; result?: unknown };
  if (r?.is_error) return [];
  const data = unwrapMcpEnvelope(r?.result) as
    | { hits?: unknown[]; results?: unknown[] }
    | null
    | undefined;
  const raw = Array.isArray(data?.hits)
    ? data.hits
    : Array.isArray(data?.results)
      ? data.results
      : [];
  return raw.flatMap((raw0) => {
    const n = raw0 as { node_id?: string; id?: string; label?: string };
    const id = n.node_id ?? n.id;
    if (typeof id !== 'string' || id.length === 0) return [];
    const vk = id.startsWith('surface:') ? id.slice('surface:'.length) : undefined;
    return [{ id, label: n.label ?? vk ?? id, viewKey: vk }];
  });
}

interface OmnibarProps {
  open: boolean;
  onClose: () => void;
  onNavigate: (viewKey: string, anchorId?: string) => void;
  onRunCommand: (command: string) => void;
  onSendToChat: (query: string) => void;
  onOpenDoc: (path: string) => void;
  onSubmitTask: () => void;
  /** Open the Catalog surface with this CLI path selected (never executes). */
  onOpenCatalogForm?: (path: string[]) => void;
  /** Explain an index row (MCP tool, crate, skill) in the drawer. */
  onExplain?: (rowId: string) => void;
  /** D-01: run a read-only CLI leaf directly (see decideCliActivation). */
  onRunCli: (path: string[]) => void;
  agents: Agent[];
  /** Installed skills (not the CLI catalog). */
  skills: CommandCatalogEntry[];
  /** The live CLI command catalog; decides run / form / explain for cli rows. */
  catalog: CommandCatalogEntry[];
  gamifyEnabled?: boolean;
  /** Handler-backed keybind actions with live chords (D-13). */
  actions?: { id: string; label: string; chord: string }[];
  onRunAction?: (id: string) => void;
}

/** Always-offered quick action — not derived from any search source, so it
 *  must be spliced into the commands facet directly rather than relying on
 *  buildOmnibarFacets' query-matched rows (which are empty on an empty query). */
const SUBMIT_TASK_ROW: OmnibarRow = {
  id: 'quick-action:submit-task',
  facet: 'commands',
  label: 'Submit new task…',
  detail: 'Open Chat and focus the composer',
  provenance: 'corpus',
  activate: { type: 'submit-task' },
};

function ProvenanceBadge({ hint }: { hint: string }) {
  return (
    <span className="shrink-0 rounded-sm border border-border-subtle bg-overlay-subtle px-1.5 py-px font-mono text-[9px] uppercase tracking-widest text-text-muted">
      {hint}
    </span>
  );
}

export function Omnibar({
  open,
  onClose,
  onNavigate,
  onRunCommand,
  onSendToChat,
  onOpenDoc,
  onSubmitTask,
  onOpenCatalogForm,
  onExplain,
  onRunCli,
  agents,
  skills,
  catalog,
  gamifyEnabled = false,
  actions,
  onRunAction,
}: OmnibarProps) {
  const [q, setQ] = useState('');
  const [selectedRowIdx, setSelectedRowIdx] = useState(-1);
  const [graph, setGraph] = useState<OmnibarGraphResult>({ rows: [], error: null });

  const { mode: prefixMode, query: effectiveQ } = parsePaletteQuery(q);
  const backendSearchEnabled = open && prefixMode === 'default';

  // Debounced copy of effectiveQ used ONLY to throttle the GRAPH discover MCP
  // call so it does not fire an `invokeMcpTool` per keystroke. Local-only facets
  // (federated/runtime/manifest) keep using effectiveQ directly — they are cheap.
  const [debouncedQ, setDebouncedQ] = useState(effectiveQ);
  useEffect(() => {
    const id = setTimeout(() => setDebouncedQ(effectiveQ), 200);
    return () => clearTimeout(id);
  }, [effectiveQ]);

  const { state: searchState, setQuery: setSearchQuery } = useSearchController({
    enabled: backendSearchEnabled,
  });
  const backendHits = useMemo(
    () => (backendSearchEnabled ? (searchState.hits as UnifiedHit[]) : []),
    [searchState.hits, backendSearchEnabled],
  );

  // finding #5: agents are not in the federated index — CommandPalette seeded them
  // from the `agents` prop. Carry them forward as activatable command-facet rows
  // (filtered in agents/default mode), routed via the `agent` activation arm.
  const agentRows = useMemo<OmnibarRow[]>(() => {
    if (prefixMode !== 'default' && prefixMode !== 'agents') return [];
    const ql = effectiveQ.trim().toLowerCase();
    if (!ql) return [];
    return agents
      .filter((a) => a.codename.toLowerCase().includes(ql) || a.id.toLowerCase().includes(ql))
      .map((a) => ({
        id: `agent:${a.id}`,
        facet: 'commands' as const,
        label: `${a.codename} (${a.id})`,
        detail: a.phase ?? '',
        provenance: 'corpus' as const,
        activate: { type: 'agent' as const, agentId: a.id },
      }));
  }, [agents, effectiveQ, prefixMode]);

  const skillSources = useMemo(
    () =>
      skills.map((s) => ({
        id: s.capability_id ?? s.command,
        name: s.command,
        description: s.about,
      })),
    [skills],
  );
  const { search: searchFederated } = useFederatedSearchIndex(skillSources);
  const {
    rows: indexRows,
    entries: loadedEntries,
    domains,
    loading: indexLoading,
    error: indexError,
    retry: retryIndex,
  } = useCapabilityIndex(open);
  const fedKinds = useMemo(() => federatedKindsForMode(prefixMode, !!indexError), [prefixMode, indexError]);
  const federated = useMemo(
    () => {
      const q = queryForFederatedSearch(effectiveQ);
      return q && fedKinds.length > 0 ? searchFederated(q, { kinds: fedKinds }) : [];
    },
    [searchFederated, effectiveQ, fedKinds],
  );

  // Capability index (D-06): loaded lazily on the first open; cli rows feed Commands (D-05).
  // WR-06: if the chunk failed to load, the live CLI catalog stands in for the cli rows (the form is
  // always safe: unclassified rows never run directly) and surfaces come from the federated lane.
  const rankEntries = useMemo(
    () =>
      indexError && loadedEntries.length === 0
        ? toRankEntries(
            catalog.map((e) => ({
              id: `cli.${e.path.join('.')}`,
              kind: 'cli' as const,
              title: e.path.join(' '),
              summary: e.about,
              domain: null,
              cli: `vox ${e.path.join(' ')}`,
              mcp: null,
              viewKey: null,
              crate: null,
              layer: null,
              docs: null,
              coverage: null,
              equivalents: [],
              refs: [],
              tier: e.tier,
              status: null,
              sideEffect: null,
              lane: null,
              catalogForm: true,
              navLabel: null,
              source: null,
            })),
          )
        : loadedEntries,
    [indexError, loadedEntries, catalog],
  );
  // WR-07: no global cap, so each facet's "N of M" counts every match and no kind is starved by
  // another's higher-ranked rows. The pool is ~1.5k rows; facets cap what they render.
  const indexHits = useMemo(
    () =>
      rankCapabilities(rankEntries, effectiveQ, {
        kinds: kindsForMode(prefixMode),
        recent: recentCapabilities(),
        limit: Number.POSITIVE_INFINITY,
      }),
    [rankEntries, effectiveQ, prefixMode],
  );

  // D-01: what Enter does on a cli row. A row missing from the index counts as unclassified (form).
  const catalogByPath = useMemo(() => new Map(catalog.map((e) => [e.path.join(' '), e])), [catalog]);
  const indexById = useMemo(() => new Map(indexRows.map((r) => [r.id, r])), [indexRows]);
  const cliDecision = useCallback(
    (rowId: string, path: string[]): CliActivation =>
      decideCliActivation({ sideEffect: indexById.get(rowId)?.sideEffect ?? null }, catalogByPath.get(path.join(' '))),
    [indexById, catalogByPath],
  );

  const installedSkillNames = useMemo(() => new Set(skills.map((s) => s.path[s.path.length - 1])), [skills]);

  const manifest = useContentManifest();
  const runtimeHits = useMemo(
    () => (effectiveQ.trim() ? querySearchableRegistry(effectiveQ) : []),
    [effectiveQ],
  );

  // GRAPH facet: graph-discover MCP tool, independently fallible. The tool is
  // vox_search_structural. Its payload sits behind the daemon's ToolResult
  // envelope, at `result.data.hits[]` (the legacy `results[]` is still
  // tolerated) — see parseDiscoverResults, which unwraps it.
  // A failure here is a real search failure, not an absent capability.
  useEffect(() => {
    if (!open || !debouncedQ.trim()) {
      setGraph({ rows: [], error: null });
      return;
    }
    let cancelled = false;
    voxTransport
      .invokeMcpTool(GRAPH_DISCOVER_TOOL, { query: debouncedQ, limit: 6 })
      .then((res) => {
        if (cancelled) return;
        const r = res as { is_error?: boolean };
        if (r?.is_error) {
          setGraph({ rows: [], error: 'graph search failed' });
          return;
        }
        setGraph({ rows: parseDiscoverResults(res), error: null });
      })
      .catch(() => {
        if (!cancelled) {
          setGraph({ rows: [], error: 'graph search unreachable' });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [open, debouncedQ]);

  const facets = useMemo(() => {
    const base = buildOmnibarFacets({
      query: effectiveQ,
      prefixMode,
      federated,
      backendHits,
      manifest,
      runtimeHits,
      graph,
      indexHits,
      domains,
      installedSkillNames,
      actions,
    });
    // Carry agents into the COMMANDS facet (finding #5) without re-capping below
    // FACET_CAP would drop real commands — agents lead, then existing commands.
    // SUBMIT_TASK_ROW is a default quick action shown only on the empty-query
    // landing state — like a real command palette's default suggestions, it
    // steps aside the moment the user starts searching for something else.
    // WR-11: only in the default mode — a bare `#` / `>` / `/` / `@` has an empty query too.
    const withSubmitTask = effectiveQ.trim() || prefixMode !== 'default' ? agentRows : [...agentRows, SUBMIT_TASK_ROW];
    if (withSubmitTask.length === 0) return base;
    return base.map((f) =>
      f.key === 'commands'
        ? { ...f, rows: [...withSubmitTask, ...f.rows], total: f.total + withSubmitTask.length }
        : f,
    );
  }, [effectiveQ, prefixMode, federated, backendHits, manifest, runtimeHits, graph, agentRows, indexHits, domains, installedSkillNames, actions]);
  const rows = useMemo(() => omnibarRowsInOrder(facets), [facets]);

  const activateRow = useCallback(
    (row: OmnibarRow) => {
      recordGamifyGuiEvent('palette_navigation', { facet: row.facet }, { enabled: gamifyEnabled });
      if (row.id.startsWith('index:')) noteRecentCapability(row.id.slice('index:'.length));
      switch (row.activate.type) {
        case 'navigate':
          onNavigate(row.activate.viewKey, row.activate.anchorId);
          break;
        case 'command':
          onRunCommand(row.activate.command);
          break;
        case 'doc':
          onOpenDoc(row.activate.path);
          break;
        case 'cli': {
          const d = cliDecision(row.activate.rowId, row.activate.path);
          if (d.kind === 'run') onRunCli(row.activate.path);
          else if (d.kind === 'form') onOpenCatalogForm?.(row.activate.path);
          else onExplain?.(row.activate.rowId);
          break;
        }
        case 'explain':
          onExplain?.(row.activate.rowId);
          break;
        case 'graph':
          if (row.activate.node.viewKey) onNavigate(row.activate.node.viewKey);
          break;
        // finding #5: carried-in CommandPalette arms — route the same way.
        case 'setting':
          try {
            localStorage.setItem(
              SETTINGS_SEED_KEY,
              JSON.stringify({ section: row.activate.section, settingId: row.activate.settingId }),
            );
          } catch {
            /* ignore */
          }
          onNavigate('settings');
          try {
            window.dispatchEvent(new Event('vox-settings-seed'));
          } catch {
            /* ignore */
          }
          break;
        case 'policy':
          onNavigate('policies');
          break;
        case 'skill':
          onRunCommand(`skill:${row.activate.skillId}`);
          break;
        case 'agent':
          onNavigate('roster');
          break;
        case 'submit-task':
          onSubmitTask();
          break;
        case 'action':
          // Close first so an overlay action (sheet, inspector) is not dismissed by the close.
          onClose();
          onRunAction?.(row.activate.actionId);
          return;
      }
      onClose();
    },
    [onNavigate, onRunCommand, onOpenDoc, onOpenCatalogForm, onExplain, onRunCli, cliDecision, onSubmitTask, onRunAction, onClose, gamifyEnabled],
  );

  const expandGraphNeighbors = useCallback((row: OmnibarRow) => {
    if (row.activate.type !== 'graph') return;
    const seed = row.activate.node;
    // vox_search_neighbors is the real neighbor primitive:
    // { corpus, node_ids, max_depth }. Its payload sits behind the daemon's
    // ToolResult envelope, at `result.data.hits[]` — parseDiscoverResults
    // unwraps it, so do not reach for `result.hits` here.
    // TODO(VG-1): pass `corpus` once the omnibar carries an active/seed corpus.
    // The discover call (GRAPH_DISCOVER_TOOL) also omits corpus today and relies
    // on the tool default; both should be threaded the same active corpus.
    voxTransport
      .invokeMcpTool(GRAPH_NEIGHBORS_TOOL, { node_ids: [seed.id], max_depth: 1 })
      .then((res) => {
        const added0 = parseDiscoverResults(res);
        setGraph((prev) => {
          if (prev.error) return prev;
          const seen = new Set(prev.rows.map((n) => n.id));
          const added = added0.filter((n) => !seen.has(n.id));
          return { rows: [...prev.rows, ...added], error: null };
        });
      })
      .catch(() => {
        /* facet stays as-is — honest no-op */
      });
  }, []);

  // a11y: remember what had focus when the palette opened (captured during render, before the
  // autofocused input steals it) and give it back on close.
  const returnFocus = useRef<HTMLElement | null>(null);
  const wasOpen = useRef(false);
  if (open && !wasOpen.current) returnFocus.current = document.activeElement as HTMLElement | null;
  wasOpen.current = open;
  useEffect(() => {
    if (open) return;
    const el = returnFocus.current;
    returnFocus.current = null;
    if (el?.isConnected) el.focus();
  }, [open]);

  const rowsRef = useRef(rows);
  const idxRef = useRef(selectedRowIdx);
  rowsRef.current = rows;
  idxRef.current = selectedRowIdx;

  useEffect(() => {
    if (!open) {
      setQ('');
      setSearchQuery('');
      setSelectedRowIdx(-1);
    }
  }, [open, setSearchQuery]);

  useEffect(() => setSelectedRowIdx(-1), [rows.length, q]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      // WR-03: the Enter / arrow that drives an IME composition belongs to the IME (keyCode 229 on WebKit).
      if (e.isComposing || e.keyCode === 229) return;
      const list = rowsRef.current;
      let idx = idxRef.current;
      if (e.key === 'Escape') {
        onClose();
      } else if (e.key === 'Enter' && e.shiftKey) {
        e.preventDefault();
        if (q.trim()) onSendToChat(q.trim());
        onClose();
      } else if (e.key === 'ArrowDown' && list.length > 0) {
        e.preventDefault();
        idx = idx < 0 ? 0 : Math.min(idx + 1, list.length - 1);
        idxRef.current = idx;
        setSelectedRowIdx(idx);
      } else if (e.key === 'ArrowUp' && list.length > 0) {
        e.preventDefault();
        idx = idx < 0 ? list.length - 1 : Math.max(idx - 1, 0);
        idxRef.current = idx;
        setSelectedRowIdx(idx);
      } else if (e.key === 'ArrowRight' && e.altKey) {
        e.preventDefault();
        const target = idx >= 0 && idx < list.length ? list[idx] : list[0];
        if (target && target.activate.type === 'graph') expandGraphNeighbors(target);
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const target = idx >= 0 && idx < list.length ? list[idx] : list[0];
        if (target) activateRow(target);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose, onSendToChat, activateRow, expandGraphNeighbors, q]);

  // WR-04: aria-modal needs a real trap. Tab wraps inside the dialog; focus that lands outside
  // (programmatic, or a click that slipped past the overlay) is pulled back to the first control.
  const dialogRef = useRef<HTMLDivElement>(null);
  const focusables = () =>
    Array.from(dialogRef.current?.querySelectorAll<HTMLElement>('input, button:not([disabled])') ?? []);
  useEffect(() => {
    if (!open) return;
    const onFocusIn = (e: FocusEvent) => {
      if (dialogRef.current && e.target instanceof Node && !dialogRef.current.contains(e.target)) {
        focusables()[0]?.focus();
      }
    };
    document.addEventListener('focusin', onFocusIn);
    return () => document.removeEventListener('focusin', onFocusIn);
  }, [open]);
  const onDialogKeyDown = (e: React.KeyboardEvent) => {
    if (e.key !== 'Tab') return;
    const items = focusables();
    if (items.length === 0) return;
    const first = items[0];
    const last = items[items.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  if (!open) return null;

  let rowCursor = 0;

  const loadingLine = q.trim().length > 0 && indexLoading;
  const noMatches = q.length > 0 && rows.length === 0 && !indexLoading && !facets.some((f) => f.error);
  const statusText = loadingLine
    ? 'Loading capability index…'
    : noMatches
      ? `No matches for "${q}" — press ⇧⏎ to ask chat`
      : rows.length > 0
        ? `${rows.length} result${rows.length === 1 ? '' : 's'}`
        : '';

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/60 backdrop-blur-xs pt-[14vh]"
      onClick={onClose}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        onKeyDown={onDialogKeyDown}
        className="w-[680px] max-w-[92vw] rounded-2xl border border-border-subtle bg-bg-base/90 shadow-[0_40px_120px_-30px_rgba(0,0,0,0.9)] backdrop-blur-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border-subtle px-4 py-3">
          <Icon.search className="size-4 text-brass" aria-hidden="true" />
          <input
            autoFocus
            aria-label="Search Vox"
            value={q}
            onChange={(e) => {
              const v = e.target.value;
              setQ(v);
              setSearchQuery(v);
            }}
            placeholder="Search surfaces, commands, on-screen text, graph, docs…  ⇧⏎ to ask chat"
            className="flex-1 bg-transparent text-[14px] text-text-primary placeholder:text-text-muted outline-hidden"
          />
          <kbd className="rounded-sm border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[10px] text-text-muted">esc</kbd>
        </div>

        <div className="max-h-[480px] overflow-auto p-2 custom-scrollbar">
          {facets.map((facet) => {
            if (facet.rows.length === 0 && !facet.error) return null;
            const capped = facet.total > facet.rows.length;
            return (
              <React.Fragment key={facet.key}>
                <div className="flex items-center justify-between px-3 py-2 mt-2 text-[10px] uppercase tracking-widest text-text-muted border-b border-border-subtle">
                  <span className="flex items-center gap-2">
                    <span>{facet.label}</span>
                    {capped ? (
                      <span className="normal-case tracking-normal">
                        {facet.rows.length} of {facet.total}
                      </span>
                    ) : null}
                  </span>
                  <ProvenanceBadge hint={facet.provenanceHint} />
                </div>
                {facet.error ? (
                  <div className="px-3 py-2 text-[11px] text-text-muted italic">
                    {facet.label} unavailable — {facet.error}
                  </div>
                ) : (
                  facet.rows.map((row) => {
                    const idx = rowCursor++;
                    const selected = idx === selectedRowIdx;
                    const KindIcon = Icon[(row.kind && KIND_ICON[row.kind]) || 'search'];
                    return (
                      <button
                        key={row.id}
                        onClick={() => activateRow(row)}
                        onFocus={() => {
                          // WR-03: Tab onto a row makes it the Enter target.
                          idxRef.current = idx;
                          setSelectedRowIdx(idx);
                        }}
                        aria-current={selected ? 'true' : undefined}
                        className={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left transition ${
                          selected ? 'bg-brass/8 border border-brass/20' : 'hover:bg-overlay-subtle'
                        }`}
                      >
                        <div className="flex min-w-0 items-center gap-2.5">
                          <KindIcon className="size-3.5 shrink-0 text-text-muted" aria-hidden="true" />
                          <div className="flex flex-col min-w-0">
                            <span className="flex items-center gap-1.5">
                              <span className="text-[13px] text-text-secondary truncate max-w-[360px]">{row.label}</span>
                              {row.chips?.interfaces.map((chip) => (
                                <span
                                  key={chip}
                                  className="shrink-0 rounded-sm border border-border-subtle bg-overlay-subtle px-1 py-px font-mono text-[9px] tracking-widest text-text-muted"
                                >
                                  {chip}
                                </span>
                              ))}
                              {row.chips?.chord ? (
                                <kbd className="shrink-0 rounded-sm border border-border-subtle bg-overlay-subtle px-1 py-px font-mono text-[10px] text-text-muted">
                                  {row.chips.chord}
                                </kbd>
                              ) : null}
                              {row.chips?.domain ? (
                                <span className="shrink-0 rounded-sm border border-brass/20 px-1 py-px text-[9px] text-text-muted">
                                  {row.chips.domain}
                                </span>
                              ) : null}
                            </span>
                            {row.detail ? (
                              <span className="text-[11px] text-text-muted truncate max-w-[460px]">{row.detail}</span>
                            ) : null}
                          </div>
                        </div>
                        <span className="font-mono text-[9px] uppercase tracking-widest text-text-muted shrink-0 ml-2">
                          {row.activate.type === 'cli' ? (
                            <span className="mr-2 normal-case tracking-normal">
                              {CLI_HINT[cliDecision(row.activate.rowId, row.activate.path).kind]}
                            </span>
                          ) : null}
                          {row.provenance}
                        </span>
                      </button>
                    );
                  })
                )}
                {capped && (facet.key === 'commands' || facet.key === 'tools') ? (
                  <div className="px-3 py-1 text-[11px] text-text-muted italic">{NARROW_HINT}</div>
                ) : null}
              </React.Fragment>
            );
          })}

          {indexError && (
            <div className="flex items-center gap-2 px-3 py-2 text-[11px] text-text-muted italic">
              <span>Capability index unavailable — {indexError}. Showing built-in surfaces and commands only.</span>
              <button
                type="button"
                onClick={retryIndex}
                className="rounded-sm border border-border-subtle px-1.5 py-px not-italic text-text-secondary hover:bg-overlay-subtle"
              >
                Retry
              </button>
            </div>
          )}

          <div
            role="status"
            aria-live="polite"
            className={
              loadingLine
                ? 'px-3 py-2 text-[11px] text-text-muted italic'
                : noMatches
                  ? 'px-3 py-6 text-center text-[12px] text-text-muted'
                  : 'sr-only'
            }
          >
            {statusText}
          </div>
        </div>
      </div>
    </div>
  );
}
