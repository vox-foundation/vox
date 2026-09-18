import React, { useState, useEffect, useMemo } from 'react';
import {
  X,
  Play,
  Pause,
  ChevronRight,
  RotateCcw,
  CheckCircle2,
  AlertCircle,
  Clock,
  ShieldAlert,
} from 'lucide-react';
import { usePipelineStepper, type UsePipelineStepperReturn } from './usePipelineStepper';
import type { StepStatus } from './types';

export interface InspectorDrawerProps {
  open: boolean;
  onClose: () => void;
  stepper?: UsePipelineStepperReturn;
}

const STATUS_ICONS: Record<StepStatus, React.ReactNode> = {
  pending: <Clock className="size-3 text-text-muted" />,
  active: <Play className="size-3 text-brass fill-brass" />,
  paused: <Pause className="size-3 text-blue-400 fill-blue-400" />,
  completed: <CheckCircle2 className="size-3 text-emerald-400" />,
  failed: <AlertCircle className="size-3 text-rose-400" />,
  skipped: <Clock className="size-3 text-text-muted opacity-50" />,
};

const STATUS_BADGE_CLASSES: Record<StepStatus, string> = {
  pending: 'bg-overlay-subtle text-text-muted border-white/5',
  active: 'bg-brass/10 text-brass border-brass/30 animate-pulse',
  paused: 'bg-blue-500/10 text-blue-400 border-blue-500/30',
  completed: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30',
  failed: 'bg-rose-500/10 text-rose-400 border-rose-500/30',
  skipped: 'bg-overlay-subtle text-text-muted border-white/5 opacity-60',
};

