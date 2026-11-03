import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CommandCardsView, SurfaceCard } from '../CommandCardsView';
import { MensServePanel } from './MensServePanel';
import { Glass } from '../../ui/Glass';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { Toast } from '../../../types/tauri';

interface MensTrainingViewProps {
  pushToast: (item: Toast) => void;
}

interface GateDto {
  name: string;
  passed: boolean;
  message: string;
}

interface GateReceiptDto {
  overall_passed: boolean;
  substantive_gate_count: number;
  failed_gates: string[];
  gates: GateDto[];
}

interface CollateralDamageReportDto {
  status: string;
  failed_on: string | null;
}

interface EvalLocalReportDto {
  anti_stub_task_success: number;
  pass_rate_at_k: number;
  placeholder_event_rate: number;
}

interface SpokeScoreRowDto {
  spoke: string;
  total_cases: number;
  before_pass_at_1: number;
  mid_pass_at_1: number;
  pass_at_1: number;
  pass_at_k: number;
  delta_pass_at_1: number;
  syntax_validity_rate: number;
  semantic_validity_rate: number;
  execution_success_rate: number;
  curriculum_weight: number;
}

interface MensSpokeScorecardDto {
  schema_version: string;
  total_cases: number;
  before_overall_pass_at_1: number;
  mid_overall_pass_at_1: number;
  overall_pass_at_1: number;
  overall_pass_at_k: number;
  overall_delta_pass_at_1: number;
  monotonic_improvement_verified: boolean;
  promotion_gate_passed: boolean;
  promoted_model_id: string;
  promoted_adapter_uri: string;
  spokes: SpokeScoreRowDto[];
}

interface OrganicSpokeRowDto {
  spoke: string;
  total_cases: number;
  search_recall_at_5: number;
  mean_reciprocal_rank: number;
  mean_grounding_ratio: number;
  syntax_validity_rate: number;
  semantic_validity_rate: number;
  compilation_pass_rate: number;
  end_to_end_pass_at_1: number;
}

interface OrganicSearchSpokeScorecardDto {
  schema_version: string;
  total_cases: number;
  overall_search_recall_at_5: number;
  overall_mrr: number;
  overall_grounding_ratio: number;
  overall_compilation_pass_rate: number;
  overall_end_to_end_pass_at_1: number;
  spokes: OrganicSpokeRowDto[];
}

interface MensRunReportsDto {
  eval_local: EvalLocalReportDto | null;
  gate_receipt: GateReceiptDto | null;
  collateral_damage: CollateralDamageReportDto | null;
  spoke_scorecard?: MensSpokeScorecardDto | null;
  organic_scorecard?: OrganicSearchSpokeScorecardDto | null;
}

/**
 * Visual tone for the gate receipt line. `assert_serve_preconditions`
 * (Task A2, `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs`)
 * refuses to serve unless `substantive_gate_count >= 2`, so a receipt below
 * that threshold is already backend-rejected evidence and must never render
 * the same as a real pass — it gets "warn" instead of falling through to
 * "pass".
 */
function gateTone(receipt: GateReceiptDto): 'pass' | 'warn' | 'fail' {
  if (!receipt.overall_passed) return 'fail';
  if (receipt.substantive_gate_count < 2) return 'warn';
  return 'pass';
}

const TONE_CLASS: Record<'pass' | 'warn' | 'fail', string> = {
  pass: 'text-emerald-400',
  warn: 'text-amber-400',
  fail: 'text-red-400',
};

/**
 * Reads and renders the MENS eval/gate report JSON files a previous,
 * separately-triggered `mens eval-local` / `mens eval-gate` /
 * `mens eval-collateral-damage` run already wrote to `runDir` — via
 * `mens_run_reports` (`crates/vox-gui/src/commands/mens_run_reports.rs`).
 * Deliberately NOT a `CommandCardsView` card: those run on every mount, and
 * the eval commands themselves are expensive real GPU runs. This only reads
 * static files, on demand, when the user names a run directory.
 */
