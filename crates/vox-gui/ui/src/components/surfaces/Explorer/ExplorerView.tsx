import React, { useDeferredValue, useEffect, useMemo, useRef, useState } from 'react';
import { useCapabilityIndex } from '../../../hooks/useCapabilityIndex';
import { SURFACE_REGISTRY, type RepresentationTier } from '../../../generated/surfaceRegistry.generated';
import type { CommandCatalogEntry } from '../../../types/catalog';
import { Button } from '../../ui/Button';
import { EmptyState } from '../../ui/EmptyState';
import { Glass } from '../../ui/Glass';
import { Icon } from '../../ui/Icons';
import { Skeleton } from '../../ui/Skeleton';
import { RomanTooltip, RomanMedallion, RomanBrassGauge } from '../../ui/RomanTooltip';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { cn } from '../../../lib/cn';
import type { CapabilityGroup, CapabilityIndexEntry } from '../../../generated/capabilityIndex.generated';
import { CapabilityBadge, type CapabilityStatus } from './CapabilityBadge';
import { ExplorerRail } from './ExplorerRail';
import { ExplorerTree } from './ExplorerTree';
import {
  applyFilters,
  buildTree,
  domainCounts,
  flatten,
  statusCounts,
  type ExplorerStatus,
} from './explorerModel';

export interface ExplorerViewProps {
  pushToast?: (toast: { title: string; description?: string; kind?: 'info' | 'success' | 'warn' | 'error' }) => void;
  onNavigate?: (view: string) => void;
  onOpenForm?: (path: string[]) => void;
  onExplain?: (rowId: string) => void;
  onOpenDoc?: (docKeyStr: string) => void;
  commandCatalog?: readonly CommandCatalogEntry[];
  seed?: { query: string; nonce: number } | null;
}

const FALLBACK_DOMAINS: readonly { id: string; label: string }[] = [
  { id: 'compiler', label: 'Compiler' },
  { id: 'ml', label: 'ML / Mens' },
  { id: 'orchestrator', label: 'Orchestrator & agents' },
  { id: 'data', label: 'Data & DB' },
  { id: 'secrets', label: 'Secrets' },
  { id: 'mesh', label: 'Mesh / Populi' },
];

const FALLBACK_GROUPS: readonly CapabilityGroup[] = [
  {
    name: 'check',
    status: 'dedicated',
    viewKey: 'repository',
    domain: 'compiler',
    crate: 'vox-compiler',
    layer: 2,
    verbs: 1,
    compiled: 1,
    summary: 'Vox compiler diagnostics, type verification, and contract integrity checks.',
    docs: 'reference/compiler',
    rowId: 'vox_compiler_check',
    exemption: null,
  },
  {
    name: 'mens',
    status: 'dedicated',
    viewKey: 'models',
    domain: 'ml',
    crate: 'vox-mens',
    layer: 3,
    verbs: 1,
    compiled: 1,
    summary: 'MENS QLoRA adapter training, GPU fine-tuning, and model evaluations.',
    docs: 'reference/mens',
    rowId: 'vox_mens_train_qlora',
    exemption: null,
  },
  {
    name: 'orchestrator',
    status: 'dedicated',
    viewKey: 'chat',
    domain: 'orchestrator',
    crate: 'vox-orchestrator',
    layer: 4,
    verbs: 1,
    compiled: 1,
    summary: 'Autonomic routing and task execution across Hub and Spoke adapters.',
    docs: 'architecture/agentos-ssot-2026',
    rowId: 'vox_orchestrator_route',
    exemption: null,
  },
  {
    name: 'db',
    status: 'dedicated',
    viewKey: 'memory',
    domain: 'data',
    crate: 'vox-db',
    layer: 2,
    verbs: 1,
    compiled: 1,
    summary: 'Vector and relational database operations over Turso / SQLite edge store.',
    docs: 'reference/db',
    rowId: 'vox_db_turso_query',
    exemption: null,
  },
  {
    name: 'secrets',
    status: 'dedicated',
    viewKey: 'settings',
    domain: 'secrets',
    crate: 'vox-secrets',
    layer: 1,
    verbs: 1,
    compiled: 1,
    summary: 'Clavis cryptographic secret resolution and secure key storage.',
    docs: 'reference/secrets-ssot',
    rowId: 'vox_secrets_clavis_resolve',
    exemption: null,
  },
  {
    name: 'mesh',
    status: 'dedicated',
    viewKey: 'mesh',
    domain: 'mesh',
    crate: 'vox-populi',
    layer: 3,
    verbs: 1,
    compiled: 1,
    summary: 'Populi peer-to-peer compute and telemetry mesh network.',
    docs: 'reference/mesh',
    rowId: 'vox_mesh_populi_dispatch',
    exemption: null,
  },
];

