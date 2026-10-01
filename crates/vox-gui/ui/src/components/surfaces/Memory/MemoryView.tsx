import React, { useCallback, useEffect, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useVirtualList } from '../../../hooks/useVirtualList';
import { shardSparkColor } from '../../../lib/visualTokens';
import { invoke } from '@tauri-apps/api/core';
import { voxTransport } from '../../../transport';
import { useLabel } from '../../../hooks/useLanguage';
import { Glass } from '../../ui/Glass';
import { Icon } from '../../ui/Icons';
import { Sparkline } from '../../ui/Sparkline';
import { renderHighlights, UnifiedHit, SearchResponse } from '../Search/searchHelpers';
import { attachItemsFromHits, AttachItem } from '../../../lib/loquelaContext';
import { MEMORY_RECALL_DEBOUNCE_MS } from '../../../config/constants';

export interface KnowledgeNodeRow {
  id: string;
  label: string;
  snippet: string;
  node_type?: string | null;
  created_at: string;
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

function CorpusChip({
  corpus,
  active,
  onToggle,
}: {
  corpus: (typeof MEM_CORPORA)[number];
  active: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-pressed={active}
      aria-label={`Scope: ${corpus.name}`}
      className={`inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 font-mono text-[10px] transition ${
        active
          ? 'border-white/15 bg-overlay-subtle text-text-primary'
          : 'border-border-subtle bg-overlay-subtle text-text-muted'
      }`}
    >
      <span aria-hidden="true" className={`size-1.5 rounded-full ${active ? 'bg-brass' : 'bg-white/15'}`} />
      {corpus.name}
    </button>
  );
}

function HitCard({
  hit,
  query,
  onPin,
  onOpen,
}: {
  hit: UnifiedHit;
  query: string;
  onPin: () => void;
  onOpen: () => void;
}) {
  const kindIcon: Record<string, React.ReactNode> = {
    code:   <Icon.file className="size-3.5" />,
    text:   <Icon.catalog className="size-3.5" />,
    chat:   <Icon.bolt className="size-3.5" />,
    policy: <Icon.shield className="size-3.5" />,
    web:    <Icon.link className="size-3.5" />,
  };

  const isOpenable = hit.locator.kind === 'file' || hit.locator.kind === 'web';
  // Bridge: use path as the src display; score as relevance; snippet as text.
  const src = hit.path ?? hit.source;

  const segments = renderHighlights(hit.snippet, query);

  return (
    <div
      className={`group flex items-start gap-3 rounded-md border border-border-subtle bg-overlay-subtle p-3 hover:border-white/15 transition ${isOpenable ? 'cursor-pointer' : ''}`}
      onClick={isOpenable ? onOpen : undefined}
    >
      <div className="flex size-7 shrink-0 items-center justify-center rounded-sm bg-overlay-subtle text-text-muted">
        {kindIcon[hit.kind] ?? <Icon.file className="size-3.5" />}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-mono text-[11px] text-text-secondary truncate">{src}</span>
          {/* Show score instead of line number (no line field in UnifiedHit). */}
          <span className="font-mono text-[9px] text-text-muted">
            {(hit.score * 100).toFixed(1)}%
          </span>
        </div>
        <div className="mt-1 text-[12px] leading-relaxed text-text-secondary line-clamp-2">
          {segments.map((seg, i) =>
            seg.mark ? (
              <mark key={i} className="bg-brass/20 text-brass rounded-sm px-0.5">{seg.text}</mark>
            ) : (
              <span key={i}>{seg.text}</span>
            )
          )}
        </div>
      </div>
      <div className="flex flex-col items-end gap-1">
        <div className="h-1 w-16 overflow-hidden rounded-full bg-overlay-subtle">
          <div
            className="h-full bg-linear-to-r from-violet-400 to-emerald-400"
            style={{ width: `${hit.score * 100}%` }}
          />
        </div>
        <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition">
          {isOpenable && (
            <button
              type="button"
              onClick={e => { e.stopPropagation(); onOpen(); }}
              className="rounded-sm border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[9px] text-text-secondary hover:bg-overlay-subtle"
            >
              <Icon.link aria-hidden="true" className="size-2.5 inline mr-0.5" />open
            </button>
          )}
          <button
            type="button"
            onClick={e => { e.stopPropagation(); onPin(); }}
            className="rounded-sm border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[9px] text-text-secondary hover:bg-overlay-subtle"
          >
            <Icon.pin aria-hidden="true" className="size-2.5 inline mr-0.5" />pin
          </button>
        </div>
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
  pushToast: (t: any) => void;
  /**
   * Pin recall hits into the shared Loquela context set. When absent (e.g. a
   * standalone render), pinning is unavailable and the controls report that
   * honestly rather than silently no-op'ing.
   */
  onAttachContext?: (items: AttachItem[]) => void;
}

export function MemoryView({ pushToast, onAttachContext }: MemoryViewProps) {
  const [memStatus, setMemStatus] = useState<MemoryStatusPayload | null>(null);
  const [query, setQuery] = useState('');
  // Scope uses the corpus vocabulary aligned with vox_search_query.
  // Default: all three memory/knowledge/chunk corpora active.
  const [scope, setScope] = useState<MemCorpusId[]>(['memory', 'knowledge', 'chunk']);
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

  // Ingest state
  const [ingestUrl, setIngestUrl] = useState('');
  const [ingestingUrl, setIngestingUrl] = useState(false);
  const [ingestTitle, setIngestTitle] = useState('');
  const [ingestContent, setIngestContent] = useState('');
  const [ingestSourceUrl, setIngestSourceUrl] = useState('');
  const [ingestingText, setIngestingText] = useState(false);

  // Health state
  const [health, setHealth] = useState<KbHealthDto | null>(null);
  const [loadingHealth, setLoadingHealth] = useState(false);

  const fetchNodes = useCallback(async (selectedType: string, selectedPage: number) => {
    setLoadingNodes(true);
    try {
      const typeArg = selectedType === 'all' ? null : selectedType;
      const res = await invoke<KnowledgeNodeRow[]>('list_knowledge_nodes', {
        nodeType: typeArg,
        page: selectedPage,
        limit: 50,
      });
      setNodes(res);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Failed to load nodes', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoadingNodes(false);
    }
  }, [pushToast]);

  const deleteNode = async (id: string) => {
    try {
      await invoke('delete_knowledge_node', { id });
      pushToast({ tone: 'ok', title: 'Node deleted', body: `Node ${id} deleted`, cause: 'backend-ok' });
      void fetchNodes(nodeTypeFilter, page);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Delete failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const fetchHealth = useCallback(async () => {
    setLoadingHealth(true);
    try {
      const res = await invoke<KbHealthDto>('get_kb_health');
      setHealth(res);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Failed to fetch KB health', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoadingHealth(false);
    }
  }, [pushToast]);

  const handleIngestUrl = async () => {
    if (!ingestUrl.trim()) return;
    setIngestingUrl(true);
    try {
      const nodeId = await invoke<string>('ingest_url', { url: ingestUrl.trim() });
      pushToast({ tone: 'ok', title: 'URL Ingested', body: `Saved as ${nodeId}`, cause: 'backend-ok' });
      setIngestUrl('');
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
      const nodeId = await invoke<string>('ingest_text', {
        title: ingestTitle.trim(),
        content: ingestContent.trim(),
        sourceUrl: ingestSourceUrl.trim() || null,
      });
      pushToast({ tone: 'ok', title: 'Document Stored', body: `Saved as ${nodeId}`, cause: 'backend-ok' });
      setIngestTitle('');
      setIngestContent('');
      setIngestSourceUrl('');
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
    invoke<MemoryStatusPayload>('get_memory_status')
      .then(setMemStatus)
      .catch((err) => pushToast({ tone: 'warn', title: 'Memory status unavailable', body: sanitizeErrorForToast(err) }));
  }, [pushToast]);

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
        .catch((err) => pushToast({ tone: 'warn', title: 'Could not persist auto-recall', body: sanitizeErrorForToast(err) }));
      return next;
    });
  };

  const corpora = MEM_CORPORA.map((c) => ({
    ...c,
    entries: memStatus?.corpus_counts?.[c.id] ?? 0,
  }));
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
      const res = await invoke<SearchResponse>('vox_search_query', {
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
      });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Recall backend error', body: sanitizeErrorForToast(err) });
    } finally {
      setRecalling(false);
      invoke<MemoryStatusPayload>('get_memory_status').then(setMemStatus).catch(() => {});
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
      });
      return;
    }
    if (!onAttachContext) {
      pushToast({
        tone: 'warn',
        title: 'Context unavailable',
        body: 'Pin-to-context is only available inside the main shell.',
      });
      return;
    }
    onAttachContext(items);
  };

  const openHit = async (hit: UnifiedHit) => {
    if (hit.locator.kind === 'file' || hit.locator.kind === 'web') {
      try {
        await voxTransport.openLocator(hit.locator);
      } catch (err) {
        pushToast({ tone: 'warn', title: 'Could not open', body: sanitizeErrorForToast(err) });
      }
    }
  };

  return (
    <div className="grid grid-cols-12 gap-5">
      {/* Header + Search */}
      <Glass className="col-span-12 p-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="font-display text-[18px] font-semibold tracking-tight text-text-primary">
              Mnemosyne · Memory
            </h2>
            <p className="mt-0.5 text-[11px] text-text-muted">
              Vector + symbolic recall · {scope.length} corpora active · {(totalEntries ?? 0).toLocaleString()} indexed entries
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={toggleAutoRecall}
              aria-pressed={recallOn}
              title={recallOn ? 'Auto-recall on — queries recall as you type' : 'Auto-recall off — press Enter or Recall'}
              className={`inline-flex items-center gap-1.5 rounded-md border px-2 py-1.5 font-mono text-[10px] transition ${
                recallOn
                  ? 'border-violet-400/40 bg-violet-400/10 text-violet-300'
                  : 'border-border-subtle bg-overlay-subtle text-text-muted hover:text-text-secondary'
              }`}
            >
              <Icon.eye aria-hidden="true" className="size-3" /> Auto-recall
            </button>
            <button
              type="button"
              onClick={() =>
                invoke('mnemosyne_reindex')
                  .then(() => pushToast({ tone: 'ok', title: 'Reindex complete', cmd: 'mnemosyne reindex' }))
                  .catch((err) => pushToast({ tone: 'warn', title: 'Reindex failed', body: sanitizeErrorForToast(err) }))
              }
              className="inline-flex items-center gap-1.5 rounded-md border border-border-subtle bg-overlay-subtle px-2 py-1.5 font-mono text-[10px] text-text-muted hover:text-text-secondary"
            >
              <Icon.refresh aria-hidden="true" className="size-3" /> Reindex
            </button>
          </div>
        </div>

        {/* Navigation Tabs */}
        <div className="mt-4 flex items-center gap-1 border-b border-border-subtle pb-2">
          {(['search', 'browse', 'ingest', 'health'] as const).map((tab) => (
            <button
              key={tab}
              type="button"
              data-testid={`tab-${tab}`}
              onClick={() => setActiveTab(tab)}
              className={`rounded-md px-3 py-1 text-xs font-medium capitalize transition ${
                activeTab === tab
                  ? 'bg-overlay-hover text-text-primary font-semibold shadow-xs'
                  : 'text-text-muted hover:text-text-secondary'
              }`}
            >
              {tab === 'search' ? 'Search & Recall' : tab === 'browse' ? 'Browse Nodes' : tab === 'ingest' ? 'Ingest Content' : 'KB Health'}
            </button>
          ))}
        </div>

        {activeTab === 'search' && (
          <>
            {/* Search bar */}
            <div className="mt-4 flex items-center gap-2 rounded-xl border border-border-subtle bg-overlay-subtle px-3 py-2">
              <Icon.search aria-hidden="true" className="size-3.5 text-text-muted" />
              <input
                value={query}
                onChange={e => setQuery(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && recall()}
                aria-label="Recall query"
                placeholder="Recall… e.g. 'ed25519 invariants', 'checkpoint stall'"
                className="flex-1 bg-transparent text-[13px] text-text-primary placeholder:text-text-muted outline-hidden"
              />
              <span className="font-mono text-[10px] text-text-muted">top</span>
              <input
                type="number" min={1} max={50} value={topK}
                onChange={e => setTopK(parseInt(e.target.value) || 8)}
                aria-label="Number of top hits"
                className="w-12 rounded-sm border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 text-center font-mono text-[11px] text-text-secondary outline-hidden"
              />
              <button
                type="button"
                onClick={() => recall()}
                disabled={!query.trim() || recalling}
                className={`inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 font-display text-[10px] uppercase tracking-widest transition ${
                  query.trim()
                    ? 'border-brass/40 bg-brass/15 text-brass hover:bg-brass/25'
                    : 'border-border-subtle bg-overlay-subtle text-text-muted cursor-not-allowed'
                }`}
              >
                {recalling ? '…' : 'Recall'}
              </button>
            </div>

            {/* Scope chips — corpus vocabulary: memory / knowledge / chunk */}
            <div className="mt-3 flex flex-wrap items-center gap-1.5">
              <span className="font-display text-[9px] uppercase tracking-[0.22em] text-text-muted">Scope</span>
              {corpora.map(c => (
                <CorpusChip
                  key={c.id}
                  corpus={c}
                  active={scope.includes(c.id)}
                  onToggle={() => toggleScope(c.id)}
                />
              ))}
            </div>
          </>
        )}
      </Glass>

      {/* Search Tab Content */}
      {activeTab === 'search' && (
        <>
          {/* Recent recalls */}
      <Glass className="col-span-12 xl:col-span-4 p-5">
        <div className="flex items-center justify-between">
          <h3 className="font-display text-[13px] uppercase tracking-[0.18em] text-text-secondary">Recent recalls</h3>
          <Icon.clock className="size-3.5 text-text-muted" />
        </div>
        <div
          ref={recallsRef}
          style={{
            height: Math.min(Math.max(recentRecalls.length, 1) * (RECALL_ITEM_HEIGHT + RECALL_GAP), 312),
            overflow: 'auto',
            marginTop: '0.75rem',
          }}
          className="custom-scrollbar"
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
                    className="flex w-full items-center justify-between rounded-md border border-border-subtle bg-overlay-subtle px-2.5 py-1.5 text-left hover:border-white/15 hover:bg-overlay-subtle transition"
                  >
                    <div className="min-w-0">
                      <div className="truncate text-[12px] text-text-secondary">{r.q}</div>
                      <div className="font-mono text-[9px] text-text-muted">{r.n} hits · {r.when} ago</div>
                    </div>
                    <Icon.chevR className="size-3 text-text-muted shrink-0" />
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      </Glass>

      {/* Hits */}
      <Glass className="col-span-12 xl:col-span-8 p-5">
        <div className="flex items-center justify-between">
          <h3 className="font-display text-[13px] uppercase tracking-[0.18em] text-text-secondary">
            Citations {hits.length > 0 && <span className="text-text-muted">· {hits.length}</span>}
          </h3>
          {hits.length > 0 && (
            <button
              type="button"
              onClick={() => pinHits(hits)}
              className="inline-flex items-center gap-1 rounded-md border border-cyan-400/30 bg-cyan-400/10 px-2 py-1 font-mono text-[10px] text-cyan-300 hover:bg-cyan-400/15"
            >
              <Icon.pin aria-hidden="true" className="size-3" /> Pin all to context
            </button>
          )}
        </div>
        <div className="mt-3 space-y-2" aria-label="Recall citations" aria-live="polite" aria-busy={recalling}>
          {recalling &&
            Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="h-12 rounded-md border border-border-subtle bg-overlay-subtle relative overflow-hidden">
                <span className="absolute inset-0 -translate-x-full animate-vox-shimmer bg-linear-to-r from-transparent via-white/5 to-transparent" />
              </div>
            ))}
          {!recalling && hits.length === 0 && (
            <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-border-subtle bg-overlay-subtle py-12 text-center">
              <Icon.memory className="size-6 text-text-muted mb-2" />
              <div className="font-display text-[12px] tracking-wider text-text-muted">No recall yet</div>
              <div className="font-mono text-[10px] text-text-muted">Type a query or click a recent recall</div>
            </div>
          )}
          {!recalling &&
            hits.map((h, i) => (
              <HitCard
                key={i}
                hit={h}
                query={query}
                onOpen={() => openHit(h)}
                onPin={() => pinHits([h])}
              />
            ))}
        </div>
      </Glass>

      {/* Memory shards */}
      <Glass className="col-span-12 p-5">
        <div className="flex items-center justify-between">
          <h3 className="font-display text-[13px] uppercase tracking-[0.18em] text-text-secondary">{useLabel('mem-shards')}</h3>
          <span className="font-mono text-[10px] text-text-muted">
            {(memStatus?.shards ?? []).length} live · HNSW
            {memStatus?.embedding_dim != null ? ` · dim ${memStatus.embedding_dim}` : ''}
          </span>
        </div>
        <div
          ref={shardsRef}
          className="mt-3 max-h-[700px] overflow-y-auto custom-scrollbar"
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
                  <div className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-6">
                    {rowShards.map(s => (
                      <div
                        key={s.id}
                        className={`rounded-xl border p-3 transition hover:border-white/15 ${
                          s.hot   ? 'border-brass/30 bg-brass/4' :
                          s.dirty ? 'border-amber-400/30 bg-amber-400/4' :
                                    'border-border-subtle bg-overlay-subtle'
                        }`}
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-mono text-[11px] text-text-secondary">shard-{s.id}</span>
                          {s.hot   && <span className="rounded-full bg-brass/15 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-widest text-brass">hot</span>}
                          {s.dirty && <span className="rounded-full bg-amber-400/15 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-widest text-amber-300">dirty</span>}
                        </div>
                        <div className="mt-2 grid grid-cols-2 gap-1.5 text-[9px]">
                          <div className="rounded-sm border border-border-subtle bg-bg-base/40 px-2 py-1.5">
                            <div className="uppercase tracking-widest text-text-muted">Depth</div>
                            <div className="mt-0.5 font-mono text-[11px] text-text-secondary">{s.depth}</div>
                          </div>
                          <div className="rounded-sm border border-border-subtle bg-bg-base/40 px-2 py-1.5">
                            <div className="uppercase tracking-widest text-text-muted">Entries</div>
                            <div className="mt-0.5 font-mono text-[11px] text-text-secondary">{(s.entries ?? 0).toLocaleString()}</div>
                          </div>
                        </div>
                        <div className="mt-2 h-8">
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
        </div>
      </Glass>
        </>
      )}

      {/* Browse Tab */}
      {activeTab === 'browse' && (
        <Glass className="col-span-12 p-5">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border-subtle pb-4">
            <div className="flex items-center gap-3">
              <h3 className="font-display text-[14px] font-semibold text-text-primary">Knowledge Graph Nodes</h3>
              <select
                aria-label="Filter by node type"
                value={nodeTypeFilter}
                onChange={(e) => {
                  setNodeTypeFilter(e.target.value);
                  setPage(0);
                }}
                className="rounded-md border border-border-subtle bg-overlay-subtle px-2.5 py-1 text-xs text-text-secondary outline-hidden"
              >
                <option value="all">All Node Types</option>
                <option value="research_synthesis">Research Synthesis</option>
                <option value="web_research_source">Web Source</option>
                <option value="document">Document</option>
                <option value="web_scrape">Web Scrape</option>
              </select>
            </div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => void fetchNodes(nodeTypeFilter, page)}
                disabled={loadingNodes}
                className="inline-flex items-center gap-1.5 rounded-md border border-border-subtle bg-overlay-subtle px-2.5 py-1 text-xs text-text-secondary hover:text-text-primary transition"
              >
                <Icon.refresh className={`size-3 ${loadingNodes ? 'animate-spin' : ''}`} />
                Refresh
              </button>
            </div>
          </div>

          <div className="mt-4 overflow-x-auto">
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
                        <div className="font-semibold text-text-primary text-[12px]">{n.label}</div>
                        <div className="text-[10px] text-text-muted truncate max-w-xs">{n.id}</div>
                      </td>
                      <td className="py-2.5 pr-3">
                        <span className="rounded-full bg-brass/10 px-2 py-0.5 text-[10px] text-brass">
                          {n.node_type || 'untyped'}
                        </span>
                      </td>
                      <td className="py-2.5 pr-3 text-text-secondary line-clamp-2 max-w-md">
                        {n.snippet}
                      </td>
                      <td className="py-2.5 pr-3 text-text-muted whitespace-nowrap text-[11px]">
                        {n.created_at}
                      </td>
                      <td className="py-2.5 text-right whitespace-nowrap">
                        <button
                          type="button"
                          data-testid={`delete-node-${n.id}`}
                          onClick={() => void deleteNode(n.id)}
                          title="Delete knowledge node and associated edges"
                          className="inline-flex items-center gap-1 rounded border border-rose-500/30 bg-rose-500/10 px-2 py-1 text-[11px] text-rose-300 hover:bg-rose-500/20 transition"
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
          </div>

          <div className="mt-4 flex items-center justify-between border-t border-border-subtle pt-3 text-xs text-text-muted">
            <button
              type="button"
              disabled={page === 0}
              onClick={() => setPage((p) => Math.max(0, p - 1))}
              className="rounded border border-border-subtle bg-overlay-subtle px-2.5 py-1 disabled:opacity-40"
            >
              Previous
            </button>
            <span>Page {page + 1}</span>
            <button
              type="button"
              disabled={nodes.length < 50}
              onClick={() => setPage((p) => p + 1)}
              className="rounded border border-border-subtle bg-overlay-subtle px-2.5 py-1 disabled:opacity-40"
            >
              Next
            </button>
          </div>
        </Glass>
      )}

      {/* Ingest Tab */}
      {activeTab === 'ingest' && (
        <>
          <Glass className="col-span-12 xl:col-span-6 p-5">
            <h3 className="font-display text-[14px] font-semibold text-text-primary flex items-center gap-2">
              <Icon.globe className="size-4 text-brass" />
              Scrape & Ingest URL
            </h3>
            <p className="mt-1 text-xs text-text-muted">
              Extract and clean webpage markdown using the native Vox search scraper (respecting robots.txt) and store as a knowledge node.
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
          </Glass>

          <Glass className="col-span-12 xl:col-span-6 p-5">
            <h3 className="font-display text-[14px] font-semibold text-text-primary flex items-center gap-2">
              <Icon.edit className="size-4 text-brass" />
              Direct Text / Markdown Ingest
            </h3>
            <p className="mt-1 text-xs text-text-muted">
              Store notes, reference documents, or manual excerpts into the knowledge graph with full FTS5 indexing.
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
                {ingestingText ? 'Storing Document…' : 'Store Document'}
              </button>
            </div>
          </Glass>
        </>
      )}

      {/* Health Tab */}
      {activeTab === 'health' && (
        <Glass className="col-span-12 p-5">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border-subtle pb-4">
            <div>
              <h3 className="font-display text-[14px] font-semibold text-text-primary">Knowledge Graph Health & Topology</h3>
              <p className="mt-0.5 text-xs text-text-muted">
                Runtime database metrics, SQLite FTS5 search index status, and node type breakdown.
              </p>
            </div>
            <button
              type="button"
              onClick={() => void fetchHealth()}
              disabled={loadingHealth}
              className="inline-flex items-center gap-1.5 rounded-md border border-border-subtle bg-overlay-subtle px-2.5 py-1 text-xs text-text-secondary hover:text-text-primary transition"
            >
              <Icon.refresh className={`size-3 ${loadingHealth ? 'animate-spin' : ''}`} />
              Refresh Health
            </button>
          </div>

          {loadingHealth ? (
            <div className="py-8 text-center text-xs text-text-muted">Inspecting knowledge base health…</div>
          ) : !health ? (
            <div className="py-8 text-center text-xs text-text-muted">No health data available. Click Refresh to probe.</div>
          ) : (
            <div className="mt-5 space-y-6">
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                <div className="rounded-xl border border-border-subtle bg-overlay-subtle p-4">
                  <div className="text-[11px] uppercase tracking-wider text-text-muted font-medium">Total Nodes</div>
                  <div className="mt-1 text-2xl font-bold font-mono text-text-primary">
                    {health.node_count.toLocaleString()}
                  </div>
                </div>

                <div className="rounded-xl border border-border-subtle bg-overlay-subtle p-4">
                  <div className="text-[11px] uppercase tracking-wider text-text-muted font-medium">Total Edges</div>
                  <div className="mt-1 text-2xl font-bold font-mono text-text-primary">
                    {health.edge_count.toLocaleString()}
                  </div>
                </div>

                <div className="rounded-xl border border-border-subtle bg-overlay-subtle p-4">
                  <div className="text-[11px] uppercase tracking-wider text-text-muted font-medium">FTS5 Search Index</div>
                  <div className="mt-1 flex items-center gap-2">
                    <span className={`size-2.5 rounded-full ${health.fts_available ? 'bg-emerald-400' : 'bg-rose-400'}`} />
                    <span className="font-semibold text-sm text-text-primary">
                      {health.fts_available ? 'Ready & Available' : 'Offline / Degraded'}
                    </span>
                  </div>
                </div>
              </div>

              <div>
                <h4 className="text-xs font-semibold text-text-secondary uppercase tracking-wider mb-3">Corpus Distribution by Node Type</h4>
                {Object.keys(health.corpus_counts).length === 0 ? (
                  <p className="text-xs text-text-muted">No classified node types yet.</p>
                ) : (
                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                    {Object.entries(health.corpus_counts).map(([type, count]) => (
                      <div key={type} className="rounded-lg border border-border-subtle bg-overlay-subtle p-3">
                        <div className="font-mono text-xs text-brass font-medium truncate">{type}</div>
                        <div className="mt-1 text-lg font-bold text-text-primary font-mono">{count.toLocaleString()}</div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}
        </Glass>
      )}
    </div>
  );
}
