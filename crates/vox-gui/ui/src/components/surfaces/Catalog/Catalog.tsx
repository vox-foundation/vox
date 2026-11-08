import React from 'react';
import { Glass } from '../../ui/Glass';
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
  const catalog = {
    generated_from: 'tauri:get_command_catalog',
    entries,
  };

  const catalogGauges = [
    {
      label: 'Commands',
      valueText: `${entries.length} Total`,
      ratio: Math.min(1, entries.length / 50),
      tone: 'gold' as const,
      purpose: 'Compiled clap CLI subcommands in registry',
    },
    {
      label: 'Gated',
      valueText: `${entries.filter((e) => e.feature_gate).length} Gated`,
      ratio: Math.min(1, entries.filter((e) => e.feature_gate).length / 10),
      tone: 'gold' as const,
      purpose: 'Feature-gated or experimental CLI subcommands',
    },
    {
      label: 'Dispatch',
      valueText: 'Direct IPC',
      ratio: 1.0,
      tone: 'emerald' as const,
      purpose: 'Command dispatch route (direct IPC with confirmation)',
    },
  ];

  return (
    <div className="flex flex-col gap-4 p-5">
      <RomanSurfaceHeader
        domainTag="COMMANDS · CLI REGISTRY"
        title="Command Catalog & Clap Dispatcher"
        subtitle="Compiled CLI catalog with generated execution forms and destructive action gates."
        waxSealBadge="CLI ACTIVE"
        icon={<Icon.catalog className="size-5 text-[#F3E5AB]" />}
        gauges={catalogGauges}
      />
      <Glass
        style={{
          backgroundImage:
            'linear-gradient(90deg, rgba(248,243,233,0.95) 0%, rgba(235,224,205,0.93) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
        className="border-2 border-[#C59B27] px-4 py-2.5 text-[#1C140C] shadow-[0_4px_14px_rgba(0,0,0,0.4),inset_0_1px_0_rgba(255,255,255,0.9)]"
      >
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="font-display text-sm font-bold tracking-wider text-[#1C140C] uppercase">{useLabel('cat-center')}</h2>
            <p className="text-[11px] font-medium text-[#4A3718]">Compiled CLI catalog with generated execution forms</p>
          </div>
        </div>
      </Glass>
      <div className="h-[calc(100vh-290px)] min-h-[560px]">
        <CommandCatalogForm catalog={catalog} initialPath={prefill?.path} nonce={prefill?.nonce} />
      </div>
    </div>
  );
}
