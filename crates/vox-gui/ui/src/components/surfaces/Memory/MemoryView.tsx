import React, { useCallback, useEffect, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useVirtualList } from '../../../hooks/useVirtualList';
import { shardSparkColor } from '../../../lib/visualTokens';
import { invokeCmd, listenScientiaQueue, voxTransport } from '../../../transport';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { useLabel } from '../../../hooks/useLanguage';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';
import { Sparkline } from '../../ui/Sparkline';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { RomanRotaryKnob } from '../../ui/RomanRotaryKnob';
import { renderHighlights, UnifiedHit, SearchResponse } from '../Search/searchHelpers';
import { attachItemsFromHits, AttachItem } from '../../../lib/loquelaContext';
import { MEMORY_RECALL_DEBOUNCE_MS } from '../../../config/constants';
import type { Toast } from '../../../types/tauri';

export interface KnowledgeNodeRow {
  id: string;
  label: string;
  snippet: string;
  node_type?: string | null;
  created_at: string;
}

/** Mirrors the Rust `FindingFiled` DTO: `pending` (new) or `duplicate` (already in the inbox). */
export interface FindingFiled {
  finding_id: string | null;
  status: string;
}

export interface KbHealthDto {
  node_count: number;
  edge_count: number;
  fts_available: boolean;
  corpus_counts: Record<string, number>;
}

// Corpus vocabulary aligned with vox_db SearchCorpus variants.
// Scope ids must match the corpus names accepted by vox_search_query.
const MEM_CORPORA = [
  { id: 'memory',    name: 'Memory'    },
  { id: 'knowledge', name: 'Knowledge' },
  { id: 'chunk',     name: 'Chunk'     },
] as const;

type MemCorpusId = typeof MEM_CORPORA[number]['id'];