function MensRunReportsPanel({ pushToast }: { pushToast: (item: Toast) => void }) {
  const [runDir, setRunDir] = useState('');
  const [reports, setReports] = useState<MensRunReportsDto | null>(null);

  const loadReports = async (dir: string) => {
    if (!dir.trim()) {
      setReports(null);
      return;
    }
    try {
      const result = await invoke<MensRunReportsDto>('mens_run_reports', { runDir: dir });
      setReports(result);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'mens run reports failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  return (
    <Glass className="p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">
          Run Reports &amp; 5-Spoke Scorecard
        </div>
        <button
          type="button"
          onClick={() => {
            setRunDir('workspace');
            void loadReports('workspace');
          }}
          className="rounded border border-border-subtle px-2 py-0.5 font-mono text-[10px] text-text-primary hover:border-text-muted"
        >
          Load 5-Spoke Benchmark
        </button>
      </div>
      <label className="flex flex-col gap-1 text-[10px] uppercase tracking-widest text-text-muted">
        Run directory
        <input
          aria-label="Run directory"
          type="text"
          value={runDir}
          onChange={(e) => setRunDir(e.target.value)}
          onBlur={() => loadReports(runDir)}
          placeholder="/path/to/mens/run/dir or workspace"
          className="rounded-lg border border-border-subtle bg-transparent px-2 py-1.5 font-mono text-xs text-text-primary"
        />
      </label>
      {reports && (
        <div className="space-y-2 font-mono text-[11px]">
          {reports.gate_receipt ? (
            (() => {
              const tone = gateTone(reports.gate_receipt);
              return (
                <div data-testid="gate-receipt-status" className={TONE_CLASS[tone]}>
                  {tone === 'fail' && 'Gate FAILED'}
                  {tone === 'warn' &&
                    `Gate passed, but only ${reports.gate_receipt.substantive_gate_count} substantive gate${
                      reports.gate_receipt.substantive_gate_count === 1 ? '' : 's'
                    } — not enough evidence to serve`}
                  {tone === 'pass' && 'Gate passed'}
                  {' '}({reports.gate_receipt.substantive_gate_count} substantive gates)
                  {reports.gate_receipt.failed_gates.length > 0 && (
                    <span> — failed: {reports.gate_receipt.failed_gates.join(', ')}</span>
                  )}
                </div>
              );
            })()
          ) : (
            <div className="text-text-muted">No gate_receipt.json in this run dir</div>
          )}
          {reports.collateral_damage && (
            <div className={reports.collateral_damage.status === 'fail' ? TONE_CLASS.fail : TONE_CLASS.pass}>
              {reports.collateral_damage.status === 'fail'
                ? `Collateral damage FAILED — ${reports.collateral_damage.failed_on ?? 'unknown benchmark'}`
                : 'Collateral damage OK'}
            </div>
          )}
          {reports.eval_local && (
            <div className="text-text-muted">
              eval-local anti-stub {(reports.eval_local.anti_stub_task_success * 100).toFixed(0)}%
            </div>
          )}
          {reports.spoke_scorecard && (
            <div data-testid="mens-spoke-scorecard" className="space-y-2 pt-2 border-t border-border-subtle">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className={reports.spoke_scorecard.promotion_gate_passed ? TONE_CLASS.pass : TONE_CLASS.warn}>
                  5-Spoke pass@1: {(reports.spoke_scorecard.before_overall_pass_at_1 * 100).toFixed(0)}% →{' '}
                  {(reports.spoke_scorecard.mid_overall_pass_at_1 * 100).toFixed(0)}% →{' '}
                  {(reports.spoke_scorecard.overall_pass_at_1 * 100).toFixed(0)}% (+
                  {(reports.spoke_scorecard.overall_delta_pass_at_1 * 100).toFixed(0)}%)
                </span>
                <span className="text-text-muted text-[10px]">
                  {reports.spoke_scorecard.promoted_model_id} ({reports.spoke_scorecard.total_cases} cases)
                </span>
              </div>
              <div className="grid grid-cols-5 gap-1 text-[10px] text-text-muted uppercase">
                <span>Spoke</span>
                <span>Before</span>
                <span>After</span>
                <span>Delta</span>
                <span>Curriculum w</span>
              </div>
              {reports.spoke_scorecard.spokes.map((s) => (
                <div key={s.spoke} className="grid grid-cols-5 gap-1 text-[11px] text-text-primary">
                  <span>{s.spoke}</span>
                  <span>{(s.before_pass_at_1 * 100).toFixed(0)}%</span>
                  <span className={s.pass_at_1 >= 1.0 ? TONE_CLASS.pass : TONE_CLASS.warn}>
                    {(s.pass_at_1 * 100).toFixed(0)}%
                  </span>
                  <span className={TONE_CLASS.pass}>+{(s.delta_pass_at_1 * 100).toFixed(0)}%</span>
                  <span>{s.curriculum_weight.toFixed(2)}</span>
                </div>
              ))}
            </div>
          )}
          {reports.organic_scorecard && (
            <div data-testid="mens-organic-scorecard" className="space-y-2 pt-2 border-t border-border-subtle">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className={reports.organic_scorecard.overall_end_to_end_pass_at_1 >= 1.0 ? TONE_CLASS.pass : TONE_CLASS.warn}>
                  Organic Search → Compilation: Recall@5 {(reports.organic_scorecard.overall_search_recall_at_5 * 100).toFixed(0)}% | MRR {reports.organic_scorecard.overall_mrr.toFixed(2)} | Grounding {(reports.organic_scorecard.overall_grounding_ratio * 100).toFixed(0)}% | Compile {(reports.organic_scorecard.overall_compilation_pass_rate * 100).toFixed(0)}% | E2E {(reports.organic_scorecard.overall_end_to_end_pass_at_1 * 100).toFixed(0)}%
                </span>
                <span className="text-text-muted text-[10px]">
                  {reports.organic_scorecard.total_cases} organic cases
                </span>
              </div>
              <div className="grid grid-cols-5 gap-1 text-[10px] text-text-muted uppercase">
                <span>Spoke</span>
                <span>Recall@5</span>
                <span>MRR</span>
                <span>Compile</span>
                <span>E2E pass@1</span>
              </div>
              {reports.organic_scorecard.spokes.map((s) => (
                <div key={s.spoke} className="grid grid-cols-5 gap-1 text-[11px] text-text-primary">
                  <span>{s.spoke}</span>
                  <span>{(s.search_recall_at_5 * 100).toFixed(0)}%</span>
                  <span>{s.mean_reciprocal_rank.toFixed(2)}</span>
                  <span className={s.compilation_pass_rate >= 1.0 ? TONE_CLASS.pass : TONE_CLASS.warn}>
                    {(s.compilation_pass_rate * 100).toFixed(0)}%
                  </span>
                  <span className={s.end_to_end_pass_at_1 >= 1.0 ? TONE_CLASS.pass : TONE_CLASS.warn}>
                    {(s.end_to_end_pass_at_1 * 100).toFixed(0)}%
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Glass>
  );
}

/**
 * `mens` surface decorator.
 *
 * Fixes a real defect: the GPU Probe card promised "Detected accelerators +
 * LoRA fit" (`decoratorRegistry.ts`) but ran `vox mens probe` with a
 * hardcoded empty argv, so `probe.rs` never entered its (then verbose-gated)
 * fit-check block — the card only ever showed a raw VRAM number, never
 * whether the model the user actually selected would fit.
 *
 * This asks the CLI to check the fit of THAT model: it reads the same
 * backend state `ModelsView` uses (`get_active_model`) and passes it to
 * `vox mens probe --detailed --model <id>`, the dry run of
 * `vox mens train --model <id>` (see `render_verdict` in
 * `crates/vox-populi/src/mens/tensor/memory_model.rs`).
 *
 * `--model <id>` is safe to pass unconditionally here even though
 * `CommandCardsView` runs every card on mount and on every Refresh
 * click: `probe.rs`'s `run_probe` only runs the real (downloading) fit
 * check when `hub::is_model_cached(id)` says the model is already on
 * disk; otherwise it falls back to the VRAM-only `recommend_config`
 * profile, so simply viewing this surface never triggers a fresh
 * multi-GB Hugging Face download.
 */
export function MensTrainingView({ pushToast }: MensTrainingViewProps) {
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;
    invoke<string | null>('get_active_model')
      .then((m) => {
        if (!cancelled) setActiveModel(m);
      })
      .catch(() => {
        // No active model resolvable — fall through to a model-less probe
        // (still useful: hardware discovery + the VRAM-only fallback).
      })
      .finally(() => {
        if (!cancelled) setReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!ready) {
    return null;
  }

  const probeArgv = activeModel ? ['--detailed', '--model', activeModel] : ['--detailed'];

  const cards: SurfaceCard[] = [
    { key: 'status', title: 'Training Status', description: 'Latest run telemetry', path: ['mens', 'status'] },
    { key: 'models', title: 'Model Registry', description: 'Locally trained models', path: ['mens', 'models'] },
    {
      key: 'probe',
      title: 'GPU Probe',
      description: 'Detected accelerators + LoRA fit',
      path: ['mens', 'probe'],
      argv: probeArgv,
    },
  ];

  return (
    <div className="space-y-4">
      <MensServePanel pushToast={pushToast} />
      <MensRunReportsPanel pushToast={pushToast} />
      <CommandCardsView
        title="Vox Mens"
        subtitle="ML training & local models"
        cards={cards}
        pushToast={pushToast}
      />
    </div>
  );
}
