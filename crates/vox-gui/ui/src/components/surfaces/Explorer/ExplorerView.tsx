import React, { useDeferredValue, useEffect, useMemo, useRef, useState } from 'react';
import { useCapabilityIndex } from '../../../hooks/useCapabilityIndex';
import { SURFACE_REGISTRY, type RepresentationTier } from '../../../generated/surfaceRegistry.generated';
import type { CommandCatalogEntry } from '../../../types/catalog';
import { Button } from '../../ui/Button';
import { EmptyState } from '../../ui/EmptyState';
import { Glass } from '../../ui/Glass';
import { Icon } from '../../ui/Icons';
import { Skeleton } from '../../ui/Skeleton';
import { cn } from '../../../lib/cn';
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
    if (retryingRef.current && !loading && !error) {
      retryingRef.current = false;
      searchInputRef.current?.focus();
    }
  }, [loading, error]);

  const tree = useMemo(() => buildTree(domains, groups, rows), [domains, groups, rows]);
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
    ? (domains.find((d) => d.id === selectedDomain)?.label ?? selectedDomain)
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
    <section className="flex h-full min-h-0 flex-col gap-6 p-6" aria-labelledby="explorer-title">
      <div className="space-y-1">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h1
            id="explorer-title"
            className="font-display text-[18px] uppercase tracking-wider text-text-primary"
          >
            Capability Explorer
          </h1>
          {coverageAsOf && sha9 && (
            <p className="font-mono text-[11px] text-text-muted">
              Coverage as of {sha9} ({coverageAsOf.date})
            </p>
          )}
        </div>
        <p className="text-[13px] text-text-secondary">
          Everything Vox can do, by domain, and how you can reach it.
        </p>
      </div>

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
          {loading ? (
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
            <div className="sticky top-0 z-10 flex flex-wrap items-center gap-4 rounded-xl bg-bg-base/90 py-1">
              <div className="relative min-w-[240px] flex-1">
                <label htmlFor="explorer-search-input" className="sr-only">
                  Search capabilities
                </label>
                <Icon.search
                  aria-hidden="true"
                  className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-text-muted"
                />
                <input
                  id="explorer-search-input"
                  ref={searchInputRef}
                  type="search"
                  aria-label="Search capabilities"
                  aria-controls={TREE_ID}
                  placeholder="Filter by name, command, or description"
                  value={query}
                  disabled={loading}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Escape' && query.length > 0) {
                      e.preventDefault();
                      e.stopPropagation();
                      setQuery('');
                    }
                  }}
                  className="h-9 w-full rounded-lg border border-border-subtle bg-bg-surface pr-3 pl-9 text-[13px] text-text-primary placeholder:text-text-muted disabled:opacity-50"
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
                    <button
                      key={status}
                      type="button"
                      aria-pressed={pressed}
                      disabled={loading}
                      onClick={() => toggleStatusChip(status)}
                      className={cn(
                        'inline-flex h-7 items-center gap-1 rounded-full bg-overlay-subtle px-2.5 py-1 text-[11px] font-medium transition-colors disabled:opacity-50',
                        pressed
                          ? 'ring-1 ring-(--color-accent-default) text-(--color-accent-default)'
                          : 'ring-1 ring-border-subtle text-text-secondary hover:bg-overlay-hover',
                      )}
                    >
                      {pressed && (
                        <span aria-hidden="true" className="font-mono">
                          ✓
                        </span>
                      )}
                      <span>{label}</span>
                      <span className="font-mono text-text-muted">{count}</span>
                    </button>
                  );
                })}
              </div>

              <label className="inline-flex cursor-pointer items-center gap-2 text-[13px] text-text-secondary">
                <input
                  type="checkbox"
                  checked={showExempt}
                  disabled={loading}
                  onChange={(e) => setShowExempt(e.target.checked)}
                  style={{ accentColor: 'var(--color-accent-default)' }}
                  className="size-4 rounded border-border-subtle"
                />
                <span>Show exempt groups ({tree.totalExemptGroups})</span>
              </label>

              {!loading && rows.length > 0 && (
                <div
                  role="status"
                  aria-live="polite"
                  className="ml-auto font-mono text-[11px] text-text-muted"
                >
                  {statusLineText}
                </div>
              )}
            </div>

            {loading ? (
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
            ) : rows.length === 0 ? (
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
