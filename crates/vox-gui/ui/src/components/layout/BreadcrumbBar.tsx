import React from 'react';
import {
  breadcrumbsForView,
  type BreadcrumbSegment,
  resolveNavigation,
  CHILD_ORDER_BY_PARENT,
  labelForNavKey,
} from '../../lib/navigation';
import { useCurrentLang } from '../../hooks/useLanguage';
import { recordGamifyGuiEvent } from '../../lib/gamifyGuiEvents';
import { surfaceAvailability, useBackendStatus } from '../../hooks/useBackendStatus';
import { UNAVAILABLE_REASON_LABEL } from './SurfaceUnavailable';
import { SURFACE_REGISTRY } from '../../generated/surfaceRegistry.generated';
import { RomanTooltip } from '../ui/RomanTooltip';

interface Props {
  viewKey: string;
  onNavigate?: (viewKey: string) => void;
  gamifyEnabled?: boolean;
}

const SUBSECTION_PURPOSE: Record<string, string> = {
  dashboard: 'Executive mission control with live agent KPIs, attention budget, and interactive workspace map.',
  'vox-operations-hub': 'Real-time agent workflow execution graph, active turns, and tool call traces.',
  'vox-task-list': 'Prioritized task queue with bi-directional hopper ledger and status filters.',
  'vox-mesh-topology': 'Distributed Populi QUIC compute mesh nodes, free VRAM, and peer latency.',
  'vox-subagent-tree': 'Hierarchical sub-agent delegation tree, roles, and active worker transcripts.',
  'vox-Ludus-view': 'Interactive Workspace Map, engineering quests, and verified cryptographic leaderboard.',
  'vox-harness-river': '3-tier model cascade and laminar verification pipeline from codegen to gated merge.',
  'vox-mens-training': 'MENS QLoRA model size ladder, 8-spoke training matrix, and Hugging Face hub sync.',
  'vox-model-router': 'Model selection policy, clutch routing weights, and live cost/latency scoreboard.',
  'vox-cost-analytics': 'Token burn, prompt cache savings, and budget caps across local and cloud providers.',
  'vox-git-view': 'Repository commit timeline, worktree status, branch comparison, and checkpoints.',
  'vox-scientia-worth': 'Evidence-grounded deep research, knowledge base facets, and citation verification.',
};

function purposeForView(key: string): string {
  if (SUBSECTION_PURPOSE[key]) return SUBSECTION_PURPOSE[key];
  return `Open the ${labelForNavKey(key)} surface to inspect and control its live workspace state.`;
}

export function BreadcrumbBar({ viewKey, onNavigate, gamifyEnabled }: Props) {
  const lang = useCurrentLang();
  const segments = breadcrumbsForView(viewKey, lang);
  const { status } = useBackendStatus();
  const availability = surfaceAvailability(status, viewKey);
  if (segments.length === 0 || viewKey === 'chat') return null;

  const { parent, child } = resolveNavigation(viewKey);
  const siblingChildren = CHILD_ORDER_BY_PARENT[parent];

  const renderSegment = (seg: BreadcrumbSegment, index: number) => {
    const isLast = index === segments.length - 1;
    const content = (
      <span className={isLast ? 'text-[#1C140C] font-bold' : 'text-[#5C441E] font-semibold'}>{seg.label}</span>
    );
    if (!isLast && onNavigate) {
      return (
        <RomanTooltip
          key={seg.key}
          title={seg.label}
          purpose={purposeForView(seg.key)}
          actionHint={`Click to return to ${seg.label}`}
          placement="bottom"
        >
          <button
            type="button"
            onClick={() => {
              void recordGamifyGuiEvent(
                'breadcrumb_navigation',
                { from: viewKey, to: seg.key },
                { enabled: gamifyEnabled },
              );
              onNavigate(seg.key);
            }}
            className="font-display text-[11px] uppercase tracking-[0.14em] hover:text-[#1C140C] transition"
            aria-label={`Navigate to ${seg.label}`}
          >
            {content}
          </button>
        </RomanTooltip>
      );
    }
    return (
      <span
        key={seg.key}
        className="font-display text-[11px] uppercase tracking-[0.14em]"
        aria-current={isLast ? 'page' : undefined}
      >
        {content}
      </span>
    );
  };

  return (
    <nav
      aria-label="Breadcrumb"
      style={{
        backgroundImage:
          'linear-gradient(90deg, rgba(245,239,227,0.94) 0%, rgba(235,225,207,0.90) 50%, rgba(245,239,227,0.94) 100%), url(/assets/romanesque/carrara-marble-pillar-slab.jpg)',
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }}
      className="flex flex-wrap items-center justify-between gap-2 rounded-lg border-2 border-[#C59B27] px-3 py-1.5 shadow-[0_4px_14px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)]"
    >
      <div className="flex items-center gap-2 min-w-0">
        {segments.map((seg, i) => (
          <React.Fragment key={`${i}:${seg.key}`}>
            {i > 0 && <span className="text-[#8C6825] font-bold text-[11px]" aria-hidden="true">›</span>}
            {renderSegment(seg, i)}
          </React.Fragment>
        ))}
        {!availability.available && availability.reason && (
          <span data-testid="breadcrumb-unavailable" className="font-display text-[10px] uppercase tracking-[0.14em] text-[#991B1B] font-semibold">
            · {UNAVAILABLE_REASON_LABEL[availability.reason]}
          </span>
        )}
      </div>

      {siblingChildren && siblingChildren.length > 1 && (
        <div
          role="tablist"
          data-testid="breadcrumb-child-tabs"
          aria-label={`${labelForNavKey(parent)} subsections`}
          className="flex items-center gap-1 border-l border-[#9E7B2F]/50 pl-3 overflow-x-auto no-scrollbar max-w-full"
        >
          {siblingChildren.map(subKey => {
            const isCurrent = child === subKey;
            const subLabel = labelForNavKey(subKey);
            return (
              <RomanTooltip
                key={subKey}
                title={subLabel}
                purpose={purposeForView(subKey)}
                actionHint={isCurrent ? 'Currently active surface' : `Click to switch to ${subLabel}`}
                placement="bottom"
              >
                <button
                  type="button"
                  role="tab"
                  data-cv-subsurface={`breadcrumb-tab-${subKey}`}
                  aria-selected={isCurrent}
                  onClick={() => {
                    if (!isCurrent && onNavigate) {
                      void recordGamifyGuiEvent(
                        'breadcrumb_navigation',
                        { from: viewKey, to: subKey },
                        { enabled: gamifyEnabled },
                      );
                      onNavigate(subKey);
                    }
                  }}
                  className={`rounded-md px-2.5 py-1 font-display text-[10px] uppercase tracking-wider transition whitespace-nowrap ${
                    isCurrent
                      ? 'border border-[#8C6825] bg-linear-to-b from-[#2E2212] to-[#161008] text-[#F3E5AB] font-bold shadow-[0_2px_6px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(243,229,171,0.35)]'
                      : 'border border-[#9E7B2F]/45 bg-[#FFFDF9]/80 text-[#23190E] font-semibold hover:border-[#8C6825] hover:bg-[#231B12] hover:text-[#F3E5AB]'
                  }`}
                >
                  {subLabel}
                </button>
              </RomanTooltip>
            );
          })}
        </div>
      )}
    </nav>
  );
}

