import React from 'react';
import { BentoCard, BentoGrid, MasterDetailWorkbench } from '../../ui/BentoGrid';
import { CommandCatalogForm } from '../../CommandCatalogForm';
import { useLabel } from '../../../hooks/useLanguage';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';
import type { CommandCatalogEntry } from '../../../types/catalog';

interface CatalogProps {
  catalog?: CommandCatalogEntry[];
  /** Palette handoff: select this CLI path (nonce re-applies a repeat of the same path). */
  prefill?: { path: string[]; nonce: number } | null;
}

export function Catalog({ catalog: entries = [], prefill }: CatalogProps) {
  const [catalogFilter, setCatalogFilter] = React.useState<'all' | 'gated' | 'compiled'>('all');

  const gatedCount = entries.filter((e) => Boolean(e.feature_gate)).length;
  const compiledCount = entries.filter((e) => e.compiled_in).length;
  const filteredEntries =
    catalogFilter === 'gated'
      ? entries.filter((e) => Boolean(e.feature_gate))
      : catalogFilter === 'compiled'
        ? entries.filter((e) => e.compiled_in)
        : entries;

  const catalog = {
    generated_from: 'tauri:get_command_catalog',
    entries: filteredEntries,
  };

  const catalogGauges = [
    {
      label: 'Commands',
      valueText: `${entries.length} Total`,
      ratio: entries.length > 0 ? filteredEntries.length / entries.length : 0,
      tone: 'gold' as const,
      purpose: 'Compiled clap CLI subcommands in registry',
      actionHint: 'Click to show all registered CLI subcommands',
      active: catalogFilter === 'all' && entries.length > 0,
      onClick: () => setCatalogFilter('all'),
    },
    {
      label: 'Gated',
      valueText: `${gatedCount} Gated`,
      ratio: entries.length > 0 ? gatedCount / entries.length : 0,
      tone: 'gold' as const,
      purpose: 'Feature-gated or experimental CLI subcommands',
      actionHint:
        catalogFilter === 'gated'
          ? 'Click to clear feature-gate filter'
          : 'Click to filter catalog to feature-gated CLI subcommands',
      active: catalogFilter === 'gated',
      onClick: () => setCatalogFilter((f) => (f === 'gated' ? 'all' : 'gated')),
    },
    {
      label: 'Dispatch',
      valueText: entries.length > 0 ? `${compiledCount}/${entries.length} IPC` : 'Direct IPC',
      ratio: entries.length > 0 ? compiledCount / entries.length : 0,
      tone: 'emerald' as const,
      purpose: 'Command dispatch route (direct IPC with confirmation)',
      actionHint:
        catalogFilter === 'compiled'
          ? 'Click to show all catalog commands'
          : 'Click to filter to compiled-in direct IPC commands',
      active: catalogFilter === 'compiled',
      onClick: () => setCatalogFilter((f) => (f === 'compiled' ? 'all' : 'compiled')),
    },
  ];

  return (
    <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-4 p-4">
      <RomanSurfaceHeader
        domainTag="COMMANDS · CLI REGISTRY"
        title="Command Catalog & Clap Dispatcher"
        subtitle="Compiled CLI catalog with generated execution forms and destructive action gates."
        waxSealBadge="CLI ACTIVE"
        icon={<Icon.catalog className="size-5 text-[#F3E5AB]" />}
        gauges={catalogGauges}
        actions={<span className="sr-only">{useLabel('cat-center')}</span>}
      />

      <BentoGrid preset="triad">
        <BentoCard className="py-2.5">
          <div className="flex items-center justify-between gap-2">
            <span className="font-display text-[10px] tracking-widest text-brass uppercase">
              Active Registry Slice
            </span>
            <span className="font-mono text-[10px] text-text-muted uppercase">{catalogFilter}</span>
          </div>
          <div className="mt-1 font-mono text-xs text-text-primary">
            {filteredEntries.length} visible of {entries.length} subcommands
          </div>
        </BentoCard>
        <BentoCard className="py-2.5">
          <div className="flex items-center justify-between gap-2">
            <span className="font-display text-[10px] tracking-widest text-emerald-300 uppercase">
              IPC Dispatch Coverage
            </span>
            <span className="font-mono text-[10px] text-emerald-300">{compiledCount} direct</span>
          </div>
          <div className="mt-1 font-mono text-xs text-text-secondary">
            {gatedCount} feature-gated · typed Clap schema hydration
          </div>
        </BentoCard>
        <BentoCard className="py-2.5">
          <div className="flex items-center justify-between gap-2">
            <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">
              Execution Gate
            </span>
            <span className="font-mono text-[10px] text-brass">WR-08 Token</span>
          </div>
          <div className="mt-1 font-mono text-xs text-text-secondary">
            Destructive verbs require explicit human consent
          </div>
        </BentoCard>
      </BentoGrid>

      <CommandCatalogForm
        catalog={catalog}
        initialPath={prefill?.path}
        nonce={prefill?.nonce}
        renderWorkbench={({ master, detail, hasActiveDetail }) => (
          <MasterDetailWorkbench
            storageKey="vox.layout.catalog.v4"
            detailTitle="Command Specification"
            detailDescription="Compiled Clap argument schema, synthesized CLI preview, and gated execution runner."
            hasActiveDetail={hasActiveDetail}
            defaultMasterPct={38}
            minMasterPct={28}
            minDetailPct={35}
            className="min-h-[520px]"
            master={master}
            detail={detail}
          />
        )}
      />
    </div>
  );
}