function HitCard({
  hit,
  query,
  index,
  onPin,
  onOpen,
  onResearchTopic,
}: {
  hit: UnifiedHit;
  query: string;
  index: number;
  onPin: () => void;
  onOpen: () => void;
  onResearchTopic: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const kindIcon: Record<string, React.ReactNode> = {
    code:   <Icon.file className="size-3.5" />,
    text:   <Icon.catalog className="size-3.5" />,
    chat:   <Icon.bolt className="size-3.5" />,
    policy: <Icon.shield className="size-3.5" />,
    web:    <Icon.link className="size-3.5" />,
  };

  const isOpenable = hit.locator.kind === 'file' || hit.locator.kind === 'web';
  const src = hit.path ?? hit.source;
  const citationKindLabel =
    hit.kind === 'code' || hit.kind === 'policy' ? 'SHARD CITATION · HNSW' : 'CODEX CITATION · SYNC';
  const provenanceLayer =
    hit.source === 'memory'
      ? {
          label: 'Memory Shard',
          tip: 'Indexed from session memory shards (~/.vox/memory + vector HNSW)',
        }
      : hit.source === 'chunk'
        ? {
            label: 'Code Chunk',
            tip: 'Indexed AST/source code chunk from repository crates',
          }
        : {
            label: 'Codex Canon',
            tip: 'Indexed from repository SSOT documentation and promoted KB findings',
          };

  const segments = renderHighlights(hit.snippet, query);

  return (
    <div
      className={`vox-recessed-tablet group flex flex-col gap-2 rounded-lg p-3 transition-colors hover:border-brass ${isOpenable ? 'cursor-pointer' : ''}`}
      onClick={isOpenable ? onOpen : undefined}
    >
      <div className="flex items-center justify-between gap-2 border-b border-border-subtle pb-1.5">
        <div className="flex items-center gap-2 min-w-0">
          <div className="flex size-7 shrink-0 items-center justify-center rounded border border-border-strong bg-bg-elevated text-text-primary">
            {kindIcon[hit.kind] ?? <Icon.file className="size-3.5" />}
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="font-display text-[11px] font-bold uppercase tracking-[0.12em] text-brass">
                {citationKindLabel}
              </span>
              <span
                title={provenanceLayer.tip}
                className="rounded border border-border-subtle bg-bg-elevated px-1.5 py-0 font-mono text-[11px] font-medium text-text-secondary cursor-help"
              >
                {provenanceLayer.label}
              </span>
            </div>
            <div className="font-mono text-xs font-semibold text-text-primary truncate">{src}</div>
          </div>
        </div>
        <div className="flex flex-col items-end gap-1 shrink-0">
          <span className="font-mono text-xs font-bold tabular-nums text-text-primary">
            {(hit.score * 100).toFixed(1)}%
          </span>
          <div className="h-2 w-24 overflow-hidden rounded-full border border-border-strong bg-bg-surface">
            <div
              className="h-full bg-brass"
              style={{ width: `${Math.max(4, Math.min(100, hit.score * 100))}%` }}
            />
          </div>
        </div>
      </div>

      <div
        className={`font-serif text-[13px] leading-relaxed text-text-secondary ${
          expanded ? 'whitespace-pre-wrap' : 'line-clamp-2'
        }`}
      >
        {segments.map((seg, i) =>
          seg.mark ? (
            <mark key={i} className="bg-brass/25 text-text-primary rounded-sm px-0.5">{seg.text}</mark>
          ) : (
            <span key={i}>{seg.text}</span>
          )
        )}
      </div>

      <div className="flex flex-wrap items-center justify-end gap-1 pt-0.5">
        <button
          type="button"
          data-slot="button"
          data-roman-plaque="true"
          data-testid={`expand-hit-${index}`}
          aria-expanded={expanded}
          onClick={e => { e.stopPropagation(); setExpanded(prev => !prev); }}
          className="vox-brass-plaque-btn inline-flex h-7 items-center gap-1 rounded px-2 text-[11px] transition-colors"
        >
          <span>{expanded ? 'Collapse' : 'Expand'}</span>
        </button>
        <button
          type="button"
          data-slot="button"
          data-roman-plaque="true"
          data-testid={`research-hit-${index}`}
          title="Launch deep Research on this memory hit"
          onClick={e => { e.stopPropagation(); onResearchTopic(); }}
          className="vox-brass-plaque-btn inline-flex h-7 items-center gap-1 rounded px-2 text-[11px] transition-colors"
        >
          <Icon.search aria-hidden="true" className="size-3" />
          <span>Research</span>
        </button>
        {isOpenable && (
          <button
            type="button"
            data-slot="button"
            data-roman-plaque="true"
            onClick={e => { e.stopPropagation(); onOpen(); }}
            className="vox-brass-plaque-btn inline-flex h-7 items-center gap-1 rounded px-2 text-[11px] transition-colors"
          >
            <Icon.link aria-hidden="true" className="size-3" />
            <span>Open</span>
          </button>
        )}
        <button
          type="button"
          data-slot="button"
          data-roman-plaque="true"
          onClick={e => { e.stopPropagation(); onPin(); }}
          className="vox-brass-plaque-btn inline-flex h-7 items-center gap-1 rounded px-2 text-[11px] transition-colors"
        >
          <Icon.pin aria-hidden="true" className="size-3" />
          <span>Pin</span>
        </button>
      </div>
    </div>
  );
}

interface MemoryStatusPayload {
  corpus_counts: Record<string, number>;
  shards: Array<{ id: string; depth: number; entries: number; hot: boolean; dirty: boolean; spark: number[] }>;
  recent_recalls: Array<{ q: string; n: number; when: string }>;
  embedding_dim?: number | null;
}

interface MemoryViewProps {
  pushToast: (t: Toast) => void;
  /**
   * Pin recall hits into the shared Loquela context set. When absent (e.g. a
   * standalone render), pinning is unavailable and the controls report that
   * honestly rather than silently no-op'ing.
   */
  onAttachContext?: (items: AttachItem[]) => void;
}

export function MemoryView({ pushToast, onAttachContext }: MemoryViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [memStatus, setMemStatus] = useState<MemoryStatusPayload | null>(null);
  const [query, setQuery] = useState('');
  // Scope uses the corpus vocabulary aligned with vox_search_query.
  // Default: all three memory/knowledge/chunk corpora active.
  const [scope, setScope] = useState<MemCorpusId[]>(['memory', 'knowledge', 'chunk']);
  const [layerScope, setLayerScope] = useState<
    'all' | 'repo_local' | 'global_library' | 'vox_baseline'
  >('all');
  const [topK, setTopK] = useState(8);
  // Auto-recall is a persisted GUI preference (gui.memory.autoRecall), hydrated
  // from the workspace db via get_gui_preference and written back on toggle —
  // the same pattern Settings uses for theme/telemetry/sign.
  const [recallOn, setRecallOn] = useState(false);
  const [hits, setHits] = useState<UnifiedHit[]>([]);
  const [recalling, setRecalling] = useState(false);

  const [activeTab, setActiveTab] = useState<'search' | 'browse' | 'ingest' | 'health'>('search');

  // Browse state
  const [nodes, setNodes] = useState<KnowledgeNodeRow[]>([]);
  const [nodeTypeFilter, setNodeTypeFilter] = useState<string>('all');
  const [page, setPage] = useState<number>(0);
  const [loadingNodes, setLoadingNodes] = useState(false);
  const nodesPrimedRef = useRef(false);

  // Ingest state
  const [ingestUrl, setIngestUrl] = useState('');
  const [ingestingUrl, setIngestingUrl] = useState(false);
  const [ingestTitle, setIngestTitle] = useState('');
  const [ingestContent, setIngestContent] = useState('');
  const [ingestSourceUrl, setIngestSourceUrl] = useState('');
  const [ingestingText, setIngestingText] = useState(false);
  const [ingestScope, setIngestScope] = useState<'repo_local' | 'global_library'>('repo_local');

  // Health state
  const [health, setHealth] = useState<KbHealthDto | null>(null);
  const [loadingHealth, setLoadingHealth] = useState(false);
  const healthPrimedRef = useRef(false);

  const fetchNodes = useCallback(async (selectedType: string, selectedPage: number, opts?: { silent?: boolean }) => {
    const showSpinner = !opts?.silent && !nodesPrimedRef.current;
    if (showSpinner) setLoadingNodes(true);
    try {
      const typeArg = selectedType === 'all' ? null : selectedType;
      const res = await invokeCmd<KnowledgeNodeRow[]>('list_knowledge_nodes', {
        nodeType: typeArg,
        page: selectedPage,
        limit: 50,
      });
      setNodes(res);
      nodesPrimedRef.current = true;
    } catch (err) {
      if (!opts?.silent) {
        pushToast({ tone: 'warn', title: 'Failed to load nodes', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      }
    } finally {
      if (showSpinner) setLoadingNodes(false);
    }
  }, [pushToast]);

  const deleteNode = async (id: string) => {
    try {
      await invokeCmd('delete_knowledge_node', { id });
      pushToast({ tone: 'ok', title: 'Node deleted', body: `Node ${id} deleted`, cause: 'backend-ok' });
      void fetchNodes(nodeTypeFilter, page, { silent: true });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Delete failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const fetchHealth = useCallback(async (opts?: { silent?: boolean }) => {
    const showSpinner = !opts?.silent && !healthPrimedRef.current;
    if (showSpinner) setLoadingHealth(true);
    try {
      const res = await invokeCmd<KbHealthDto>('get_kb_health');
      setHealth(res);
      healthPrimedRef.current = true;
    } catch (err) {
      if (!opts?.silent) {
        pushToast({ tone: 'warn', title: 'Failed to fetch KB health', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      }
    } finally {
      if (showSpinner) setLoadingHealth(false);
    }
  }, [pushToast]);

  const toastFiled = (res: FindingFiled | string, what: string) => {
    if (typeof res === 'string') {
      const scopeLabel = ingestScope === 'global_library' ? 'Global Library' : 'Repo-Local';
      pushToast({
        tone: 'ok',
        title: `Indexed into ${scopeLabel} KB`,
        body: `${what} (${res}) is now searchable and queued for audit review.`,
        cause: 'backend-ok',
      });
      return;
    }
    if (res.status === 'duplicate') {
      pushToast({ tone: 'ok', title: 'Already pending review', body: `${what} is already in the review inbox.`, cause: 'backend-ok' });
    } else {
      pushToast({ tone: 'ok', title: 'Filed for review', body: `${what} is pending review and not yet searchable.`, cause: 'backend-ok' });
    }
  };

  const handleIngestUrl = async () => {
    if (!ingestUrl.trim()) return;
    setIngestingUrl(true);
    try {
      const res = await invokeCmd<FindingFiled | string>('ingest_url', {
        url: ingestUrl.trim(),
        targetScope: ingestScope,
      });
      toastFiled(res, 'The page');
      setIngestUrl('');
      void invokeCmd<MemoryStatusPayload>('get_memory_status').then(setMemStatus).catch(() => {});
      void fetchNodes(nodeTypeFilter, page, { silent: true });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Scrape failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setIngestingUrl(false);
    }
  };

  const handleIngestText = async () => {
    if (!ingestContent.trim()) return;
    setIngestingText(true);
    try {
      const res = await invokeCmd<FindingFiled | string>('ingest_text', {
        title: ingestTitle.trim(),
        content: ingestContent.trim(),
        sourceUrl: ingestSourceUrl.trim() || null,
        targetScope: ingestScope,
      });
      toastFiled(res, 'The text');
      setIngestTitle('');
      setIngestContent('');
      setIngestSourceUrl('');
      void invokeCmd<MemoryStatusPayload>('get_memory_status').then(setMemStatus).catch(() => {});
      void fetchNodes(nodeTypeFilter, page, { silent: true });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Store failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setIngestingText(false);
    }
  };

  useEffect(() => {
    if (activeTab === 'browse') {
      void fetchNodes(nodeTypeFilter, page);
    } else if (activeTab === 'health') {
      void fetchHealth();
    }
  }, [activeTab, nodeTypeFilter, page, fetchNodes, fetchHealth]);

  const recallsRef = useRef<HTMLDivElement>(null);
  const shardsRef = useRef<HTMLDivElement>(null);

  const RECALL_ITEM_HEIGHT = 52; // px
  const RECALL_GAP = 6;          // px
  const SHARD_COLS = 6;
  const SHARD_ROW_HEIGHT = 132;
  const SHARD_ROW_GAP = 12;

  const recentRecalls = memStatus?.recent_recalls ?? [];
  const shards = memStatus?.shards ?? [];
  const shardRowCount = Math.ceil(shards.length / SHARD_COLS) || 0;

  const recallsVL = useVirtualList({
    containerRef: recallsRef,
    count: recentRecalls.length,
    estimateSize: () => RECALL_ITEM_HEIGHT,
    overscan: 3,
  });

  const shardsVL = useVirtualList({
    containerRef: shardsRef,
    count: shardRowCount,
    estimateSize: () => SHARD_ROW_HEIGHT + SHARD_ROW_GAP,
    overscan: 2,
  });

  useEffect(() => {
    let statusPrimed = false;
    const syncMemory = () => {
      invokeCmd<MemoryStatusPayload>('get_memory_status')
        .then((res) => {
          statusPrimed = true;
          setMemStatus(res);
        })
        .catch((err) => {
          if (!statusPrimed) {
            pushToast({ tone: 'error', title: 'Memory status unavailable', body: sanitizeErrorForToast(err), cause: 'backend-error' });
          }
        });
    };
    syncMemory();
    if (embedded) return;
    const refreshActiveTab = () => {
      if (document.visibilityState === 'hidden') return;
      syncMemory();
      if (activeTab === 'browse') void fetchNodes(nodeTypeFilter, page, { silent: true });
      if (activeTab === 'health') void fetchHealth({ silent: true });
    };
    const intervalId = window.setInterval(refreshActiveTab, 15_000);
    const onVisible = () => {
      if (document.visibilityState === 'visible') refreshActiveTab();
    };
    document.addEventListener('visibilitychange', onVisible);
    let unlisten: (() => void) | undefined;
    listenScientiaQueue(refreshActiveTab)
      .then((fn) => { unlisten = fn; })
      .catch(() => { /* outside Tauri — visibility poll covers it */ });
    return () => {
      window.clearInterval(intervalId);
      document.removeEventListener('visibilitychange', onVisible);
      unlisten?.();
    };
  }, [pushToast, embedded, activeTab, nodeTypeFilter, page, fetchNodes, fetchHealth]);

  // Hydrate the persisted auto-recall preference on mount.
  useEffect(() => {
    voxTransport.getGuiPreference('gui.memory.autoRecall')
      .then((v) => { if (v != null) setRecallOn(v === 'true'); })
      .catch(() => { /* no workspace db (e.g. plain browser dev) — keep default off */ });
  }, []);

  const toggleAutoRecall = () => {
    setRecallOn((prev) => {
      const next = !prev;
      voxTransport.setGuiPreference('gui.memory.autoRecall', String(next))
        .catch((err) => pushToast({ tone: 'error', title: 'Could not persist auto-recall', body: sanitizeErrorForToast(err), cause: 'backend-error' }));
      return next;
    });
  };

  const corpora = MEM_CORPORA.map((c) => {
    const counts = memStatus?.corpus_counts;
    let entries = counts?.[c.id] ?? 0;
    if (entries === 0 && counts) {
      if (c.id === 'memory') entries = (counts.chats ?? 0) + (counts.rules ?? 0);
      else if (c.id === 'knowledge') entries = (counts.docs ?? 0) + (counts.web ?? 0);
      else if (c.id === 'chunk') entries = counts.proj ?? 0;
    }
    return { ...c, entries };
  });
  const corpusTotal = corpora.reduce(
    (s, c) => s + (scope.includes(c.id) ? c.entries : 0),
    0
  );
  // Fall back to summing shard entries when corpus_counts is absent or all-zero
  // (backend may omit corpus_counts on a partial status payload).
  const shardTotal = (memStatus?.shards ?? []).reduce((s, sh) => s + sh.entries, 0);
  const totalEntries = corpusTotal > 0 ? corpusTotal : shardTotal;
  const toggleScope = (id: MemCorpusId) =>
    setScope(s => (s.includes(id) ? s.filter(x => x !== id) : [...s, id]));

  const recall = async (q?: string) => {
    const qq = (q ?? query).trim();
    if (!qq) return;
    setRecalling(true);
    setHits([]);

    try {
      // Repointed from mnemosyne_recall to vox_search_query scoped to memory corpora.
      const res = await invokeCmd<SearchResponse>('vox_search_query', {
        query: qq,
        scope: scope.length > 0 ? scope : null,
        limit: topK,
      });
      setHits(res.hits.slice(0, topK));
      pushToast({
        tone: 'ok',
        title: 'Recall complete',
        body: `Top hits across ${scope.length} corpora`,
        cmd: `vox_search_query • "${qq}"`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({ tone: 'error', title: 'Recall backend error', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setRecalling(false);
      invokeCmd<MemoryStatusPayload>('get_memory_status').then(setMemStatus).catch(() => {});
    }
  };

  // Auto-recall behavior is gated on the persisted preference: when enabled,
  // edits to the query (or active scope) debounce-trigger a recall instead of
  // requiring an explicit Enter / Recall click.
  useEffect(() => {
    if (!recallOn) return;
    const q = query.trim();
    if (!q) return;
    const t = setTimeout(() => { recall(q); }, MEMORY_RECALL_DEBOUNCE_MS);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- debounce recall on query/scope/topK changes
  }, [query, recallOn, scope.join(','), topK]);

  // Pin recall hits into the shared Loquela context. Only file/web hits carry
  // an attachable locator; if none do, tell the user instead of pretending.
  const pinHits = (hitsToPin: UnifiedHit[]) => {
    const items = attachItemsFromHits(hitsToPin);
    if (items.length === 0) {
      pushToast({
        tone: 'warn',
        title: 'Nothing to pin',
        body: 'These hits have no file/web locator the agent can read.',
        cause: 'validation',
      });
      return;
    }
    if (!onAttachContext) {
      pushToast({
        tone: 'warn',
        title: 'Context unavailable',
        body: 'Pin-to-context is only available inside the main shell.',
        cause: 'validation',
      });
      return;
    }
    onAttachContext(items);
  };

  const openHit = async (hit: UnifiedHit) => {
    const fileTarget = hit.locator.value || hit.path || '';
    if (
      hit.locator.kind === 'file' &&
      /\.(md|mdx|markdown|txt|adoc|asciidoc|json|jsonl|yaml|yml|toml|csv|tsv|vox|rs|ts|tsx|sql)$/i.test(
        fileTarget,
      )
    ) {
      window.dispatchEvent(
        new CustomEvent('vox://open-doc', {
          detail: { path: fileTarget, title: hit.title ?? undefined },
        }),
      );
      return;
    }
    if (hit.locator.kind === 'file' || hit.locator.kind === 'web') {
      try {
        await voxTransport.openLocator(hit.locator);
      } catch (err) {
        pushToast({ tone: 'error', title: 'Could not open', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      }
    }
  };

  return (
    <div className="@container/surface flex min-h-0 min-w-0 flex-col gap-3">
      {/* Roman Bas-Relief Surface Header Frieze with Dual Chronometers & Central Coin Signet */}
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE · MEMORY"
        title="Codex Vector Shards & Context Ledger"
        subtitle="Inspect persistent vector index shards, embedding density, and context retrieval telemetry."
        waxSealBadge="CODEX SYNCED"
        icon={<Icon.memory className="size-5 text-text-primary" />}
        gauges={[
          {
            label: 'SHARDS',
            valueText: `${(memStatus?.shards ?? []).length}`,
            ratio: Math.max(0, Math.min(1, (memStatus?.shards ?? []).length / 16)),
            tone: 'gold',
            purpose: 'Active HNSW vector partitions in local database',
            actionHint: 'Jump to live HNSW vector shards',
            active: activeTab === 'search',
            onClick: () => {
              setActiveTab('search');
              requestAnimationFrame(() => {
                shardsRef.current?.scrollIntoView?.({ behavior: 'smooth', block: 'nearest' });
              });
            },
          },
          {
            label: 'ENTRIES',
            valueText:
              (totalEntries ?? 0) > 1000
                ? `${((totalEntries ?? 0) / 1000).toFixed(1)}k`
                : `${totalEntries ?? 0}`,
            ratio: Math.max(0, Math.min(1, (totalEntries ?? 0) / 10000)),
            tone: 'emerald',
            purpose: 'Total indexed vector and symbolic memory entries',
            actionHint: 'Browse indexed knowledge graph nodes',
            active: activeTab === 'browse',
            onClick: () => setActiveTab('browse'),
          },
          {
            label: 'CORPORA',
            valueText: `${scope.length}/${MEM_CORPORA.length}`,
            ratio: Math.max(0, Math.min(1, scope.length / MEM_CORPORA.length)),
            tone: 'gold',
            purpose: 'Active recall search corpora',
            actionHint: 'Focus and enable all recall corpora',
            active: activeTab === 'search' && scope.length === MEM_CORPORA.length,
            onClick: () => {
              setActiveTab('search');
              setScope(MEM_CORPORA.map((c) => c.id));
            },
          },
        ]}
        actions={
          <div className="flex flex-wrap items-center gap-1.5">
            <button
              type="button"
              onClick={toggleAutoRecall}
              aria-pressed={recallOn}
              data-slot="button"
              data-roman-plaque="true"
              title={recallOn ? 'Auto-recall on — queries recall as you type' : 'Auto-recall off — press Enter or Recall'}
              className="vox-brass-plaque-btn inline-flex h-8 items-center gap-1.5 rounded-md px-3 text-xs transition-colors"
            >
              <Icon.eye aria-hidden="true" className="size-3.5" />
              <span>Auto-recall</span>
            </button>
            <button
              type="button"
              data-slot="button"
              data-roman-plaque="true"
              onClick={() =>
                invokeCmd('mnemosyne_reindex')
                  .then(() => pushToast({ tone: 'ok', title: 'Reindex complete', cmd: 'mnemosyne reindex', cause: 'backend-ok' }))
                  .catch((err) => pushToast({ tone: 'error', title: 'Reindex failed', body: sanitizeErrorForToast(err), cause: 'backend-error' }))
              }
              className="vox-brass-plaque-btn inline-flex h-8 items-center gap-1.5 rounded-md px-3 text-xs transition-colors"
            >
              <Icon.refresh aria-hidden="true" className="size-3.5" />
              <span>Reindex</span>
            </button>
            <button
              type="button"
              data-slot="button"
              data-roman-plaque="true"
              data-testid="memory-to-research-btn"
              onClick={() => {
                try {
                  if (query.trim()) {
                    window.localStorage.setItem('vox_research_seed_query', query.trim());
                  }
                } catch {
                  /* localStorage unavailable */
                }
                window.dispatchEvent(
                  new CustomEvent('vox://navigate-surface', {
                    detail: { view: 'research' },
                  }),
                );
              }}
              className="vox-brass-plaque-btn inline-flex h-8 items-center gap-1.5 rounded-md px-3 text-xs transition-colors"
            >
              <Icon.search aria-hidden="true" className="size-3.5" />
              <span>To Research</span>
            </button>
          </div>
        }
      />

      {/* Unified Sculpted Command Console Bar (Search + 4 Brass Corpus/Recall Plaques + Section Segmented Bar) */}
      <div className="vox-recessed-console-frame p-3 space-y-2.5">
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex min-w-[13rem] flex-1 items-center gap-2 rounded-md border border-border-strong bg-bg-surface px-3 h-9 text-text-primary shadow-[inset_0_1px_4px_rgba(120,83,18,0.12)]">
            <Icon.search aria-hidden="true" className="size-4 shrink-0 text-brass" />
            <input
              value={query}
              onChange={e => setQuery(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && recall()}
              aria-label="Recall query"
              placeholder="Recall… e.g. 'ed25519 invariants', 'checkpoint stall'"
              className="h-8 min-w-[90px] flex-1 bg-transparent font-serif text-sm font-medium text-text-primary placeholder:text-text-muted outline-hidden"
            />
          </div>

          <button
            type="button"
            data-slot="button"
            data-roman-plaque="true"
            onClick={() => recall()}
            disabled={!query.trim() || recalling}
            className="vox-brass-plaque-btn inline-flex h-9 items-center gap-1.5 rounded-md px-3.5 text-xs transition-all disabled:opacity-55"
          >
            <Icon.bolt aria-hidden="true" className="size-3.5 shrink-0" />
            <span>{recalling ? 'Recalling…' : 'Recall'}</span>
          </button>

          <div role="tablist" aria-label="Knowledge Base sections" className="vox-segmented-brass-bar shrink-0">
            {(['search', 'browse', 'ingest', 'health'] as const).map((tab) => (
              <button
                key={tab}
                type="button"
                role="tab"
                data-slot="tab"
                aria-selected={activeTab === tab}
                data-testid={`tab-${tab}`}
                onClick={() => setActiveTab(tab)}
                className="inline-flex h-9 items-center px-3 font-display text-xs font-semibold uppercase tracking-wider transition-colors"
              >
                {tab === 'search' ? 'Search' : tab === 'browse' ? 'Nodes' : tab === 'ingest' ? 'Ingest' : 'Health'}
              </button>
            ))}
          </div>
        </div>

        {/* 3-Layer Federated KB Scope Selector (Repo-Local, Global Library, Vox Baseline Canon) */}
        <div
          role="group"
          aria-label="Federated KB layer scope"
          data-testid="kb-layer-scope-bar"
          className="vox-recessed-tablet flex flex-wrap items-center justify-between gap-2 px-3 py-2"
        >
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="font-display text-[11px] font-bold uppercase tracking-wider text-text-secondary mr-1">
              KB Layer
            </span>
            {([
              { id: 'all', label: 'All Layers', corpora: ['memory', 'knowledge', 'chunk'] as MemCorpusId[] },
              { id: 'repo_local', label: 'Repo-Local', corpora: ['memory', 'knowledge', 'chunk'] as MemCorpusId[] },
              { id: 'global_library', label: 'Global Library', corpora: ['memory', 'knowledge'] as MemCorpusId[] },
              { id: 'vox_baseline', label: 'Vox Baseline Canon', corpora: ['knowledge'] as MemCorpusId[] },
            ] as const).map((layer) => {
              const active = layerScope === layer.id;
              return (
                <button
                  key={layer.id}
                  type="button"
                  data-slot="button"
                  data-roman-plaque="true"
                  aria-pressed={active}
                  data-testid={`kb-layer-${layer.id}`}
                  onClick={() => {
                    setLayerScope(layer.id);
                    setScope([...layer.corpora]);
                  }}
                  className={`vox-brass-plaque-btn inline-flex h-8 items-center rounded-md px-2.5 text-[11px] transition-colors ${
                    active ? 'ring-2 ring-brass/70' : 'opacity-80'
                  }`}
                >
                  <span>{layer.label}</span>
                </button>
              );
            })}
          </div>
          <span className="font-mono text-[11px] text-text-muted">
            {layerScope === 'vox_baseline'
              ? 'Canon baseline (docs/src)'
              : layerScope === 'global_library'
                ? 'Cross-repo library (~/.vox/library)'
                : layerScope === 'repo_local'
                  ? 'Repo-isolated corpus (docs/wiki + local DB)'
                  : 'Federated: Canon + Global + Repo'}
          </span>
        </div>

        {/* 2-Column Recessed Architectural Console for Search & Recall */}
        {activeTab === 'search' && (() => {
          const avgSpark = Array.from({ length: 12 }, (_, idx) => {
            if (shards.length === 0) return 3 + Math.round(Math.sin(idx * 0.7) * 2 + 2);
            const sh = shards[idx % shards.length]!;
            const pts = sh.spark ?? [2, 4, 3, 5];
            return pts[idx % pts.length] ?? 2;
          });
          const primaryWaveD = avgSpark
            .map((v, i) => `${i === 0 ? 'M' : 'L'} ${(i * 22 + 8).toFixed(1)} ${(48 - Math.min(40, v * 4)).toFixed(1)}`)
            .join(' ');
          const secondaryWaveD = avgSpark
            .map((v, i) => `${i === 0 ? 'M' : 'L'} ${(i * 22 + 8).toFixed(1)} ${(44 - Math.min(34, v * 3.1 + ((i % 3) * 2))).toFixed(1)}`)
            .join(' ');
          const taxonomyRows = [
            { id: 'memory' as const, name: 'Architecture Specs · Memory', entries: corpora[0]?.entries ?? 0, last: false },
            { id: 'chunk' as const, name: 'Rust Crates · Chunk', entries: corpora[2]?.entries ?? 0, last: false },
            { id: 'knowledge' as const, name: 'Codex Docs · Knowledge', entries: corpora[1]?.entries ?? 0, last: true },
          ];

          return (
            <div className="grid grid-cols-1 @3xl/surface:grid-cols-12 gap-4 items-start">
              {/* LEFT RECESSED TABLET: CORPUS TAXONOMY, TOP-K KNOB & MEMORY SHARDS */}
              <div className="vox-recessed-tablet @3xl/surface:col-span-4 p-4 space-y-4">
                <div className="flex flex-col items-center gap-1.5">
                  <h3 className="font-display text-xs font-bold uppercase tracking-[0.15em] text-text-primary text-center">
                    Corpus Taxonomy &amp; HNSW Shards
                  </h3>
                  <div className="vox-rule w-full" aria-hidden="true" />
                </div>

                {/* Hierarchical Taxonomy Tree with SVG Brass Connectors */}
                <div className="rounded-lg border border-border-strong bg-bg-surface p-2.5 space-y-1">
                  <div className="font-display text-[11px] font-bold uppercase tracking-wider text-brass px-1 pb-1">
                    Corpora · {scope.length}/{MEM_CORPORA.length} Active
                  </div>
                  {taxonomyRows.map((row) => {
                    const isOn = scope.includes(row.id);
                    const rowDesc =
                      row.id === 'memory'
                        ? 'Architecture notes, durable rules, and session context embeddings'
                        : row.id === 'chunk'
                          ? 'Indexed workspace Rust/TypeScript AST code chunks and symbol maps'
                          : 'Canonical reference docs (vox_baseline), personal library, and wiki syntheses';
                    return (
                      <button
                        key={row.id}
                        type="button"
                        title={rowDesc}
                        onClick={() => toggleScope(row.id)}
                        className={`flex w-full items-center justify-between gap-2 rounded-md border px-2.5 py-1.5 text-left transition-colors ${
                          isOn
                            ? 'border-brass/65 bg-bg-elevated text-text-primary font-semibold'
                            : 'border-transparent bg-transparent text-text-secondary hover:bg-bg-elevated/60'
                        }`}
                      >
                        <span className="flex items-center gap-2 min-w-0">
                          <svg viewBox="0 0 14 16" className="size-3.5 shrink-0 text-brass" aria-hidden="true">
                            <line x1="5" y1="0" x2="5" y2={row.last ? 9 : 16} stroke="currentColor" strokeWidth="1.4" />
                            <line x1="5" y1="9" x2="13" y2="9" stroke="currentColor" strokeWidth="1.4" />
                          </svg>
                          <span className="truncate font-serif text-xs">{row.name}</span>
                        </span>
                        <span className="shrink-0 font-mono text-[11px] text-brass">
                          {(row.entries ?? 0).toLocaleString()}
                        </span>
                      </button>
                    );
                  })}
                </div>

                {/* Physical Knurled Brass TOP-K Knob */}
                <div
                  className="vox-recessed-subwell rounded-xl p-3 space-y-2"
                  title="Top-K controls how many highest-ranking vector + lexical chunks merge into retrieval context; 8–15 balances precision vs token budget"
                >
                  <div className="flex items-center justify-around gap-3">
                    <RomanRotaryKnob
                      ariaLabel="Top-K retrieval depth"
                      label="Top-K Depth"
                      value={topK}
                      min={1}
                      max={50}
                      step={1}
                      detents={[5, 8, 15, 25, 50]}
                      size="lg"
                      formatValue={(v) => `${v} hits`}
                      onValueChange={(v) => setTopK(v)}
                    />
                    <div className="flex flex-col items-end text-right gap-1">
                      <span className="font-display text-[11px] font-bold uppercase tracking-wider text-text-muted">
                        Vector Dim
                      </span>
                      <span className="font-mono text-sm font-bold text-text-primary">
                        {memStatus?.embedding_dim ?? 768}d
                      </span>
                      <span className="font-mono text-[11px] text-brass">
                        {(totalEntries ?? 0).toLocaleString()} vectors
                      </span>
                    </div>
                  </div>
                  <p className="font-serif text-[11px] leading-snug text-text-muted text-center">
                    Turn dial to adjust retrieval chunk count (8–15 recommended for synthesis).
                  </p>
                </div>

                {/* Recent Recalls Ledger */}
                <div className="rounded-lg border border-border-strong bg-bg-surface p-2.5 space-y-2">
                  <div className="flex items-center justify-between gap-2">
                    <h3 className="font-display text-xs font-bold uppercase tracking-[0.14em] text-text-secondary">
                      Recent recalls
                    </h3>
                    <Icon.clock className="size-3.5 shrink-0 text-text-muted" />
                  </div>
                  {recentRecalls.length === 0 ? (
                    <div className="rounded-lg border border-dashed border-border-subtle bg-overlay-subtle p-3 text-center font-mono text-[11px] text-text-muted">
                      No recall queries recorded in this session.
                    </div>
                  ) : (
                    <BentoScrollRegion
                      ref={recallsRef}
                      ariaLabel="Recent recall queries"
                      style={{
                        height: Math.min(Math.max(recentRecalls.length, 1) * (RECALL_ITEM_HEIGHT + RECALL_GAP), 210),
                      }}
                    >
                      <div style={{ height: recallsVL.totalSize, position: 'relative' }}>
                        {recallsVL.virtualItems.map(vItem => {
                          const r = recentRecalls[vItem.index];
                          return (
                            <div
                              key={String(vItem.key)}
                              ref={recallsVL.virtualizer.measureElement}
                              data-index={vItem.index}
                              style={{
                                position: 'absolute',
                                top: 0,
                                transform: `translateY(${vItem.start}px)`,
                                width: '100%',
                                paddingBottom: RECALL_GAP,
                              }}
                            >
                              <button
                                type="button"
                                onClick={() => { setQuery(r.q); recall(r.q); }}
                                aria-label={`Re-run recall: ${r.q}`}
                                className="flex w-full items-center justify-between rounded-md border border-border-subtle bg-bg-surface px-3 py-2 text-left hover:border-brass/50 hover:bg-bg-elevated transition-colors"
                              >
                                <div className="min-w-0">
                                  <div className="truncate text-xs font-medium text-text-primary">{r.q}</div>
                                  <div className="font-mono text-[11px] text-text-muted">{r.n} hits · {r.when} ago</div>
                                </div>
                                <Icon.chevR className="size-3.5 text-text-muted shrink-0" />
                              </button>
                            </div>
                          );
                        })}
                      </div>
                    </BentoScrollRegion>
                  )}
                </div>

                {/* Bottom Framed Memory Shards & Dual-Waveform Oscilloscope Chart */}
                <div className="vox-recessed-subwell rounded-xl p-3 space-y-2.5">
                  <div className="flex items-center justify-between gap-2">
                    <h3 className="font-display text-xs font-bold uppercase tracking-[0.14em] text-text-primary">
                      {useLabel('mem-shards')}
                    </h3>
                    <span className="font-mono text-[11px] text-text-secondary">
                      {(memStatus?.shards ?? []).length} live · HNSW
                    </span>
                  </div>
                  <div
                    data-cv-chart="true"
                    className="rounded-md border border-border-strong bg-bg-surface p-2"
                  >
                    <svg viewBox="0 0 264 56" className="w-full h-14 overflow-visible" aria-hidden="true">
                      {[14, 28, 42].map((y) => (
                        <line key={y} x1="4" y1={y} x2="260" y2={y} stroke="var(--color-border-subtle)" strokeWidth="0.7" strokeDasharray="3 3" />
                      ))}
                      <path d={secondaryWaveD} fill="none" stroke="var(--color-status-pass)" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                      <path d={primaryWaveD} fill="none" stroke="#C59B27" strokeWidth="2.0" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                  </div>
                </div>
              </div>

              {/* RIGHT RECESSED TABLET: MNEMOSYNE · CITATIONS GRID & CODEX READER */}
              <div className="vox-recessed-tablet @3xl/surface:col-span-8 p-4 space-y-4">
                <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2.5">
                  <div className="flex flex-wrap items-center gap-2 min-w-0">
                    <h2 className="font-display text-sm font-bold uppercase tracking-[0.14em] text-text-primary">
                      Mnemosyne · Citations &amp; Codex Reader
                    </h2>
                    <span className="font-mono text-xs text-text-secondary">
                      {hits.length > 0 ? `${hits.length} matched hits` : `${(totalEntries ?? 0).toLocaleString()} indexed entries`}
                    </span>
                  </div>
                  {hits.length > 0 && (
                    <button
                      type="button"
                      data-slot="button"
                      data-roman-plaque="true"
                      onClick={() => pinHits(hits)}
                      className="vox-brass-plaque-btn inline-flex h-8 items-center gap-1.5 rounded-md px-3 text-xs transition-colors"
                    >
                      <Icon.pin aria-hidden="true" className="size-3" />
                      <span>Pin all to context</span>
                    </button>
                  )}
                </div>

                <div>
                  <h3 className="mb-2 font-display text-xs font-bold uppercase tracking-[0.16em] text-text-secondary">
                    Citations {hits.length > 0 && <span className="text-text-muted">· {hits.length}</span>}
                  </h3>
                  <BentoScrollRegion
                    ariaLabel="Recall citations"
                    aria-live="polite"
                    aria-busy={recalling}
                    className="max-h-[340px] pr-1"
                  >
                    {recalling && (
                      <div className="grid grid-cols-1 @xl/surface:grid-cols-2 gap-2.5">
                        {Array.from({ length: 4 }).map((_, i) => (
                          <div key={i} className="h-24 rounded-lg border border-border-subtle bg-overlay-subtle relative overflow-hidden">
                            <span className="absolute inset-0 -translate-x-full animate-vox-shimmer bg-linear-to-r from-transparent via-white/5 to-transparent" />
                          </div>
                        ))}
                      </div>
                    )}
                    {!recalling && hits.length === 0 && (
                      <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-border-strong bg-bg-surface py-8 text-center">
                        <Icon.memory className="size-6 text-brass mb-1.5" />
                        <div className="font-display text-xs font-semibold tracking-wider text-text-primary">No recall yet</div>
                        <div className="font-serif text-xs text-text-secondary mt-0.5">Type a query or click a recent recall</div>
                      </div>
                    )}
                    {!recalling && hits.length > 0 && (
                      <div className="grid grid-cols-1 @2xl/surface:grid-cols-2 gap-2.5">
                        {hits.map((h, i) => (
                          <HitCard
                            key={i}
                            hit={h}
                            query={query}
                            index={i}
                            onOpen={() => openHit(h)}
                            onPin={() => pinHits([h])}
                            onResearchTopic={() => {
                              const seed = (query.trim() || h.path || h.source || h.snippet.slice(0, 120)).trim();
                              try {
                                window.localStorage.setItem('vox_research_seed_query', seed);
                              } catch {
                                /* ignore storage unavailable */
                              }
                              window.dispatchEvent(
                                new CustomEvent('vox://navigate-surface', { detail: { view: 'research' } }),
                              );
                            }}
                          />
                        ))}
                      </div>
                    )}
                  </BentoScrollRegion>
                </div>

                {/* Codex Specification Reader & Chiseled Bronze INVARIANT Plaque */}
                <details className="group vox-recessed-subwell rounded-xl px-3.5 py-2.5">
                  <summary className="flex cursor-pointer flex-wrap items-center justify-between gap-2 select-none">
                    <div className="flex items-center gap-2">
                      <span className="vox-invariant-plaque inline-flex items-center rounded px-2.5 py-0.5 text-[11px]">
                        INVARIANT
                      </span>
                      <span className="font-display text-xs font-bold uppercase tracking-[0.12em] text-text-primary">
                        Codex Specification · Hybrid HNSW &amp; RRF Grounding
                      </span>
                    </div>
                    <span
                      className="font-mono text-[11px] text-text-secondary"
                      title="Reciprocal Rank Fusion merges keyword BM25 matches and dense vector similarity using constant smoothing k=60"
                    >
                      BM25 + {memStatus?.embedding_dim ?? 768}d Vector RRF (k=60)
                    </span>
                  </summary>
                  <p className="mt-2.5 border-t border-border-strong/45 pt-2 font-serif text-xs leading-relaxed text-text-secondary">
                    Recall merges dense HNSW vector shards with Tantivy BM25 lexical indices via Reciprocal Rank Fusion (<code className="font-mono font-semibold text-text-primary">rrf_k=60</code>), preserving file-locator ground truth across workspace rules, code chunks, and verified research nodes.
                  </p>
                </details>

                {/* Live HNSW Shard Ledger Strip */}
                <details open className="group rounded-lg border border-border-strong bg-bg-surface p-3 space-y-3">
                  <summary className="flex cursor-pointer flex-wrap items-center justify-between gap-2 select-none font-display text-xs font-bold uppercase tracking-[0.14em] text-text-secondary">
                    <span>Mounted Vector Shards ({(memStatus?.shards ?? []).length} HNSW Partitions)</span>
                    <Icon.chevR className="size-3.5 text-text-muted transition-transform rotate-90 group-open:-rotate-90" aria-hidden="true" />
                  </summary>
                  {shards.length === 0 ? (
                    <div
                      ref={shardsRef}
                      className="rounded-lg border border-dashed border-border-subtle bg-bg-surface p-4 text-center text-xs text-text-muted"
                    >
                      No active HNSW vector shards mounted in this workspace session.
                    </div>
                  ) : (
                    <BentoScrollRegion
                      ref={shardsRef}
                      ariaLabel="Memory vector shards"
                      className="max-h-[340px]"
                    >
                      <div style={{ height: shardsVL.totalSize, position: 'relative' }}>
                        {shardsVL.virtualItems.map(vItem => {
                          const rowStart = vItem.index * SHARD_COLS;
                          const rowShards = shards.slice(rowStart, rowStart + SHARD_COLS);
                          return (
                            <div
                              key={String(vItem.key)}
                              ref={shardsVL.virtualizer.measureElement}
                              data-index={vItem.index}
                              style={{
                                position: 'absolute',
                                top: 0,
                                transform: `translateY(${vItem.start}px)`,
                                width: '100%',
                                paddingBottom: SHARD_ROW_GAP,
                              }}
                            >
                              <div className="grid grid-cols-1 gap-2.5 @sm/surface:grid-cols-2 @2xl/surface:grid-cols-3 @5xl/surface:grid-cols-4">
                                {rowShards.map(s => (
                                  <div
                                    key={s.id}
                                    data-slot="shard-card"
                                    className={`min-w-0 overflow-hidden rounded-lg border p-2.5 transition-colors hover:border-brass/50 ${
                                      s.hot   ? 'border-brass/50 bg-bg-elevated' :
                                      s.dirty ? 'border-(--color-status-warn)/40 bg-bg-elevated' :
                                                'border-border-subtle bg-bg-surface'
                                    }`}
                                  >
                                    <div className="flex flex-wrap items-center justify-between gap-1">
                                      <span className="truncate font-mono text-xs font-medium text-text-primary">shard-{s.id}</span>
                                      {s.hot   && <span className="rounded-full border border-brass/35 bg-bg-surface px-2 py-0.5 font-display text-[11px] uppercase tracking-wider text-brass">hot</span>}
                                      {s.dirty && <span className="rounded-full border border-(--color-status-warn)/35 bg-bg-surface px-2 py-0.5 font-display text-[11px] uppercase tracking-wider text-(--color-status-warn)">dirty</span>}
                                    </div>
                                    <div className="mt-1.5 grid grid-cols-2 gap-1.5 text-[11px]">
                                      <div className="min-w-0 rounded border border-border-subtle bg-bg-elevated px-2 py-1">
                                        <div className="font-sans text-[11px] uppercase tracking-wider text-text-muted">Depth</div>
                                        <div className="mt-0.5 font-mono text-xs text-text-primary">{s.depth}</div>
                                      </div>
                                      <div className="min-w-0 rounded border border-border-subtle bg-bg-elevated px-2 py-1">
                                        <div className="font-sans text-[11px] uppercase tracking-wider text-text-muted">Entries</div>
                                        <div className="mt-0.5 font-mono text-xs text-text-primary">{(s.entries ?? 0).toLocaleString()}</div>
                                      </div>
                                    </div>
                                    <div className="mt-2 h-8 min-w-0 overflow-hidden">
                                      <Sparkline
                                        data={s.spark}
                                        color={shardSparkColor(s)}
                                        width={160}
                                        height={28}
                                      />
                                    </div>
                                  </div>
                                ))}
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </BentoScrollRegion>
                  )}
                </details>
              </div>
            </div>
          );
        })()}
      </div>

      {/* Browse Tab */}
      {activeTab === 'browse' && (
        <BentoCard className="gap-4 p-4 @2xl/surface:p-5">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border-subtle pb-4">
            <div className="flex flex-wrap items-center gap-3 min-w-0">
              <h3 className="font-display text-[14px] font-semibold text-text-primary">Knowledge Graph Nodes</h3>
              <VoxSelect
                aria-label="Filter by node type"
                value={nodeTypeFilter}
                onValueChange={(v) => {
                  setNodeTypeFilter(v);
                  setPage(0);
                }}
                options={[
                  { value: 'all', label: 'All Node Types' },
                  { value: 'research_synthesis', label: 'Research Synthesis' },
                  { value: 'web_research_source', label: 'Web Source' },
                  { value: 'document', label: 'Document' },
                  { value: 'web_scrape', label: 'Web Scrape' },
                ]}
                className="w-full @sm/surface:w-44 border-border-subtle bg-overlay-subtle text-xs text-text-secondary"
              />
            </div>
          </div>

          <BentoScrollRegion ariaLabel="Knowledge graph nodes table" className="max-h-[520px]">
            {loadingNodes ? (
              <div className="py-8 text-center text-xs text-text-muted">Loading knowledge nodes…</div>
            ) : nodes.length === 0 ? (
              <div className="py-8 text-center text-xs text-text-muted">No knowledge nodes found for this criteria.</div>
            ) : (
              <table className="w-full text-left text-xs">
                <thead>
                  <tr className="border-b border-border-subtle text-[11px] text-text-muted">
                    <th className="pb-2 font-medium">Label / ID</th>
                    <th className="pb-2 font-medium">Type</th>
                    <th className="pb-2 font-medium">Snippet Preview</th>
                    <th className="pb-2 font-medium">Created</th>
                    <th className="pb-2 text-right font-medium">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle/50">
                  {nodes.map((n) => (
                    <tr key={n.id} className="hover:bg-overlay-subtle/40 transition">
                      <td className="py-2.5 pr-3 font-mono">
                        <div className="font-semibold text-text-primary text-xs">{n.label}</div>
                        <div className="text-[11px] text-text-muted truncate max-w-xs">{n.id}</div>
                      </td>
                      <td className="py-2.5 pr-3">
                        <span className="rounded border border-brass/35 bg-bg-elevated px-2 py-0.5 text-[11px] text-brass">
                          {n.node_type || 'untyped'}
                        </span>
                      </td>
                      <td className="py-2.5 pr-3 text-xs text-text-secondary line-clamp-2 max-w-md">
                        {n.snippet}
                      </td>
                      <td className="py-2.5 pr-3 text-text-muted whitespace-nowrap text-[11px]">
                        {n.created_at}
                      </td>
                      <td className="py-2.5 text-right whitespace-nowrap">
                        <button
                          type="button"
                          data-slot="button"
                          data-testid={`delete-node-${n.id}`}
                          onClick={() => void deleteNode(n.id)}
                          title="Delete knowledge node and associated edges"
                          className="inline-flex h-8 items-center gap-1 rounded border border-(--color-status-fail)/35 bg-bg-elevated px-2.5 text-xs text-(--color-status-fail) hover:border-(--color-status-fail) transition-colors"
                        >
                          <Icon.trash className="size-3" />
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </BentoScrollRegion>

          <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border-subtle pt-3 text-xs text-text-muted">
            <button
              type="button"
              data-slot="button"
              disabled={page === 0}
              onClick={() => setPage((p) => Math.max(0, p - 1))}
              className="h-8 rounded border border-border-subtle bg-bg-elevated px-3 text-xs font-medium text-text-primary disabled:opacity-40"
            >
              Previous
            </button>
            <span>Page {page + 1}</span>
            <button
              type="button"
              data-slot="button"
              disabled={nodes.length < 50}
              onClick={() => setPage((p) => p + 1)}
              className="h-8 rounded border border-border-subtle bg-bg-elevated px-3 text-xs font-medium text-text-primary disabled:opacity-40"
            >
              Next
            </button>
          </div>
        </BentoCard>
      )}

      {/* Ingest Tab */}
      {activeTab === 'ingest' && (
        <div className="space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-subtle bg-bg-surface px-3 py-2 text-xs">
            <span className="font-mono text-xs text-text-secondary">Target Knowledgebase Scope:</span>
            <button
              type="button"
              data-slot="button"
              data-testid="memory-ingest-scope-toggle"
              aria-pressed={ingestScope === 'global_library'}
              onClick={() =>
                setIngestScope((prev) => (prev === 'repo_local' ? 'global_library' : 'repo_local'))
              }
              className="h-8 rounded border border-brass/50 bg-bg-elevated px-3 font-mono text-[11px] font-semibold text-text-primary hover:bg-brass/15"
            >
              {ingestScope === 'repo_local'
                ? 'Scope: Repo-Local (docs/wiki/)'
                : 'Scope: Global Library (~/.vox/library/)'}
            </button>
          </div>
          <BentoGrid preset="balanced" className="@3xl/surface:grid-cols-2 @5xl/surface:grid-cols-2">
          <BentoCard className="p-4 @2xl/surface:p-5">
            <h3 className="font-display text-[14px] font-semibold text-text-primary flex items-center gap-2">
              <Icon.globe className="size-4 shrink-0 text-brass" />
              Scrape &amp; Ingest URL
            </h3>
            <p className="mt-1 text-xs text-text-muted">
              Extract and clean webpage markdown using the native Vox search scraper (respecting robots.txt) and file it in the review inbox. It is not searchable until reviewed.
            </p>
            <div className="mt-4 space-y-3">
              <div>
                <label className="block text-[11px] font-medium text-text-secondary mb-1">Target URL</label>
                <input
                  type="url"
                  placeholder="https://example.com/article"
                  value={ingestUrl}
                  onChange={(e) => setIngestUrl(e.target.value)}
                  className="w-full rounded-md border border-border-subtle bg-overlay-subtle px-3 py-2 text-xs text-text-primary placeholder:text-text-muted outline-hidden"
                />
              </div>
              <button
                type="button"
                data-testid="ingest-url-btn"
                disabled={ingestingUrl || !ingestUrl.trim()}
                onClick={() => void handleIngestUrl()}
                className="inline-flex items-center gap-1.5 rounded-md border border-brass/40 bg-brass/15 px-3 py-1.5 text-xs font-medium text-brass hover:bg-brass/25 disabled:opacity-40 transition"
              >
                {ingestingUrl ? 'Scraping & Ingesting…' : 'Fetch & Store into KB'}
              </button>
            </div>
          </BentoCard>

          <BentoCard className="p-4 @2xl/surface:p-5">
            <h3 className="font-display text-[14px] font-semibold text-text-primary flex items-center gap-2">
              <Icon.edit className="size-4 shrink-0 text-brass" />
              Direct Text / Markdown Ingest
            </h3>
            <p className="mt-1 text-xs text-text-muted">
              File notes, reference documents, or manual excerpts in the review inbox. They are not searchable until reviewed.
            </p>
            <div className="mt-4 space-y-3">
              <div>
                <label className="block text-[11px] font-medium text-text-secondary mb-1">Title / Label</label>
                <input
                  type="text"
                  placeholder="Architecture invariants overview"
                  value={ingestTitle}
                  onChange={(e) => setIngestTitle(e.target.value)}
                  className="w-full rounded-md border border-border-subtle bg-overlay-subtle px-3 py-2 text-xs text-text-primary placeholder:text-text-muted outline-hidden"
                />
              </div>
              <div>
                <label className="block text-[11px] font-medium text-text-secondary mb-1">Source URL (Optional)</label>
                <input
                  type="url"
                  placeholder="https://internal.wiki/page"
                  value={ingestSourceUrl}
                  onChange={(e) => setIngestSourceUrl(e.target.value)}
                  className="w-full rounded-md border border-border-subtle bg-overlay-subtle px-3 py-2 text-xs text-text-primary placeholder:text-text-muted outline-hidden"
                />
              </div>
              <div>
                <label className="block text-[11px] font-medium text-text-secondary mb-1">Content (Markdown or Text)</label>
                <textarea
                  rows={5}
                  placeholder="Paste contents here..."
                  value={ingestContent}
                  onChange={(e) => setIngestContent(e.target.value)}
                  className="w-full rounded-md border border-border-subtle bg-overlay-subtle px-3 py-2 text-xs text-text-primary placeholder:text-text-muted outline-hidden resize-y"
                />
              </div>
              <button
                type="button"
                data-testid="ingest-text-btn"
                disabled={ingestingText || !ingestContent.trim()}
                onClick={() => void handleIngestText()}
                className="inline-flex items-center gap-1.5 rounded-md border border-brass/40 bg-brass/15 px-3 py-1.5 text-xs font-medium text-brass hover:bg-brass/25 disabled:opacity-40 transition"
              >
                {ingestingText ? 'Filing…' : 'File for Review'}
              </button>
            </div>
          </BentoCard>
        </BentoGrid>
        </div>
      )}

      {/* Health Tab */}
      {activeTab === 'health' && (
        <BentoCard className="gap-4 p-4 @2xl/surface:p-5">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border-subtle pb-4">
            <div className="min-w-0">
              <h3 className="font-display text-[14px] font-semibold text-text-primary">Knowledge Graph Health &amp; Topology</h3>
              <p className="mt-0.5 text-xs text-text-muted">
                Runtime database metrics, SQLite FTS5 search index status, and node type breakdown.
              </p>
            </div>
          </div>

          {loadingHealth ? (
            <div className="py-8 text-center text-xs text-text-muted">Inspecting knowledge base health…</div>
          ) : !health ? (
            <div className="py-8 text-center text-xs text-text-muted">Auto-probing knowledge base health…</div>
          ) : (
            <div className="space-y-5">
              <BentoGrid preset="triad">
                <div className="min-w-0 rounded-xl border border-border-subtle bg-overlay-subtle p-4">
                  <div className="text-[11px] uppercase tracking-wider text-text-muted font-medium">Total Nodes</div>
                  <div className="mt-1 text-2xl font-bold font-mono text-text-primary">
                    {health.node_count.toLocaleString()}
                  </div>
                </div>

                <div className="min-w-0 rounded-xl border border-border-subtle bg-overlay-subtle p-4">
                  <div className="text-[11px] uppercase tracking-wider text-text-muted font-medium">Total Edges</div>
                  <div className="mt-1 text-2xl font-bold font-mono text-text-primary">
                    {health.edge_count.toLocaleString()}
                  </div>
                </div>

                <div className="min-w-0 rounded-xl border border-border-subtle bg-overlay-subtle p-4">
                  <div className="text-[11px] uppercase tracking-wider text-text-muted font-medium">FTS5 Search Index</div>
                  <div className="mt-1 flex items-center gap-2">
                    <span className={`size-2.5 shrink-0 rounded-full ${health.fts_available ? 'bg-(--color-status-pass)' : 'bg-(--color-status-fail)'}`} />
                    <span className="font-semibold text-sm text-text-primary">
                      {health.fts_available ? 'Ready & Available' : 'Offline / Degraded'}
                    </span>
                  </div>
                </div>
              </BentoGrid>

              <div>
                <h4 className="text-xs font-semibold text-text-secondary uppercase tracking-wider mb-3">Corpus Distribution by Node Type</h4>
                {Object.keys(health.corpus_counts).length === 0 ? (
                  <p className="text-xs text-text-muted">No classified node types yet.</p>
                ) : (
                  <BentoGrid preset="dense">
                    {Object.entries(health.corpus_counts).map(([type, count]) => (
                      <div key={type} className="min-w-0 rounded-lg border border-border-subtle bg-overlay-subtle p-3">
                        <div className="font-mono text-xs text-brass font-medium truncate">{type}</div>
                        <div className="mt-1 text-lg font-bold text-text-primary font-mono">{count.toLocaleString()}</div>
                      </div>
                    ))}
                  </BentoGrid>
                )}
              </div>
            </div>
          )}
        </BentoCard>
      )}
    </div>
  );
}
