import React, { useCallback, useEffect, useState } from 'react';
import { invokeCmd } from '../../../transport';
import { BentoCard, BentoGrid, BentoScrollRegion, MasterDetailWorkbench } from '../../ui/BentoGrid';
import { Button } from '../../ui/Button';
import { StatusPill } from '../../ui/StatusPill';
import type { StatusToneKind } from '../../../styles/tokens';
import { EmptyState } from '../../ui/EmptyState';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { Toast } from '../../../types/tauri';
import { GsdDocumentsPane, type GsdDocumentHeader } from './GsdDocumentsPane';

interface GsdMilestone {
  milestone_id: string;
  title: string;
  status: string;
  start_phase: number;
  end_phase: number;
}

interface GsdPhase {
  milestone_id: string;
  phase_number: number;
  title: string;
  status: string;
  wave_count: number;
  plan_session_id?: string | null;
}

interface GsdProgressSnapshot {
  active_milestone?: GsdMilestone | null;
  phases: GsdPhase[];
  requirements: Array<{ req_id: string; phase_number: number; description: string; satisfied: boolean }>;
  documents: GsdDocumentHeader[];
  next_recommended_skill: string;
}

interface GsdStatusEnvelope {
  health?: { healthy: boolean; has_planning_dir?: boolean; missing?: string[] };
  snapshot?: GsdProgressSnapshot;
}

function phaseTone(status: string): StatusToneKind {
  switch (status) {
    case 'completed':
      return 'pass';
    case 'executing':
    case 'verifying':
      return 'Executing';
    case 'planned':
      return 'Planning';
    case 'blocked':
      return 'fail';
    default:
      return 'neutral';
  }
}