export function InspectorDrawer({ open, onClose, stepper: externalStepper }: InspectorDrawerProps) {
  const internalStepper = usePipelineStepper();
  const stepper = externalStepper ?? internalStepper;

  const [selectedStageId, setSelectedStageId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'payloads' | 'violations'>('payloads');

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  const activeStageId = stepper.steps[stepper.currentStageIndex]?.id;
  const currentInspectId = selectedStageId ?? activeStageId;
  const inspectedStep = useMemo(() => {
    return stepper.steps.find(s => s.id === currentInspectId) ?? stepper.currentStep;
  }, [stepper.steps, currentInspectId, stepper.currentStep]);

  const isPlaying = stepper.autoPlay && !stepper.isPaused;

  const handleReset = () => {
    setSelectedStageId(null);
    stepper.reset();
  };

  if (!open) return null;

  return (
    <div
      data-testid="inspector-drawer"
      role="dialog"
      aria-label="Pipeline Inspector & Debugger"
      className="fixed inset-y-0 right-0 z-45 w-[420px] bg-bg-surface border-l border-border-subtle shadow-2xl flex flex-col"
    >
      {/* Drawer Header */}
      <div className="flex items-center justify-between border-b border-border-subtle px-4 py-3 bg-overlay-subtle">
        <div>
          <h2 className="font-display text-sm font-semibold tracking-tight text-text-primary">
            Pipeline Inspector &amp; Debugger
          </h2>
          <span className="text-[11px] text-text-muted font-mono">
            Stage {stepper.currentStageIndex + 1} of {stepper.steps.length}
          </span>
        </div>
        <button
          type="button"
          aria-label="Close inspector"
          onClick={onClose}
          className="flex size-7 items-center justify-center rounded-md text-text-muted hover:bg-overlay-hover hover:text-text-primary transition-colors"
        >
          <X className="size-4" aria-hidden="true" />
        </button>
      </div>

      {/* Stepper Toolbar */}
      <div className="flex items-center gap-1.5 border-b border-border-subtle px-4 py-2 bg-bg-surface">
        <button
          type="button"
          aria-label={isPlaying ? 'Pause' : 'Play'}
          onClick={isPlaying ? stepper.pause : stepper.resume}
          className="flex items-center gap-1.5 rounded border border-border-subtle bg-overlay-subtle px-2.5 py-1 text-xs text-text-secondary hover:bg-overlay-hover hover:text-text-primary transition-colors"
        >
          {isPlaying ? <Pause className="size-3.5" /> : <Play className="size-3.5" />}
          <span>{isPlaying ? 'Pause' : 'Play'}</span>
        </button>

        <button
          type="button"
          aria-label="Step Next"
          onClick={stepper.stepNext}
          className="flex items-center gap-1.5 rounded border border-border-subtle bg-overlay-subtle px-2.5 py-1 text-xs text-text-secondary hover:bg-overlay-hover hover:text-text-primary transition-colors"
        >
          <ChevronRight className="size-3.5" />
          <span>Step Next</span>
        </button>

        <button
          type="button"
          aria-label="Reset"
          onClick={handleReset}
          className="flex items-center gap-1.5 rounded border border-border-subtle bg-overlay-subtle px-2.5 py-1 text-xs text-text-secondary hover:bg-overlay-hover hover:text-text-primary transition-colors"
        >
          <RotateCcw className="size-3.5" />
          <span>Reset</span>
        </button>

        <button
          type="button"
          aria-label="Audit Invariants"
          onClick={() => stepper.auditInvariants()}
          className="ml-auto flex items-center gap-1 rounded border border-brass/30 bg-brass/10 px-2 py-1 text-xs text-brass hover:bg-brass/20 transition-colors"
        >
          <ShieldAlert className="size-3.5" />
          <span>Audit</span>
        </button>
      </div>

      {/* 8-Stage Timeline */}
      <div className="border-b border-border-subtle p-3 bg-overlay-subtle/50">
        <div className="mb-2 flex items-center justify-between">
          <span className="font-display text-[11px] uppercase tracking-wider text-text-muted font-medium">
            Execution Stages
          </span>
          <span className="font-mono text-[10px] text-text-muted">
            {isPlaying ? 'RUNNING' : stepper.isPaused ? 'PAUSED' : 'STEPPER'}
          </span>
        </div>
        <div className="flex flex-col gap-1 max-h-48 overflow-y-auto pr-1">
          {stepper.steps.map((step, idx) => {
            const isCurrent = idx === stepper.currentStageIndex;
            const isSelected = step.id === currentInspectId;
            return (
              <button
                key={step.id}
                type="button"
                onClick={() => setSelectedStageId(step.id)}
                className={`flex items-center justify-between rounded px-2.5 py-1.5 text-left text-xs transition-colors border ${
                  isSelected
                    ? 'border-brass/40 bg-brass/10 text-text-primary'
                    : isCurrent
                    ? 'border-border-subtle bg-overlay-subtle text-text-primary'
                    : 'border-transparent text-text-secondary hover:bg-overlay-subtle'
                }`}
              >
                <div className="flex items-center gap-2 min-w-0">
                  <span className="flex size-4 items-center justify-center">
                    {STATUS_ICONS[step.status]}
                  </span>
                  <span className="font-mono text-[11px] truncate">{step.id}</span>
                </div>
                <div className="flex items-center gap-1.5 ml-2 shrink-0">
                  <span
                    className={`rounded border px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-wider ${
                      STATUS_BADGE_CLASSES[step.status]
                    }`}
                  >
                    {step.status}
                  </span>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Tabs Header */}
      <div className="flex items-center border-b border-border-subtle bg-overlay-subtle/30 px-3">
        <button
          type="button"
          onClick={() => setActiveTab('payloads')}
          className={`flex items-center gap-1.5 border-b-2 px-3 py-2 text-xs font-medium transition-colors ${
            activeTab === 'payloads'
              ? 'border-brass text-text-primary'
              : 'border-transparent text-text-muted hover:text-text-secondary'
          }`}
        >
          <span>Payloads</span>
          {inspectedStep && (
            <span className="rounded bg-overlay-subtle px-1.5 py-0.2 font-mono text-[10px] text-text-muted">
              {inspectedStep.id}
            </span>
          )}
        </button>

        <button
          type="button"
          onClick={() => setActiveTab('violations')}
          className={`flex items-center gap-1.5 border-b-2 px-3 py-2 text-xs font-medium transition-colors ${
            activeTab === 'violations'
              ? 'border-brass text-text-primary'
              : 'border-transparent text-text-muted hover:text-text-secondary'
          }`}
        >
          <span>Violations</span>
          {stepper.violations.length > 0 && (
            <span className="rounded-full bg-rose-500/20 border border-rose-500/30 px-1.5 py-0.2 font-mono text-[10px] text-rose-400 font-semibold">
              {stepper.violations.length}
            </span>
          )}
        </button>
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-y-auto p-3">
        {activeTab === 'payloads' && (
          <div className="flex flex-col gap-3">
            {inspectedStep ? (
              <>
                <div className="flex items-center justify-between pb-1 border-b border-border-subtle">
                  <span className="font-mono text-xs font-semibold text-text-primary">
                    Stage: {inspectedStep.id}
                  </span>
                  <span
                    className={`rounded border px-1.5 py-0.5 font-mono text-[10px] uppercase ${
                      STATUS_BADGE_CLASSES[inspectedStep.status]
                    }`}
                  >
                    {inspectedStep.status}
                  </span>
                </div>

                {inspectedStep.error && (
                  <div className="rounded border border-rose-500/30 bg-rose-500/10 p-2 text-xs text-rose-300">
                    <span className="font-semibold">Error:</span> {inspectedStep.error}
                  </div>
                )}

                <div>
                  <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-text-muted">
                    Input Payload
                  </div>
                  {inspectedStep.inputPayload !== undefined ? (
                    <pre className="max-h-40 overflow-auto rounded border border-border-subtle bg-black/40 p-2 font-mono text-[11px] text-text-secondary">
                      {JSON.stringify(inspectedStep.inputPayload, null, 2)}
                    </pre>
                  ) : (
                    <div className="rounded border border-dashed border-border-subtle p-2 text-[11px] italic text-text-muted">
                      No input payload recorded
                    </div>
                  )}
                </div>

                <div>
                  <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-text-muted">
                    Output Payload
                  </div>
                  {inspectedStep.outputPayload !== undefined ? (
                    <pre className="max-h-48 overflow-auto rounded border border-border-subtle bg-black/40 p-2 font-mono text-[11px] text-text-secondary">
                      {JSON.stringify(inspectedStep.outputPayload, null, 2)}
                    </pre>
                  ) : (
                    <div className="rounded border border-dashed border-border-subtle p-2 text-[11px] italic text-text-muted">
                      No output payload recorded
                    </div>
                  )}
                </div>
              </>
            ) : (
              <div className="text-center py-6 text-xs text-text-muted">
                No stage selected for inspection.
              </div>
            )}
          </div>
        )}

        {activeTab === 'violations' && (
          <div className="flex flex-col gap-2">
            {stepper.violations.length === 0 ? (
              <div className="rounded border border-dashed border-border-subtle py-8 text-center text-xs text-text-muted">
                <ShieldAlert className="mx-auto mb-2 size-6 text-emerald-400 opacity-60" />
                <p className="font-medium text-text-secondary">No invariant violations</p>
                <p className="mt-1 text-[11px]">Pipeline sentries report clean state</p>
              </div>
            ) : (
              stepper.violations.map((v, i) => (
                <div
                  key={i}
                  className="rounded border border-border-subtle bg-bg-elevated/40 p-2.5 flex flex-col gap-1"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-xs font-semibold text-text-primary">
                      {v.kind}
                    </span>
                    <span
                      className={`rounded border px-1.5 py-0.5 font-mono text-[10px] uppercase font-medium ${
                        v.severity === 'critical'
                          ? 'border-rose-500/30 bg-rose-500/20 text-rose-400'
                          : v.severity === 'major'
                          ? 'border-amber-500/30 bg-amber-500/20 text-amber-400'
                          : 'border-blue-500/30 bg-blue-500/20 text-blue-400'
                      }`}
                    >
                      {v.severity}
                    </span>
                  </div>
                  <p className="text-xs text-text-secondary leading-snug">{v.message}</p>
                  {v.location && (
                    <div className="font-mono text-[10px] text-text-muted">
                      <span className="text-text-secondary">Location:</span> {v.location}
                    </div>
                  )}
                  {v.elementSelector && (
                    <div className="font-mono text-[10px] text-text-muted">
                      <span className="text-text-secondary">Selector:</span> {v.elementSelector}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  );
}