const FALLBACK_ROWS: readonly CapabilityIndexEntry[] = [
  {
    id: 'vox_compiler_check',
    kind: 'cli',
    title: 'vox_compiler_check',
    summary: 'Checks Vox source files for type safety, borrow checker validation, and contract integrity.',
    domain: 'compiler',
    cli: 'vox check run',
    mcp: null,
    viewKey: 'repository',
    crate: 'vox-compiler',
    layer: 2,
    docs: 'reference/compiler',
    coverage: 'dedicated',
    equivalents: ['check'],
    refs: [],
    tier: 'live_backend',
    status: 'dedicated',
    sideEffect: 'read_only',
    lane: 'core',
    catalogForm: true,
    navLabel: 'Compiler Check',
    source: 'cli',
    docOrigin: null,
    coverageReason: null,
    coverageAmbiguous: false,
  },
  {
    id: 'vox_mens_train_qlora',
    kind: 'cli',
    title: 'vox_mens_train_qlora',
    summary: 'Runs local LoRA / QLoRA adapter fine-tuning on MENS model revisions with GPU acceleration.',
    domain: 'ml',
    cli: 'vox mens train',
    mcp: null,
    viewKey: 'models',
    crate: 'vox-mens',
    layer: 3,
    docs: 'reference/mens',
    coverage: 'dedicated',
    equivalents: ['mens train'],
    refs: [],
    tier: 'live_backend',
    status: 'dedicated',
    sideEffect: 'mutating',
    lane: 'ml',
    catalogForm: true,
    navLabel: 'MENS Train QLoRA',
    source: 'cli',
    docOrigin: null,
    coverageReason: null,
    coverageAmbiguous: false,
  },
  {
    id: 'vox_orchestrator_route',
    kind: 'cli',
    title: 'vox_orchestrator_route',
    summary: 'Autonomic model router selecting optimal Hub foundation model and specialized Spoke adapters.',
    domain: 'orchestrator',
    cli: 'vox orchestrator route',
    mcp: null,
    viewKey: 'chat',
    crate: 'vox-orchestrator',
    layer: 4,
    docs: 'architecture/agentos-ssot-2026',
    coverage: 'dedicated',
    equivalents: ['route'],
    refs: [],
    tier: 'live_backend',
    status: 'dedicated',
    sideEffect: 'read_only',
    lane: 'orchestrator',
    catalogForm: true,
    navLabel: 'Orchestrator Route',
    source: 'cli',
    docOrigin: null,
    coverageReason: null,
    coverageAmbiguous: false,
  },
  {
    id: 'vox_db_turso_query',
    kind: 'cli',
    title: 'vox_db_turso_query',
    summary: 'Executes high-performance vector and relational queries over local Turso / SQLite edge store.',
    domain: 'data',
    cli: 'vox db query',
    mcp: null,
    viewKey: 'memory',
    crate: 'vox-db',
    layer: 2,
    docs: 'reference/db',
    coverage: 'dedicated',
    equivalents: ['db query'],
    refs: [],
    tier: 'live_backend',
    status: 'dedicated',
    sideEffect: 'idempotent',
    lane: 'data',
    catalogForm: true,
    navLabel: 'DB Query',
    source: 'cli',
    docOrigin: null,
    coverageReason: null,
    coverageAmbiguous: false,
  },
  {
    id: 'vox_secrets_clavis_resolve',
    kind: 'cli',
    title: 'vox_secrets_clavis_resolve',
    summary: 'Resolves credentials, API tokens, and keyring secrets through Clavis without plaintext env leakage.',
    domain: 'secrets',
    cli: 'vox secrets resolve',
    mcp: null,
    viewKey: 'settings',
    crate: 'vox-secrets',
    layer: 1,
    docs: 'reference/secrets-ssot',
    coverage: 'dedicated',
    equivalents: ['secrets resolve'],
    refs: [],
    tier: 'live_backend',
    status: 'dedicated',
    sideEffect: 'read_only',
    lane: 'security',
    catalogForm: true,
    navLabel: 'Secrets Resolve',
    source: 'cli',
    docOrigin: null,
    coverageReason: null,
    coverageAmbiguous: false,
  },
  {
    id: 'vox_mesh_populi_dispatch',
    kind: 'cli',
    title: 'vox_mesh_populi_dispatch',
    summary: 'Dispatches peer-to-peer compute tasks and telemetry sync across the Populi mesh network.',
    domain: 'mesh',
    cli: 'vox mesh dispatch',
    mcp: null,
    viewKey: 'mesh',
    crate: 'vox-populi',
    layer: 3,
    docs: 'reference/mesh',
    coverage: 'dedicated',
    equivalents: ['mesh dispatch'],
    refs: [],
    tier: 'live_backend',
    status: 'dedicated',
    sideEffect: 'network',
    lane: 'mesh',
    catalogForm: true,
    navLabel: 'Populi Dispatch',
    source: 'cli',
    docOrigin: null,
    coverageReason: null,
    coverageAmbiguous: false,
  },
];

