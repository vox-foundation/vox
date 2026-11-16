import React, { useEffect, useState } from 'react';
import { ScientiaDashboard } from './ScientiaDashboard';
import { ClaimsView } from './ClaimsView';
import { HarnessIssuesPanel } from './HarnessIssuesPanel';
import { DiscoveryInbox } from './DiscoveryInbox';
import { DiscoveryReview } from './DiscoveryReview';
import { WorthinessRadarPanel } from './WorthinessRadarPanel';
import { DirectPublicationDrawer } from './DirectPublicationDrawer';
import { BeforeAfterAuditInspector } from './BeforeAfterAuditInspector';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';

export { WorthinessRadarPanel, DirectPublicationDrawer, BeforeAfterAuditInspector };

const TABS = [
  { id: 'dashboard', label: 'Dashboard' },
  { id: 'inbox', label: 'Inbox' },
  { id: 'studio', label: 'Studio & Review' },
  { id: 'claims', label: 'Claims' },
  { id: 'audit', label: 'Audit Diff' },
  { id: 'harness', label: 'Harness Issues' },
] as const;

type ScientiaTab = typeof TABS[number]['id'];

/**
 * Findings surface (view key `scientia`). Unifies the Scientia queue dashboard,
 * continuous discovery inbox, 6-axis worthiness studio & direct publication flywheel,
 * atomic claims ledger, and harness issue proposals.
 */
export function ScientiaSurface(props: SurfaceDecoratorProps) {
  const [tab, setTab] = useState<ScientiaTab>('dashboard');
  const ScientiaIcon = (Icon as any).scientia ?? Icon.beaker;

  useEffect(() => {
    const onOpenStudio = () => {
      setTab('studio');
    };
    window.addEventListener('vox://scientia-open-studio', onOpenStudio);
    return () => window.removeEventListener('vox://scientia-open-studio', onOpenStudio);
  }, []);

  return (
    <div className="flex min-h-0 flex-col gap-3 p-4">
      {tab !== 'dashboard' && (
        <RomanSurfaceHeader
          domainTag="KNOWLEDGE · SCIENTIA"
          title="Scientia Epistemic Discovery & Verification Ledger"
          subtitle="Track machine-verified research claims, novelty evidence, and publication readiness."
          waxSealBadge="EPISTEMIC LEDGER"
          icon={<ScientiaIcon className="size-5 text-[#F3E5AB]" />}
        />
      )}
      <div role="tablist" aria-label="Findings sections" className="flex flex-wrap gap-1">
        {TABS.map(t => (
          <button
            key={t.id}
            type="button"
            role="tab"
            data-testid={`scientia-tab-${t.id}`}
            aria-selected={tab === t.id}
            onClick={() => setTab(t.id)}
            className={`rounded-md px-3 py-1.5 font-display text-[11px] uppercase tracking-[0.16em] transition ${
              tab === t.id
                ? 'border border-[#C59B27]/55 bg-bg-elevated text-text-primary font-semibold shadow-xs'
                : 'border border-transparent text-text-secondary hover:border-[#C59B27]/35 hover:bg-bg-surface hover:text-text-primary'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>
      {tab === 'dashboard' ? (
        <ScientiaDashboard {...props} />
      ) : tab === 'inbox' ? (
        <div className="pt-1">
          <DiscoveryInbox {...props} />
        </div>
      ) : tab === 'studio' ? (
        <div className="pt-1">
          <DiscoveryReview {...props} />
        </div>
      ) : tab === 'claims' ? (
        <ClaimsView {...props} />
      ) : tab === 'audit' ? (
        <div className="pt-1">
          <BeforeAfterAuditInspector />
        </div>
      ) : (
        <HarnessIssuesPanel {...props} />
      )}
    </div>
  );
}