export function GsdStudioView({
  pushToast,
  onNavigate,
}: {
  pushToast: (t: Toast) => void;
  onNavigate?: (viewKey: string) => void;
}) {
  const [snapshot, setSnapshot] = useState<GsdProgressSnapshot | null>(null);
  const [healthy, setHealthy] = useState<boolean>(true);
  const [loading, setLoading] = useState<boolean>(true);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [versionInput, setVersionInput] = useState('v7.1');
  const [titleInput, setTitleInput] = useState('');
  const [startPhaseInput, setStartPhaseInput] = useState('413');
  const [phasesCountInput, setPhasesCountInput] = useState('3');
  const [phaseFilter, setPhaseFilter] = useState<'all' | 'completed' | 'pending'>('all');
  const [selectedPhaseNum, setSelectedPhaseNum] = useState<number | null>(null);

  const loadStatus = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invokeCmd<GsdStatusEnvelope & Partial<GsdProgressSnapshot>>('gsd_get_status', {});
      const snap = res?.snapshot ?? (res?.phases ? (res as GsdProgressSnapshot) : null);
      if (snap) setSnapshot(snap);
      if (res?.health !== undefined) setHealthy(Boolean(res.health.healthy));
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'GSD status failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setLoading(false);
    }
  }, [pushToast]);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  const handleNewMilestone = async (e: React.FormEvent) => {
    e.preventDefault();
    const version = versionInput.trim();
    const title = titleInput.trim();
    if (!version || !title) {
      pushToast({ tone: 'warn', title: 'Milestone needs version and title', cause: 'validation' });
      return;
    }
    setBusyAction('new-milestone');
    try {
      const snap = await invokeCmd<GsdProgressSnapshot>('gsd_new_milestone', {
        input: {
          version,
          title,
          start_phase: Number(startPhaseInput) || 1,
          phases: Number(phasesCountInput) || 3,
        },
      });
      if (snap) setSnapshot(snap);
      setTitleInput('');
      pushToast({ tone: 'ok', title: `Milestone ${version} initialized`, body: title, cause: 'backend-ok' });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Failed to create milestone',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusyAction(null);
    }
  };

  const runPhaseCommand = async (
    cmd: 'gsd_plan_phase' | 'gsd_execute_phase' | 'gsd_verify_work',
    phaseNumber: number,
    label: string,
  ) => {
    setBusyAction(`${cmd}-${phaseNumber}`);
    try {
      const args: Record<string, unknown> = {
        phase: phaseNumber,
        milestoneId: snapshot?.active_milestone?.milestone_id ?? null,
        milestone_id: snapshot?.active_milestone?.milestone_id ?? null,
      };
      if (cmd === 'gsd_verify_work') {
        args.passed = true;
        args.evidence = `Verified Phase ${phaseNumber} via GSD Studio`;
      }
      await invokeCmd(cmd, args);
      pushToast({ tone: 'ok', title: `${label} — Phase ${phaseNumber}`, cause: 'backend-ok' });
      await loadStatus();
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: `${label} failed`,
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusyAction(null);
    }
  };

  const handleCompleteMilestone = async () => {
    const version = snapshot?.active_milestone?.milestone_id;
    if (!version) return;
    setBusyAction('complete-milestone');
    try {
      const snap = await invokeCmd<GsdProgressSnapshot>('gsd_complete_milestone', { version });
      if (snap) setSnapshot(snap);
      pushToast({ tone: 'ok', title: `Milestone ${version} completed`, cause: 'backend-ok' });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Complete milestone failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusyAction(null);
    }
  };

  const phases = snapshot?.phases ?? [];
  const requirements = snapshot?.requirements ?? [];
  const documents = snapshot?.documents ?? [];
  const completedCount = phases.filter((p) => p.status === 'completed').length;
  const activeMs = snapshot?.active_milestone;
  const displayedPhases = phases.filter((p) => {
    if (phaseFilter === 'completed') return p.status === 'completed';
    if (phaseFilter === 'pending') return p.status !== 'completed';
    return true;
  });
  const selectedPhase =
    displayedPhases.find((p) => p.phase_number === selectedPhaseNum) ??
    displayedPhases[0] ??
    phases[0] ??
    null;
  const selectedPhaseRequirements = selectedPhase
    ? requirements.filter((r) => r.phase_number === selectedPhase.phase_number)
    : [];

  const masterPane = (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-[13px] font-semibold text-text-primary">Phase Roadmap &amp; Execution Waves</h3>
        <StatusPill tone="info" size="xs" label={`${displayedPhases.length} phases`} />
      </div>
      <BentoScrollRegion ariaLabel="Milestone phase roadmap and wave matrix" className="flex-1 pr-1">
        {displayedPhases.length === 0 ? (
          <EmptyState title="No phases queued" description="Use New Milestone above or type /gsd-new-milestone in chat." />
        ) : (
          <div className="grid grid-cols-1 gap-3 @2xl/surface:grid-cols-2">
            {displayedPhases.map((phase) => {
              const isSelected = selectedPhase?.phase_number === phase.phase_number;
              return (
                <div
                  key={`${phase.milestone_id}-${phase.phase_number}`}
                  data-testid={`gsd-phase-card-${phase.phase_number}`}
                  onClick={() => setSelectedPhaseNum(phase.phase_number)}
                  className={`cursor-pointer flex flex-col gap-2 rounded-lg border p-3 transition-colors ${
                    isSelected ? 'border-brass bg-brass/10' : 'border-border-subtle bg-overlay-subtle/30 hover:border-brass/40'
                  }`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <div className="text-[12px] font-semibold text-text-primary">
                        Phase {phase.phase_number}: {phase.title}
                      </div>
                      <div className="text-[11px] text-text-muted">
                        {phase.wave_count} wave{phase.wave_count === 1 ? '' : 's'}
                        {phase.plan_session_id ? ` · session ${phase.plan_session_id}` : ''}
                      </div>
                    </div>
                    <StatusPill tone={phaseTone(phase.status)} size="xs" label={phase.status} />
                  </div>
                  <div className="flex flex-wrap items-center gap-1.5 pt-1" onClick={(e) => e.stopPropagation()}>
                    <Button size="xs" variant="outline" disabled={busyAction !== null} onClick={() => void runPhaseCommand('gsd_plan_phase', phase.phase_number, 'Planned')}>
                      Plan Phase
                    </Button>
                    <Button size="xs" variant="primary" disabled={busyAction !== null} onClick={() => void runPhaseCommand('gsd_execute_phase', phase.phase_number, 'Executed')}>
                      Execute Phase
                    </Button>
                    <Button size="xs" variant="secondary" disabled={busyAction !== null} onClick={() => void runPhaseCommand('gsd_verify_work', phase.phase_number, 'Verified')}>
                      Verify Work
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </BentoScrollRegion>
    </div>
  );

  const detailPane = (
    <BentoScrollRegion ariaLabel="Selected phase requirements and planning artifacts" data-testid="gsd-phase-inspector" className="flex-1 pr-1">
      <div className="flex flex-col gap-3">
        {selectedPhase && (
          <div className="rounded-xl border border-border-subtle bg-bg-surface p-3.5 flex flex-col gap-2.5">
            <div className="flex items-start justify-between gap-2">
              <div>
                <div className="font-display text-xs tracking-wider text-brass uppercase">{`Selected Phase ${selectedPhase.phase_number}`}</div>
                <div className="text-sm font-semibold text-text-primary">{selectedPhase.title}</div>
                <div className="font-mono text-[11px] text-text-muted">
                  {`Milestone ${selectedPhase.milestone_id} · ${selectedPhase.wave_count} wave(s)`}
                  {selectedPhase.plan_session_id ? ` · ${selectedPhase.plan_session_id}` : ''}
                </div>
              </div>
              <StatusPill tone={phaseTone(selectedPhase.status)} size="xs" label={selectedPhase.status} />
            </div>

            <div className="flex flex-wrap items-center gap-1.5 border-t border-border-subtle pt-2.5">
              <Button size="xs" variant="outline" data-testid="gsd-inspector-plan-btn" disabled={busyAction !== null} onClick={() => void runPhaseCommand('gsd_plan_phase', selectedPhase.phase_number, 'Planned')}>
                Run Phase Plan
              </Button>
              <Button size="xs" variant="primary" data-testid="gsd-inspector-execute-btn" disabled={busyAction !== null} onClick={() => void runPhaseCommand('gsd_execute_phase', selectedPhase.phase_number, 'Executed')}>
                Dispatch Wave Execution
              </Button>
              <Button size="xs" variant="secondary" data-testid="gsd-inspector-verify-btn" disabled={busyAction !== null} onClick={() => void runPhaseCommand('gsd_verify_work', selectedPhase.phase_number, 'Verified')}>
                Record Phase Verification
              </Button>
            </div>

            <div className="border-t border-border-subtle pt-2.5">
              <div className="mb-1.5 flex items-center justify-between">
                <span className="font-display text-[10px] tracking-widest text-text-secondary uppercase">Phase Requirements Contract</span>
                <span className="font-mono text-[10px] text-text-muted">{`${selectedPhaseRequirements.length} linked`}</span>
              </div>
              {selectedPhaseRequirements.length === 0 ? (
                <p className="font-mono text-[11px] text-text-muted">
                  No explicit requirement IDs mapped to Phase {selectedPhase.phase_number} yet.
                </p>
              ) : (
                <ul className="space-y-1.5">
                  {selectedPhaseRequirements.map((req) => (
                    <li key={req.req_id} className="flex items-start justify-between gap-2 rounded-md border border-border-subtle bg-bg-base/70 px-2.5 py-1.5 text-[11px]">
                      <div>
                        <span className="font-mono font-semibold text-brass">{req.req_id}</span>
                        <span className="ml-1.5 text-text-secondary">{req.description}</span>
                      </div>
                      <StatusPill tone={req.satisfied ? 'pass' : 'neutral'} size="xs" label={req.satisfied ? 'satisfied' : 'open'} />
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        )}

        <GsdDocumentsPane
          documents={documents}
          activeMilestoneId={activeMs?.milestone_id ?? 'v7.0'}
          pushToast={pushToast}
          onStored={() => void loadStatus()}
        />
      </div>
    </BentoScrollRegion>
  );

  return (
    <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-4 p-4" data-testid="gsd-studio-view">
      <RomanSurfaceHeader
        domainTag="AGENTS · GSD"
        title="GSD Milestone & Phase Studio"
        subtitle={
          activeMs
            ? `${activeMs.milestone_id} · ${activeMs.title} (Phases ${activeMs.start_phase}–${activeMs.end_phase})`
            : 'No active .planning milestone found'
        }
        gauges={[
          {
            label: 'Phases',
            valueText: `${phases.length}`,
            ratio: phases.length > 0 ? displayedPhases.length / phases.length : 0,
            tone: 'gold',
            purpose: 'Total milestone phases in .planning roadmap',
            actionHint: 'Click to show all milestone phases',
            active: phaseFilter === 'all' && phases.length > 0,
            onClick: () => setPhaseFilter('all'),
          },
          {
            label: 'Verified',
            valueText: `${completedCount}/${phases.length}`,
            ratio: phases.length > 0 ? completedCount / phases.length : 0,
            tone: completedCount === phases.length && phases.length > 0 ? 'emerald' : 'gold',
            purpose: 'Completed and verified roadmap phases',
            actionHint: phaseFilter === 'completed' ? 'Click to show all phases' : 'Click to filter roadmap to completed phases',
            active: phaseFilter === 'completed',
            onClick: () => setPhaseFilter((f) => (f === 'completed' ? 'all' : 'completed')),
          },
          {
            label: 'Active',
            valueText: `${phases.length - completedCount}`,
            ratio: phases.length > 0 ? (phases.length - completedCount) / phases.length : 0,
            tone: 'gold',
            purpose: 'Planned or executing phases awaiting verification',
            actionHint: phaseFilter === 'pending' ? 'Click to show all phases' : 'Click to filter roadmap to pending phases',
            active: phaseFilter === 'pending',
            onClick: () => setPhaseFilter((f) => (f === 'pending' ? 'all' : 'pending')),
          },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <StatusPill tone={healthy ? 'pass' : 'warn'} size="sm" label={healthy ? 'Health OK' : 'Health Drift'} />
            {onNavigate && (
              <Button size="sm" variant="ghost" onClick={() => onNavigate('flow')}>Open Flow DAG</Button>
            )}
            <Button size="sm" variant="outline" onClick={() => void loadStatus()} disabled={loading}>Refresh</Button>
          </div>
        }
      />

      <BentoGrid preset="hero">
        <BentoCard span="hero-main" className="gap-2">
          <div className="flex items-center justify-between gap-2">
            <div>
              <div className="text-[12px] font-semibold text-text-primary">Milestone Progress</div>
              <div className="text-[11px] text-text-muted">
                {completedCount}/{phases.length} phases verified · Next recommended:{' '}
                <code className="text-text-secondary">{snapshot?.next_recommended_skill || '/gsd-plan-phase'}</code>
              </div>
            </div>
            {activeMs && activeMs.status !== 'completed' && (
              <Button size="sm" variant="secondary" disabled={busyAction !== null} onClick={() => void handleCompleteMilestone()}>
                Complete Milestone
              </Button>
            )}
          </div>
          <div className="h-2 w-full rounded-full bg-overlay-subtle overflow-hidden">
            <div
              className="h-full bg-brass transition-[width] duration-200"
              style={{ width: `${phases.length > 0 ? Math.round((completedCount / phases.length) * 100) : 0}%` }}
            />
          </div>
        </BentoCard>

        <BentoCard span="hero-rail" className="gap-2">
          <h3 className="text-[12px] font-semibold text-text-primary">Initialize New Milestone</h3>
          <form onSubmit={(e) => void handleNewMilestone(e)} className="flex flex-col gap-2">
            <div className="flex gap-2">
              <input type="text" aria-label="Milestone version" value={versionInput} onChange={(e) => setVersionInput(e.target.value)} placeholder="v7.1" className="w-24 rounded-md border border-border-subtle bg-bg-base px-2 py-1 text-[12px] text-text-primary" />
              <input type="text" aria-label="Milestone title" value={titleInput} onChange={(e) => setTitleInput(e.target.value)} placeholder="Next milestone goal" className="flex-1 min-w-0 rounded-md border border-border-subtle bg-bg-base px-2 py-1 text-[12px] text-text-primary" />
            </div>
            <div className="flex items-center gap-2">
              <input type="number" aria-label="Start phase" value={startPhaseInput} onChange={(e) => setStartPhaseInput(e.target.value)} className="w-20 rounded-md border border-border-subtle bg-bg-base px-2 py-1 text-[12px] text-text-primary" />
              <input type="number" aria-label="Phase count" value={phasesCountInput} onChange={(e) => setPhasesCountInput(e.target.value)} className="w-20 rounded-md border border-border-subtle bg-bg-base px-2 py-1 text-[12px] text-text-primary" />
              <Button type="submit" size="sm" variant="primary" disabled={busyAction !== null}>New Milestone</Button>
            </div>
          </form>
        </BentoCard>
      </BentoGrid>

      <MasterDetailWorkbench
        storageKey="vox.layout.gsd.v4"
        detailTitle="Phase & Milestone Inspector"
        detailDescription="Selected phase requirements, execution wave triggers, and stored Arca CAS planning artifacts."
        hasActiveDetail={selectedPhaseNum !== null}
        defaultMasterPct={54}
        master={masterPane}
        detail={detailPane}
      />
    </div>
  );
}