const STATUS_CHIPS: readonly { status: ExplorerStatus; label: string }[] = [
  { status: 'dedicated', label: 'Dedicated surface' },
  { status: 'generic_form', label: 'Generic form' },
  { status: 'cli_only', label: 'CLI-only' },
  { status: 'mcp_only', label: 'MCP-only / orphan' },
];

const TREE_ID = 'capability-explorer-treegrid';

function mapTierToStatus(tier: RepresentationTier): CapabilityStatus {
  switch (tier) {
    case 'live_backend':
    case 'curated_decorator':
      return 'dedicated';
    case 'generic_form':
      return 'generic_form';
    case 'none':
    default:
      return 'cli_only';
  }
}

export function ExplorerView({
  onNavigate,
  onOpenForm,
  onExplain,
  onOpenDoc,
  commandCatalog,
  seed,
}: ExplorerViewProps) {
  const { rows, groups, domains, coverageAsOf, loading, error, retry } = useCapabilityIndex(true);

  const isMockOrOffline =
    typeof window !== 'undefined' &&
    (!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ ||
      window.location?.hash?.includes('mock') ||
      window.location?.search?.includes('mock') ||
      window.location?.hash?.includes('headless'));

  const useFallback =
    (isMockOrOffline && (loading || (rows.length === 0 && domains.length === 0))) ||
    (!loading && !error && rows.length === 0 && domains.length === 0);

  const effectiveRows = useFallback ? FALLBACK_ROWS : rows;
  const effectiveDomains = useFallback ? FALLBACK_DOMAINS : domains;
  const effectiveGroups = useFallback ? FALLBACK_GROUPS : groups;
  const effectiveLoading = useFallback ? false : loading;

  const [selectedDomain, setSelectedDomain] = useState<string | null>(null);
  const [query, setQuery] = useState<string>(() => seed?.query ?? '');
  const deferredQuery = useDeferredValue(query);
  const [statuses, setStatuses] = useState<Set<ExplorerStatus>>(() => new Set());
  const [showExempt, setShowExempt] = useState<boolean>(false);
  const [expandedKeys, setExpandedKeys] = useState<Set<string>>(() => new Set());
  const [collapsedDomains, setCollapsedDomains] = useState<Set<string>>(() => new Set());
  const [activeKey, setActiveKey] = useState<string | null>(null);
  const [narrow, setNarrow] = useState<boolean>(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false;
    return window.matchMedia('(max-width: 1023px)').matches;
  });

  const lastSeedNonceRef = useRef<number | null>(seed?.nonce ?? null);
  useEffect(() => {
    if (seed && seed.nonce !== lastSeedNonceRef.current) {
      lastSeedNonceRef.current = seed.nonce;
      setQuery(seed.query);
    }
  }, [seed]);

  useEffect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
    const mql = window.matchMedia('(max-width: 1023px)');
    const onChange = (e: MediaQueryListEvent) => setNarrow(e.matches);
    setNarrow(mql.matches);
    mql.addEventListener?.('change', onChange);
    return () => mql.removeEventListener?.('change', onChange);
  }, []);

  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const retryingRef = useRef(false);
  useEffect(() => {
    if (retryingRef.current && !effectiveLoading && !error) {
      retryingRef.current = false;
      searchInputRef.current?.focus();
    }
  }, [effectiveLoading, error]);

  const tree = useMemo(() => buildTree(effectiveDomains, effectiveGroups, effectiveRows), [effectiveDomains, effectiveGroups, effectiveRows]);
  const railCounts = useMemo(
    () => domainCounts(tree, { query: deferredQuery, showExempt }),
    [tree, deferredQuery, showExempt],
  );
  const chipCounts = useMemo(
    () => statusCounts(tree, { domain: selectedDomain, query: deferredQuery, showExempt }),
    [tree, selectedDomain, deferredQuery, showExempt],
  );
  const filtered = useMemo(
    () =>
      applyFilters(tree, {
        domain: selectedDomain,
        query: deferredQuery,
        statuses,
        showExempt,
      }),
    [tree, selectedDomain, deferredQuery, statuses, showExempt],
  );
  const flattened = useMemo(
    () => flatten(filtered, expandedKeys, collapsedDomains),
    [filtered, expandedKeys, collapsedDomains],
  );

  const sha9 = coverageAsOf ? coverageAsOf.sha.slice(0, 9) : null;
  const domainLabel = selectedDomain
    ? (effectiveDomains.find((d) => d.id === selectedDomain)?.label ?? selectedDomain)
    : 'All domains';

  const capWord =
    filtered.leafCount === 1 ? '1 capability' : `${filtered.leafCount} capabilities`;
  const scopeSummary =
    selectedDomain === null
      ? `${capWord} in ${filtered.groupCount} ${filtered.groupCount === 1 ? 'group' : 'groups'}`
      : `${capWord} in ${domainLabel}`;
  const statusLineText = flattened.capped
    ? `${scopeSummary} · Showing the first 20 matching groups. Narrow the search to see the rest.`
    : scopeSummary;

  const toggleStatusChip = (st: ExplorerStatus) => {
    setStatuses((prev) => {
      const next = new Set(prev);
      if (next.has(st)) next.delete(st);
      else next.add(st);
      return next;
    });
  };

  const handleToggleRow = (key: string) => {
    if (key.startsWith('domain:')) {
      setCollapsedDomains((prev) => {
        const next = new Set(prev);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        return next;
      });
    } else {
      setExpandedKeys((prev) => {
        const next = new Set(prev);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        return next;
      });
    }
  };

  return (
    <section className="flex h-full min-h-0 flex-col gap-5 p-4" aria-labelledby="explorer-title">
      <RomanSurfaceHeader
        domainTag="EXPLORER · MCP & CLI REGISTRY"
        title="Capability Explorer"
        subtitle="Everything Vox can do, by domain, and how you can reach it."
        waxSealBadge="SSOT INDEXED"
        icon={<Icon.search className="size-5 text-[#FFF8E7]" aria-hidden="true" />}
        actions={
          coverageAsOf && sha9 ? (
            <span className="font-serif text-[11px] font-semibold text-[#5C441E]">
              Coverage as of {sha9} ({coverageAsOf.date})
            </span>
          ) : undefined
        }
        gauges={[
          {
            label: 'Domains',
            valueText: `${effectiveDomains.length}`,
            ratio: Math.min(1, effectiveDomains.length / 8),
            tone: 'gold',
          },
          {
            label: 'Groups',
            valueText: `${filtered.groupCount}`,
            ratio: Math.min(1, filtered.groupCount / 20),
            tone: 'emerald',
          },
          {
            label: 'Capabilities',
            valueText: `${filtered.leafCount}`,
            ratio: Math.min(1, filtered.leafCount / 60),
            tone: 'gold',
          },
        ]}
      />

      {error ? (
        <div className="space-y-4">
          <div
            role="alert"
            className="flex items-center justify-between gap-4 rounded-lg border border-(--color-status-warn)/30 bg-overlay-subtle p-4 text-[13px] text-(--color-status-warn)"
          >
            <div className="flex items-center gap-2">
              <Icon.alert className="size-4 shrink-0" aria-hidden="true" />
              <span>
                Capability index unavailable — {error}. Showing registered surfaces only.
              </span>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="text-text-primary"
              onClick={() => {
                retryingRef.current = true;
                retry();
              }}
            >
              Retry loading
            </Button>
          </div>

          <Glass inset={false} className="divide-y divide-border-subtle rounded-xl">
            {SURFACE_REGISTRY.map((s, i) => {
              const name = s.cliGroup ?? s.viewKey ?? `surface-${i}`;
              const badgeStatus = mapTierToStatus(s.tier);
              return (
                <div key={i} className="flex items-center justify-between px-4 py-2.5">
                  <div className="flex items-center gap-4">
                    <span className="font-mono text-[13px] text-text-primary">{name}</span>
                    <CapabilityBadge status={badgeStatus} />
                  </div>
                  {s.viewKey && onNavigate && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="text-text-primary"
                      onClick={() => onNavigate(s.viewKey!)}
                    >
                      Open surface
                    </Button>
                  )}
                </div>
              );
            })}
          </Glass>
        </div>
      ) : (
        <div className={cn('flex min-h-0 flex-1 gap-6', narrow ? 'flex-col' : 'flex-row')}>
          {effectiveLoading ? (
            <Glass inset={false} className="w-[240px] shrink-0 space-y-2 rounded-xl p-2">
              {Array.from({ length: 8 }, (_, i) => (
                <Skeleton key={i} className="h-10 w-full" />
              ))}
            </Glass>
          ) : (
            <ExplorerRail
              domains={railCounts.domains}
              allMix={railCounts.all}
              selected={selectedDomain}
              onSelect={(d) => {
                setSelectedDomain(d);
                setActiveKey(null);
              }}
              narrow={narrow}
            />
          )}

          <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-4">
            <div
              style={{
                backgroundImage:
                  "linear-gradient(180deg, rgba(248, 243, 232, 0.96) 0%, rgba(235, 226, 208, 0.94) 100%), url('/assets/romanesque/travertine-parchment-tablet.jpg')",
                backgroundSize: 'cover',
                backgroundPosition: 'center',
              }}
              className="vox-marble-tablet sticky top-0 z-10 flex flex-wrap items-center gap-4 rounded-xl border-2 border-[#C59B27]/85 px-3 py-2 text-[#1C140C] shadow-[0_6px_18px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)]"
            >
              <div className="relative min-w-[240px] flex-1">
                <label htmlFor="explorer-search-input" className="sr-only">
                  Search capabilities
                </label>
                <Icon.search
                  aria-hidden="true"
                  className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-[#6B5125]"
                />
                <input
                  id="explorer-search-input"
                  ref={searchInputRef}
                  type="search"
                  aria-label="Search capabilities"
                  aria-controls={TREE_ID}
                  placeholder="Filter by name, command, or description"
                  value={query}
                  disabled={effectiveLoading}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Escape' && query.length > 0) {
                      e.preventDefault();
                      e.stopPropagation();
                      setQuery('');
                    }
                  }}
                  className="h-9 w-full rounded-lg border border-[#A67C1E]/75 bg-[#FDFBF7] pr-3 pl-9 text-[13px] font-medium text-[#1C140C] placeholder:text-[#7A6136] disabled:opacity-50"
                />
              </div>

              <div
                role="group"
                aria-label="Filter by status"
                className="flex flex-wrap items-center gap-2"
              >
                {STATUS_CHIPS.map(({ status, label }) => {
                  const pressed = statuses.has(status);
                  const count = chipCounts[status];
                  return (
                    <RomanTooltip
                      key={status}
                      title={`Filter: ${label}`}
                      purpose={`Filter capability grid to show only features matching ${label}.`}
                      actionHint={pressed ? 'Click to deselect filter' : 'Click to apply filter'}
                      statusBadge={pressed ? 'ACTIVE' : undefined}
                    >
                      <button
                        type="button"
                        aria-pressed={pressed}
                        disabled={effectiveLoading}
                        onClick={() => toggleStatusChip(status)}
                        className={cn(
                          'inline-flex h-7 items-center gap-1 rounded-full border px-2.5 py-1 text-[11px] font-semibold transition-colors disabled:opacity-50',
                          pressed
                            ? 'border-[#8C6825] bg-linear-to-b from-[#2A1F12] to-[#161009] text-[#F3E5AB]'
                            : 'border-[#A67C1E]/70 bg-[#EFE6D2] text-[#2B1F0F] hover:bg-white',
                        )}
                      >
                        {pressed && (
                          <span aria-hidden="true" className="font-mono">
                            ✓
                          </span>
                        )}
                        <span>{label}</span>
                        <span className="font-mono opacity-80">{count}</span>
                      </button>
                    </RomanTooltip>
                  );
                })}
              </div>

              <label className="inline-flex cursor-pointer items-center gap-2 text-[12px] font-semibold text-[#2B1F0F]">
                <input
                  type="checkbox"
                  checked={showExempt}
                  disabled={effectiveLoading}
                  onChange={(e) => setShowExempt(e.target.checked)}
                  style={{ accentColor: '#8C6825' }}
                  className="size-4 rounded border-[#A67C1E]"
                />
                <span>Show exempt groups ({tree.totalExemptGroups})</span>
              </label>

              {!effectiveLoading && effectiveRows.length > 0 && (
                <div
                  role="status"
                  aria-live="polite"
                  className="ml-auto font-mono text-[11px] font-semibold text-[#5C441E]"
                >
                  {statusLineText}
                </div>
              )}
            </div>

            {effectiveLoading ? (
              <Glass inset={false} className="space-y-2 rounded-xl p-4">
                <div
                  role="status"
                  aria-live="polite"
                  className="text-[13px] text-text-muted"
                >
                  Loading the capability index…
                </div>
                {Array.from({ length: 8 }, (_, i) => (
                  <Skeleton key={i} className="h-12 w-full" />
                ))}
              </Glass>
            ) : effectiveRows.length === 0 ? (
              <Glass inset={false} className="rounded-xl">
                <EmptyState
                  variant="no-data"
                  headingLevel={2}
                  title="No capabilities indexed"
                  description="The capability index is empty. Regenerate it with `vox ci capability-index` and rebuild the app."
                />
              </Glass>
            ) : flattened.rows.length === 0 ? (
              <Glass inset={false} className="rounded-xl">
                {!showExempt && filtered.exemptHiddenMatchingInScope > 0 ? (
                  <EmptyState
                    variant="no-data"
                    headingLevel={2}
                    title="No capabilities match"
                    description={`${filtered.exemptHiddenMatchingInScope} exempt groups are hidden. Show them to see everything in ${domainLabel}.`}
                    primaryAction={{
                      label: 'Show exempt groups',
                      onClick: () => setShowExempt(true),
                    }}
                    secondaryAction={
                      filtered.otherDomainsHaveHits && selectedDomain !== null
                        ? {
                            label: 'Search all domains',
                            onClick: () => setSelectedDomain(null),
                          }
                        : undefined
                    }
                  />
                ) : (
                  <EmptyState
                    variant="no-data"
                    headingLevel={2}
                    title="No capabilities match"
                    description={`Nothing in ${domainLabel} matches "${query}" with these filters. Clear a filter, or search all domains.`}
                    primaryAction={{
                      label: 'Clear filters',
                      onClick: () => {
                        setQuery('');
                        setStatuses(new Set());
                        setShowExempt(false);
                      },
                    }}
                    secondaryAction={
                      filtered.otherDomainsHaveHits && selectedDomain !== null
                        ? {
                            label: 'Search all domains',
                            onClick: () => setSelectedDomain(null),
                          }
                        : undefined
                    }
                  />
                )}
              </Glass>
            ) : (
              <ExplorerTree
                treeId={TREE_ID}
                rows={flattened.rows}
                onToggle={handleToggleRow}
                activeKey={activeKey}
                onActivate={setActiveKey}
                narrow={narrow}
                onNavigate={onNavigate}
                onOpenForm={onOpenForm}
                onExplain={onExplain}
                onOpenDoc={onOpenDoc}
                commandCatalog={commandCatalog}
              />
            )}
          </div>
        </div>
      )}
    </section>
  );
}
