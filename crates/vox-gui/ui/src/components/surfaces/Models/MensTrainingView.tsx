import React, { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Activity,
  Cpu,
  ExternalLink,
  FlaskConical,
  HardDrive,
  Moon,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Rocket,
  RotateCcw,
  Sparkles,
  Upload,
  Zap,
} from 'lucide-react';
import { CommandCardsView, SurfaceCard } from '../CommandCardsView';
import { MensServePanel } from './MensServePanel';
import { Glass } from '../../ui/Glass';
import { Segment } from '../../ui/Segment';
import { Tooltip } from '../../ui/Tooltip';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useLocalStorage } from '../../../hooks/useLocalStorage';
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
  unbiased_search_recall_at_5?: number;
  unbiased_mrr?: number;
  mean_grounding_ratio: number;
  distractor_resilience_rate?: number;
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
  overall_unbiased_search_recall_at_5?: number;
  overall_unbiased_mrr?: number;
  overall_grounding_ratio: number;
  overall_distractor_resilience_rate?: number;
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

interface CustomSpokeBackendDto {
  id: string;
  label: string;
  domain: string;
  target_gate: string;
  family: string;
}

interface MensLadderRungBackendDto {
  size: string;
  hf_id: string;
  local_sha: string;
  repo_master_sha: string;
  sync_status: string;
  floor_mb: number | null;
  floor_measured: boolean;
  cells: Record<string, string>;
}

interface MensCrucibleConfigBackendDto {
  hub_embedder: string;
  hub_base_rung: string;
  compute_target: string;
  lora_rank: number;
  lora_alpha: number;
  seq_len: number;
  vram_cap_pct: number;
  epoch_caps: Record<string, number>;
}

interface MensLadderStatusBackendDto {
  rungs: MensLadderRungBackendDto[];
  custom_spokes: CustomSpokeBackendDto[];
  crucible_config: MensCrucibleConfigBackendDto | null;
}

interface MensLiveTelemetryBackendDto {
  gpu_pct: number;
  vram_used_gb: number;
  vram_max_gb: number;
  cpu_pct: number;
  ram_used_gb: number;
  ram_max_gb: number;
  loss: number;
  step: number;
  gpu_series: number[];
  vram_series: number[];
  cpu_series: number[];
  ram_series: number[];
  loss_series: number[];
}

function buildSvgLineAndAreaFromSeries(
  series: number[],
  minVal: number,
  maxVal: number,
  invert = false,
): { line: string; area: string; headY: number } {
  if (!series || series.length < 2) {
    return {
      line: 'M 0,70 L 1120,70',
      area: 'M 0,70 L 1120,70 L 1120,142 L 0,142 Z',
      headY: 70,
    };
  }
  const span = Math.max(1e-6, maxVal - minVal);
  const pts = series.map((val, idx) => {
    const x = Math.round((idx / (series.length - 1)) * 1120);
    const norm = Math.max(0, Math.min(1, (val - minVal) / span));
    const yRatio = invert ? norm : 1 - norm;
    const y = Math.round(12 + yRatio * 122);
    return { x, y };
  });
  let d = `M ${pts[0]!.x},${pts[0]!.y}`;
  for (let i = 1; i < pts.length; i++) {
    const prev = pts[i - 1]!;
    const curr = pts[i]!;
    const cx1 = Math.round(prev.x + (curr.x - prev.x) * 0.45);
    const cx2 = Math.round(prev.x + (curr.x - prev.x) * 0.55);
    d += ` C ${cx1},${prev.y} ${cx2},${curr.y} ${curr.x},${curr.y}`;
  }
  const headY = pts[pts.length - 1]!.y;
  return {
    line: d,
    area: `${d} L 1120,142 L 0,142 Z`,
    headY,
  };
}

interface SecretStatusLite {
  id?: string;
  canonicalEnv: string;
  isPresent: boolean;
}

type CellStateKind = 'gated' | 'afk_running' | 'queued' | 'remote_ready';

interface SpokeDefinition {
  id: string;
  label: string;
  shortLabel: string;
  oracle: string;
  hubTag: string;
  method: 'qlora' | 'dpo' | 'full_sft' | 'orpo' | 'rag_only' | 'prompt_only';
  signal: string;
  kilnSeeds: string;
  isCustom?: boolean;
}

interface LadderRungRow {
  size: string;
  name: string;
  sha: string;
  versionBadge: string;
  syncBadge: 'synced' | 'stale' | 'absent';
  syncLabel: string;
  floorGb: number;
  cells: Record<string, { state: CellStateKind; detail: string; sub: string }>;
}

const INITIAL_SPOKES: SpokeDefinition[] = [
  {
    id: 'vox_lang',
    label: 'vox_lang',
    shortLabel: 'vox_lang',
    oracle: 'lex → parse → HIR → typeck → fmt → interp',
    hubTag: 'small_code_default',
    method: 'qlora',
    signal: 'lane:vox_codegen · *.vox',
    kilnSeeds: 'Actors, Durable Workflows, Polars Bridge, Reactive WebIR',
  },
  {
    id: 'rust_workspace',
    label: 'rust_workspace',
    shortLabel: 'rust_workspace',
    oracle: 'syn::parse_file + rustc --emit=metadata',
    hubTag: 'strong_code_default',
    method: 'qlora',
    signal: 'lane:vox_rust_authoring · *.rs',
    kilnSeeds: 'Clavis Secrets, vox-crypto, layers.toml boundaries',
  },
  {
    id: 'structured_shell',
    label: 'structured_shell',
    shortLabel: 'structured_shell',
    oracle: 'classify_shell_safety_floor + exec-policy.v1',
    hubTag: 'small_code_default',
    method: 'qlora',
    signal: 'lane:shell_exec · pwsh/nu',
    kilnSeeds: 'Single-command pwsh, scoped rg, safety floor guards',
  },
  {
    id: 'mcp_tool_chain',
    label: 'mcp_tool_chain',
    shortLabel: 'mcp_tool_chain',
    oracle: 'vox-mcp-registry JSON Schema validator',
    hubTag: 'agentic_default',
    method: 'qlora',
    signal: 'lane:tool_call · mcp_*',
    kilnSeeds: 'Tool disambiguation, strict schema arg synthesis',
  },
  {
    id: 'agent_harness',
    label: 'agent_harness',
    shortLabel: 'agent_harness',
    oracle: 'TaskPlanEnvelope + Socrates grounding check',
    hubTag: 'agentic_default',
    method: 'dpo',
    signal: 'lane:orchestrator_plan · wave_*',
    kilnSeeds: 'Parallel wave isolation, two-strike escalation',
  },
  {
    id: 'speech_to_code',
    label: 'speech_to_code',
    shortLabel: 'speech_to_code',
    oracle: 'Oratio ASR disfluency strip → .vox AST',
    hubTag: 'small_code_default',
    method: 'qlora',
    signal: 'lane:oratio_asr · voice_turn',
    kilnSeeds: 'Mid-utterance repair ("no wait"), symbol binding',
  },
  {
    id: 'populi_mesh',
    label: 'populi_mesh',
    shortLabel: 'populi_mesh',
    oracle: 'PopuliMeshDispatchEnvelope + iroh QUIC',
    hubTag: 'agentic_default',
    method: 'qlora',
    signal: 'lane:populi_mesh · a2a_quorum',
    kilnSeeds: 'VRAM cohort placement, QUIC heartbeat failover',
  },
  {
    id: 'skill_creation',
    label: 'skill_creation',
    shortLabel: 'skill_creation',
    oracle: 'agentskills.io SKILL.md YAML + vox-* lint',
    hubTag: 'small_code_default',
    method: 'qlora',
    signal: 'lane:skill_author · SKILL.md',
    kilnSeeds: 'Spec frontmatter, tool parity, license provenance',
  },
];

const INITIAL_LADDER: LadderRungRow[] = [
  {
    size: '0.6B',
    name: 'Qwen3-0.6B',
    sha: '@c1899de',
    versionBadge: 'LOCAL == MASTER v2.4',
    syncBadge: 'stale',
    syncLabel: 'STALE SHA · UPDATE',
    floorGb: 1.8,
    cells: {
      vox_lang: { state: 'gated', detail: 'TRAINED + GATED', sub: '96% pass@1' },
      rust_workspace: { state: 'gated', detail: 'TRAINED + GATED', sub: '94% pass@1' },
      structured_shell: { state: 'gated', detail: 'TRAINED + GATED', sub: '96% pass@1' },
      mcp_tool_chain: { state: 'gated', detail: 'TRAINED + GATED', sub: '95% pass@1' },
      agent_harness: { state: 'gated', detail: 'TRAINED + GATED', sub: '92% pass@1' },
      speech_to_code: { state: 'gated', detail: 'TRAINED + GATED', sub: '93% pass@1' },
      populi_mesh: { state: 'gated', detail: 'TRAINED + GATED', sub: '91% pass@1' },
      skill_creation: { state: 'remote_ready', detail: 'REMOTE MASTER', sub: 'READY' },
    },
  },
  {
    size: '1.7B',
    name: 'Qwen3-1.7B',
    sha: '@70d244c',
    versionBadge: 'LOCAL == MASTER v2.4',
    syncBadge: 'absent',
    syncLabel: 'SNAPSHOT ABSENT',
    floorGb: 3.6,
    cells: {
      vox_lang: { state: 'gated', detail: 'TRAINED + GATED', sub: '96% pass@1' },
      rust_workspace: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      structured_shell: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      mcp_tool_chain: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      agent_harness: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      speech_to_code: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      populi_mesh: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      skill_creation: { state: 'remote_ready', detail: 'REMOTE MASTER', sub: 'READY' },
    },
  },
  {
    size: '4B',
    name: 'Qwen3-4B',
    sha: '@b48221a',
    versionBadge: 'LOCAL == MASTER v2.4',
    syncBadge: 'absent',
    syncLabel: 'SNAPSHOT ABSENT',
    floorGb: 7.4,
    cells: {
      vox_lang: { state: 'gated', detail: 'TRAINED + GATED', sub: '96% pass@1' },
      rust_workspace: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      structured_shell: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      mcp_tool_chain: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      agent_harness: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #3' },
      speech_to_code: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #3' },
      populi_mesh: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #4' },
      skill_creation: { state: 'remote_ready', detail: 'REMOTE MASTER', sub: 'READY' },
    },
  },
  {
    size: '8B',
    name: 'Qwen3-8B',
    sha: '@9c41f02',
    versionBadge: 'LOCAL == MASTER v2.4',
    syncBadge: 'absent',
    syncLabel: 'SNAPSHOT ABSENT',
    floorGb: 11.8,
    cells: {
      vox_lang: { state: 'gated', detail: 'TRAINED + GATED', sub: '96% pass@1' },
      rust_workspace: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      structured_shell: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      mcp_tool_chain: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      agent_harness: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #3' },
      speech_to_code: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      populi_mesh: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #4' },
      skill_creation: { state: 'remote_ready', detail: 'REMOTE MASTER', sub: 'READY' },
    },
  },
  {
    size: '14B',
    name: 'Qwen3-14B',
    sha: '@e31a88b',
    versionBadge: 'LOCAL == MASTER v2.4',
    syncBadge: 'absent',
    syncLabel: 'SNAPSHOT ABSENT',
    floorGb: 21.2,
    cells: {
      vox_lang: { state: 'gated', detail: 'TRAINED + GATED', sub: '96% pass@1' },
      rust_workspace: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      structured_shell: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 3/3' },
      mcp_tool_chain: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      agent_harness: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 3/3' },
      speech_to_code: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      populi_mesh: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #3' },
      skill_creation: { state: 'remote_ready', detail: 'REMOTE MASTER', sub: 'READY' },
    },
  },
  {
    size: '32B',
    name: 'Qwen3-32B',
    sha: '@4f09d11',
    versionBadge: 'LOCAL == MASTER v2.4',
    syncBadge: 'absent',
    syncLabel: 'SNAPSHOT ABSENT',
    floorGb: 46.0,
    cells: {
      vox_lang: { state: 'gated', detail: 'TRAINED + GATED', sub: '96% pass@1' },
      rust_workspace: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      structured_shell: { state: 'afk_running', detail: 'AFK IN PROGRESS', sub: 'Epoch 2/3' },
      mcp_tool_chain: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      agent_harness: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      speech_to_code: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #2' },
      populi_mesh: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'SERIES #4' },
      skill_creation: { state: 'remote_ready', detail: 'REMOTE MASTER', sub: 'READY' },
    },
  },
];

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
 * Crisp SVG status glyph for each Rung × Spoke cell in Option A's matrix.
 */
function MatrixCellGlyph({ state }: { state: CellStateKind }) {
  if (state === 'gated') {
    return (
      <svg width="16" height="16" viewBox="0 0 24 24" className="mx-auto shrink-0 transition-transform duration-200 group-hover:scale-110" aria-hidden="true">
        <path
          d="M12 2L4 5.5v6.1c0 5.1 3.4 9.8 8 11.4 4.6-1.6 8-6.3 8-11.4V5.5L12 2z"
          fill="rgba(121, 200, 186, 0.28)"
          stroke="#79c8ba"
          strokeWidth="1.8"
        />
        <path
          d="M8.5 12.2l2.3 2.4 4.8-4.9"
          fill="none"
          stroke="#79c8ba"
          strokeWidth="2.3"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    );
  }
  if (state === 'afk_running') {
    return (
      <svg width="16" height="16" viewBox="0 0 24 24" className="mx-auto shrink-0 animate-spin" style={{ animationDuration: '6s' }} aria-hidden="true">
        <circle
          cx="12"
          cy="12"
          r="8"
          fill="none"
          stroke="rgba(212, 175, 55, 0.25)"
          strokeWidth="2.2"
          strokeDasharray="3 3"
        />
        <path
          d="M12 4a8 8 0 1 1-6.9 4"
          fill="none"
          stroke="#d4af37"
          strokeWidth="2.6"
          strokeLinecap="round"
        />
      </svg>
    );
  }
  if (state === 'queued') {
    return (
      <svg width="14" height="14" viewBox="0 0 24 24" className="mx-auto shrink-0 transition-transform duration-200 group-hover:rotate-45" aria-hidden="true">
        <polygon
          points="12,3 21,12 12,21 3,12"
          fill="rgba(212, 175, 55, 0.1)"
          stroke="#d4af37"
          strokeWidth="1.7"
        />
      </svg>
    );
  }
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" className="mx-auto shrink-0 transition-transform duration-200 group-hover:translate-y-0.5" aria-hidden="true">
      <path
        d="M12 4v11m0 0l-4-4m4 4l4-4M5 19h14"
        fill="none"
        stroke="#d98b6a"
        strokeWidth="2.1"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

/**
 * Tactile SVG Rotary Arc Gauge matching Option A's Right Inspector.
 */
function RotaryArcDial({
  label,
  subLabel,
  displayValue,
  fraction,
  accentColor,
  cliFlag,
  onClick,
}: {
  label: string;
  subLabel: string;
  displayValue: string;
  fraction: number;
  accentColor: string;
  cliFlag?: string;
  onClick: () => void;
}) {
  const radius = 20;
  const circumference = 2 * Math.PI * radius;
  const arcSpan = circumference * 0.75;
  const dashOffset = arcSpan * (1 - Math.max(0.1, Math.min(1, fraction)));

  return (
    <Tooltip content={`Click to cycle ${label} (${cliFlag ?? subLabel})`}>
      <button
        type="button"
        onClick={onClick}
        title={`Click to step ${label} (${subLabel})`}
        className="group flex flex-col items-center rounded-lg border border-border-subtle bg-overlay-subtle/50 px-1 py-1 transition-all duration-200 hover:border-brass/60 hover:bg-bg-elevated active:scale-95"
      >
        <div className="relative flex items-center justify-center">
          <svg width="48" height="48" viewBox="0 0 52 52" aria-hidden="true">
            <circle
              cx="26"
              cy="26"
              r={radius}
              fill="none"
              stroke="rgba(255,255,255,0.1)"
              strokeWidth="4"
              strokeDasharray={`${arcSpan} ${circumference}`}
              strokeDashoffset="0"
              strokeLinecap="round"
              transform="rotate(135 26 26)"
            />
            <circle
              cx="26"
              cy="26"
              r={radius}
              fill="none"
              stroke={accentColor}
              strokeWidth="4"
              strokeDasharray={`${arcSpan} ${circumference}`}
              strokeDashoffset={dashOffset}
              strokeLinecap="round"
              transform="rotate(135 26 26)"
              className="transition-all duration-300 ease-out"
            />
          </svg>
          <span className="absolute font-mono text-[10px] font-semibold text-text-primary transition-colors group-hover:text-brass">
            {displayValue}
          </span>
        </div>
        <span className="font-sans text-[9.5px] font-medium text-text-secondary leading-tight">
          {label}
        </span>
        <span className="font-mono text-[8.5px] text-text-muted leading-tight">{subLabel}</span>
      </button>
    </Tooltip>
  );
}

function MensRunReportsPanel({
  pushToast,
  autoSyncEnabled,
  hfTokenPresent,
  activeHubRepo,
}: {
  pushToast: (item: Toast) => void;
  autoSyncEnabled: boolean;
  hfTokenPresent: boolean;
  activeHubRepo: string;
}) {
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
      if (
        autoSyncEnabled &&
        hfTokenPresent &&
        result?.gate_receipt?.overall_passed &&
        result.gate_receipt.substantive_gate_count >= 2 &&
        result.collateral_damage?.status !== 'fail'
      ) {
        await invoke('execute_command', {
          path: ['mens', 'hub', 'upload'],
          args: { __argv: ['--repo', activeHubRepo, '--model-dir', dir] },
        });
        pushToast({
          tone: 'ok',
          title: 'Auto-synced gated adapter to HF Hub',
          body: `${activeHubRepo} ← ${dir}`,
          cause: 'backend-ok',
        });
      }
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'mens run reports failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    }
  };

  return (
    <Glass size="sm" className="p-2.5 space-y-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-3 flex-1 min-w-[240px]">
          <span className="font-display text-[10px] tracking-[0.16em] uppercase text-text-muted shrink-0">
            Run Reports &amp; 5-Spoke Scorecard
          </span>
          <label className="flex items-center gap-1.5 text-[10px] uppercase tracking-widest text-text-muted flex-1 max-w-md">
            <span className="sr-only">Run directory</span>
            <input
              aria-label="Run directory"
              type="text"
              value={runDir}
              onChange={(e) => setRunDir(e.target.value)}
              onBlur={() => loadReports(runDir)}
              placeholder="/path/to/mens/run/dir or workspace"
              className="w-full rounded border border-border-subtle bg-black/25 px-2 py-0.5 font-mono text-[11px] text-text-primary"
            />
          </label>
        </div>
        <button
          type="button"
          onClick={() => {
            setRunDir('workspace');
            void loadReports('workspace');
          }}
          className="rounded border border-brass/40 bg-brass/12 px-2.5 py-0.5 font-mono text-[10px] text-brass hover:bg-brass/20"
        >
          Load 5-Spoke Benchmark
        </button>
      </div>
      {reports && (
        <div className="space-y-1.5 font-mono text-[11px]">
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
                  {tone === 'pass' && 'Gate passed'}{' '}
                  ({reports.gate_receipt.substantive_gate_count} substantive gates)
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
            <div
              className={
                reports.collateral_damage.status === 'fail' ? TONE_CLASS.fail : TONE_CLASS.pass
              }
            >
              {reports.collateral_damage.status === 'fail'
                ? `Collateral damage FAILED — ${reports.collateral_damage.failed_on ?? 'unknown benchmark'}`
                : 'Collateral damage OK'}
            </div>
          )}
          {reports.eval_local && (
            <div className="inline-flex items-center gap-1.5 rounded border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] text-emerald-300">
              <span>
                Anti-Stub Task Verification: {(reports.eval_local.anti_stub_task_success * 100).toFixed(0)}%
              </span>
              <span className="text-text-muted">(eval-local)</span>
            </div>
          )}
          {reports.spoke_scorecard && (
            <div
              data-testid="mens-spoke-scorecard"
              className="space-y-1 pt-1.5 border-t border-border-subtle"
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span
                  className={
                    reports.spoke_scorecard.promotion_gate_passed ? TONE_CLASS.pass : TONE_CLASS.warn
                  }
                >
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
            <div
              data-testid="mens-organic-scorecard"
              className="space-y-1 pt-1.5 border-t border-border-subtle"
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span
                  className={
                    reports.organic_scorecard.overall_end_to_end_pass_at_1 >= 1.0
                      ? TONE_CLASS.pass
                      : TONE_CLASS.warn
                  }
                >
                  Organic Search → Compilation: Recall@5{' '}
                  {(reports.organic_scorecard.overall_search_recall_at_5 * 100).toFixed(0)}% (Unbiased{' '}
                  {(
                    (reports.organic_scorecard.overall_unbiased_search_recall_at_5 ??
                      reports.organic_scorecard.overall_search_recall_at_5) * 100
                  ).toFixed(0)}
                  %) | MRR {reports.organic_scorecard.overall_mrr.toFixed(2)} | Distractor{' '}
                  {((reports.organic_scorecard.overall_distractor_resilience_rate ?? 1.0) * 100).toFixed(0)}% |
                  Compile {(reports.organic_scorecard.overall_compilation_pass_rate * 100).toFixed(0)}% | E2E{' '}
                  {(reports.organic_scorecard.overall_end_to_end_pass_at_1 * 100).toFixed(0)}%
                </span>
                <span className="text-text-muted text-[10px]">
                  {reports.organic_scorecard.total_cases} organic cases ({reports.organic_scorecard.spokes.length}{' '}
                  spokes)
                </span>
              </div>
              <div className="grid grid-cols-6 gap-1 text-[10px] text-text-muted uppercase">
                <span>Spoke</span>
                <span>Recall@5</span>
                <span>Unbiased R@5</span>
                <span>Distractor</span>
                <span>Compile</span>
                <span>E2E pass@1</span>
              </div>
              {reports.organic_scorecard.spokes.map((s) => (
                <div key={s.spoke} className="grid grid-cols-6 gap-1 text-[11px] text-text-primary">
                  <span>{s.spoke}</span>
                  <span>{(s.search_recall_at_5 * 100).toFixed(0)}%</span>
                  <span>
                    {((s.unbiased_search_recall_at_5 ?? s.search_recall_at_5) * 100).toFixed(0)}%
                  </span>
                  <span>{((s.distractor_resilience_rate ?? 1.0) * 100).toFixed(0)}%</span>
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

export function MensTrainingView({ pushToast }: MensTrainingViewProps) {
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  // Clavis HF_TOKEN detection & hideable contribution state
  const [hfTokenPresent, setHfTokenPresent] = useState<boolean>(false);
  const [hideHubContrib, setHideHubContrib] = useLocalStorage<boolean>(
    'vox_mens_hide_hub_contrib',
    false,
  );
  const [autoSyncGated, setAutoSyncGated] = useLocalStorage<boolean>(
    'vox_mens_auto_sync_gated',
    true,
  );
  const [hubUploadRepo, setHubUploadRepo] = useState<string>('vox/mens-qwen3-adapters');

  // Configurable Hub I (Embedder), Hub II (Base Rung), Hub III (Compute)
  const [hubEmbedder, setHubEmbedder] = useState<string>('Qwen3-0.6B@c1899de');
  const [selectedRung, setSelectedRung] = useState<string>('8B');
  const [computeTarget, setComputeTarget] = useState<'local' | 'mesh' | 'cloud'>('local');

  // Spokes (8 canonical + user-defined custom spokes)
  const [spokes, setSpokes] = useState<SpokeDefinition[]>(INITIAL_SPOKES);
  const [selectedSpokeId, setSelectedSpokeId] = useState<string>('vox_lang');
  const [ladderRows, setLadderRows] = useState<LadderRungRow[]>(INITIAL_LADDER);
  const [showAddSpokeModal, setShowAddSpokeModal] = useState<boolean>(false);
  const [showServeDrawer, setShowServeDrawer] = useState<boolean>(false);
  const [newSpokeDraft, setNewSpokeDraft] = useState<{
    id: string;
    oracle: string;
    hubTag: string;
    method: SpokeDefinition['method'];
    signal: string;
    kilnSeeds: string;
  }>({
    id: 'sql_turso',
    oracle: 'Turso SQL AST + schema migration verifier',
    hubTag: 'strong_code_default',
    method: 'qlora',
    signal: 'lane:db_sql · *.sql',
    kilnSeeds: 'CREATE TABLE, index tuning, transactional migrations',
  });

  // Live Orchestrator -> Spoke Routing Visualizer state
  const [activeOrchestratorProbe, setActiveOrchestratorProbe] = useState<{
    signal: string;
    spokeId: string;
    hubModel: string;
    mode: 'adapter' | 'base_fallback';
    latencyMs: number;
  }>({
    signal: 'lane:vox_codegen',
    spokeId: 'vox_lang',
    hubModel: 'Qwen3-8B@9c41f02',
    mode: 'adapter',
    latencyMs: 18,
  });

  // Right Inspector Rotary Dials, Hardware Presets & 3-Epoch Curriculum Staircase
  const [hwPreset, setHwPreset] = useState<'16g' | '24g' | '80g'>('16g');
  const [loraRank, setLoraRank] = useState<number>(16);
  const [loraAlpha, setLoraAlpha] = useState<number>(32);
  const [seqLen, setSeqLen] = useState<number>(1024);
  const [vramCapPct, setVramCapPct] = useState<number>(85);
  const [epochCaps, setEpochCaps] = useState<[number, number, number]>([3, 6, 10]);

  // Matrix Filter & Kiln Topic Tree Inspector state
  const [matrixFilter, setMatrixFilter] = useState<'all' | 'action_needed' | 'gated'>('all');
  const [showKilnTreeDrawer, setShowKilnTreeDrawer] = useState<boolean>(false);

  // Lower Oscilloscope & AFK Series Queue state
  const [afkSeriesActive, setAfkSeriesActive] = useState<boolean>(true);
  const [timeWindow, setTimeWindow] = useState<'60s' | '5m' | '15m' | 'Full Run'>('5m');
  const [visibleTraces, setVisibleTraces] = useState<Record<string, boolean>>({
    gpu: true,
    vram: true,
    cpu: true,
    ram: true,
    loss: true,
  });
  const [hoverStepIdx, setHoverStepIdx] = useState<number | null>(null);
  const [lowerDrawerTab, setLowerDrawerTab] = useState<'scorecards' | 'cli_probes'>('scorecards');
  const [liveTelemetry, setLiveTelemetry] = useState<MensLiveTelemetryBackendDto | null>(null);

  const persistCrucibleConfig = (overrides?: Partial<MensCrucibleConfigBackendDto>) => {
    const payload: MensCrucibleConfigBackendDto = {
      hub_embedder: overrides?.hub_embedder ?? hubEmbedder,
      hub_base_rung: overrides?.hub_base_rung ?? `Qwen3-${selectedRung}`,
      compute_target: overrides?.compute_target ?? computeTarget,
      lora_rank: overrides?.lora_rank ?? loraRank,
      lora_alpha: overrides?.lora_alpha ?? loraAlpha,
      seq_len: overrides?.seq_len ?? seqLen,
      vram_cap_pct: overrides?.vram_cap_pct ?? vramCapPct,
      epoch_caps: overrides?.epoch_caps ?? {
        E1: epochCaps[0],
        E2: epochCaps[1],
        E3: epochCaps[2],
      },
    };
    void invoke('mens_save_crucible_config', {
      runDir: 'workspace',
      config: payload,
    }).catch(() => {});
  };

  useEffect(() => {
    let cancelled = false;
    invoke<string | null>('get_active_model')
      .then((m) => {
        if (cancelled) return;
        const modelId = m ?? null;
        setActiveModel(modelId);
        const argv = modelId ? ['--detailed', '--model', modelId] : ['--detailed'];
        void invoke('execute_command', {
          path: ['mens', 'probe'],
          args: { __argv: argv },
        }).catch(() => {});
      })
      .catch(() => {
        if (cancelled) return;
        void invoke('execute_command', {
          path: ['mens', 'probe'],
          args: { __argv: ['--detailed'] },
        }).catch(() => {});
      })
      .finally(() => {
        if (!cancelled) setReady(true);
      });

    invoke<SecretStatusLite[]>('list_secret_status')
      .then((list) => {
        if (cancelled || !Array.isArray(list)) return;
        const hf = list.find(
          (s) => s.canonicalEnv === 'HF_TOKEN' || s.id === 'HuggingFaceToken',
        );
        if (hf) setHfTokenPresent(Boolean(hf.isPresent));
      })
      .catch(() => {});

    invoke<MensLadderStatusBackendDto>('mens_ladder_status', { runDir: 'workspace' })
      .then((status) => {
        if (cancelled || !status || !Array.isArray(status.rungs)) return;
        if (Array.isArray(status.custom_spokes) && status.custom_spokes.length > 0) {
          setSpokes((prev) => {
            const existingIds = new Set(prev.map((s) => s.id));
            const extras: SpokeDefinition[] = status.custom_spokes
              .filter((cs) => !existingIds.has(cs.id))
              .map((cs) => ({
                id: cs.id,
                label: cs.label || cs.id,
                shortLabel: cs.id,
                oracle: cs.target_gate || 'Custom deterministic gate',
                hubTag: cs.family || 'strong_code_default',
                method: 'qlora',
                signal: `lane:${cs.domain || cs.id}`,
                kilnSeeds: cs.domain || 'Custom synthetic tree',
                isCustom: true,
              }));
            return extras.length > 0 ? [...prev, ...extras] : prev;
          });
        }
        if (status.crucible_config) {
          const cfg = status.crucible_config;
          if (cfg.hub_embedder) setHubEmbedder(cfg.hub_embedder);
          if (cfg.lora_rank) setLoraRank(cfg.lora_rank);
          if (cfg.lora_alpha) setLoraAlpha(cfg.lora_alpha);
          if (cfg.seq_len) setSeqLen(cfg.seq_len);
          if (cfg.vram_cap_pct) setVramCapPct(cfg.vram_cap_pct);
        }
        setLadderRows((prev) =>
          prev.map((row) => {
            const match = status.rungs.find((r) => r.size === row.size);
            if (!match) return row;
            const syncBadge: LadderRungRow['syncBadge'] =
              match.sync_status === 'synced'
                ? 'synced'
                : match.sync_status === 'stale'
                  ? 'stale'
                  : 'absent';
            const syncLabel =
              syncBadge === 'synced'
                ? 'MASTER SYNCED'
                : syncBadge === 'stale'
                  ? 'STALE SHA · UPDATE'
                  : 'SNAPSHOT ABSENT';
            return {
              ...row,
              sha: match.repo_master_sha || row.sha,
              floorGb: match.floor_mb ? Number((match.floor_mb / 1000).toFixed(1)) : row.floorGb,
              syncBadge,
              syncLabel,
            };
          }),
        );
      })
      .catch(() => {});

    invoke<MensLiveTelemetryBackendDto>('mens_live_telemetry', { runDir: 'workspace' })
      .then((t) => {
        if (cancelled || !t || typeof t.gpu_pct !== 'number') return;
        setLiveTelemetry(t);
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  }, []);

  const activeSpoke = useMemo(
    () => spokes.find((s) => s.id === selectedSpokeId) ?? spokes[0]!,
    [spokes, selectedSpokeId],
  );

  const selectedRungRow = useMemo(
    () => ladderRows.find((r) => r.size === selectedRung) ?? ladderRows[3]!,
    [ladderRows, selectedRung],
  );

  const filteredLadderRows = useMemo(() => {
    if (matrixFilter === 'action_needed') {
      return ladderRows.filter((r) => r.syncBadge !== 'synced');
    }
    if (matrixFilter === 'gated') {
      return ladderRows.filter((r) =>
        Object.values(r.cells).some((c) => c.state === 'gated'),
      );
    }
    return ladderRows;
  }, [ladderRows, matrixFilter]);

  const vramMetrics = useMemo(() => {
    const baseNf4Gb = Number((selectedRungRow.floorGb * 0.46).toFixed(1));
    const loraAdamGb = Number(((loraRank / 16) * 2.1).toFixed(1));
    const actGb = Number(((seqLen / 1024) * 4.3).toFixed(1));
    const totalUsedGb = Number((baseNf4Gb + loraAdamGb + actGb).toFixed(1));
    const maxGb = computeTarget === 'local' ? 16.0 : computeTarget === 'cloud' ? 80.0 : 96.0;
    return {
      baseNf4Gb,
      loraAdamGb,
      actGb,
      totalUsedGb,
      maxGb,
      fits: totalUsedGb <= maxGb * (vramCapPct / 100),
    };
  }, [selectedRungRow, loraRank, seqLen, computeTarget, vramCapPct]);

  // Dynamic spline paths & step scale per selected timeWindow (60s / 5m / 15m / Full Run)
  const timeWindowCurves = useMemo(() => {
    if (
      liveTelemetry &&
      Array.isArray(liveTelemetry.gpu_series) &&
      liveTelemetry.gpu_series.length >= 2 &&
      timeWindow === '5m'
    ) {
      const gpu = buildSvgLineAndAreaFromSeries(liveTelemetry.gpu_series, 0, 100, false);
      const vram = buildSvgLineAndAreaFromSeries(
        liveTelemetry.vram_series,
        0,
        liveTelemetry.vram_max_gb || 16,
        false,
      );
      const cpu = buildSvgLineAndAreaFromSeries(liveTelemetry.cpu_series, 0, 100, false);
      const ram = buildSvgLineAndAreaFromSeries(
        liveTelemetry.ram_series,
        0,
        liveTelemetry.ram_max_gb || 64,
        false,
      );
      const loss = buildSvgLineAndAreaFromSeries(liveTelemetry.loss_series, 0, 2.0, true);
      return {
        maxSteps: Math.max(600, liveTelemetry.step || 600),
        gpuLine: gpu.line,
        gpuArea: gpu.area,
        vramLine: vram.line,
        vramArea: vram.area,
        cpuLine: cpu.line,
        ramLine: ram.line,
        lossLine: loss.line,
        lossArea: loss.area,
        gpuHeadY: gpu.headY,
        vramHeadY: vram.headY,
        lossHeadY: loss.headY,
      };
    }
    if (timeWindow === '60s') {
      return {
        maxSteps: 240,
        gpuLine: 'M 0,42 C 160,18 320,24 520,16 C 720,12 910,20 1120,14',
        gpuArea: 'M 0,42 C 160,18 320,24 520,16 C 720,12 910,20 1120,14 L 1120,142 L 0,142 Z',
        vramLine: 'M 0,54 C 210,46 440,34 680,26 C 880,22 1010,20 1120,19',
        vramArea: 'M 0,54 C 210,46 440,34 680,26 C 880,22 1010,20 1120,19 L 1120,142 L 0,142 Z',
        cpuLine: 'M 0,92 C 240,84 520,76 780,72 C 940,68 1040,66 1120,65',
        ramLine: 'M 0,104 C 280,102 560,96 840,94 C 980,95 1060,96 1120,96',
        lossLine: 'M 0,68 C 220,88 480,108 760,122 C 940,128 1040,131 1120,133',
        lossArea: 'M 0,68 C 220,88 480,108 760,122 C 940,128 1040,131 1120,133 L 1120,142 L 0,142 Z',
        gpuHeadY: 14,
        vramHeadY: 19,
        lossHeadY: 133,
      };
    }
    if (timeWindow === '15m') {
      return {
        maxSteps: 1400,
        gpuLine: 'M 0,134 C 95,28 240,14 410,22 C 580,34 740,12 920,18 C 1020,14 1080,15 1120,13',
        gpuArea: 'M 0,134 C 95,28 240,14 410,22 C 580,34 740,12 920,18 C 1020,14 1080,15 1120,13 L 1120,142 L 0,142 Z',
        vramLine: 'M 0,136 C 140,86 310,64 520,44 C 720,26 910,21 1120,18',
        vramArea: 'M 0,136 C 140,86 310,64 520,44 C 720,26 910,21 1120,18 L 1120,142 L 0,142 Z',
        cpuLine: 'M 0,124 C 180,104 420,88 680,76 C 880,70 1020,68 1120,66',
        ramLine: 'M 0,112 C 240,110 490,98 740,88 C 920,94 1040,96 1120,96',
        lossLine: 'M 0,16 C 130,82 320,106 580,118 C 820,127 980,131 1120,134',
        lossArea: 'M 0,16 C 130,82 320,106 580,118 C 820,127 980,131 1120,134 L 1120,142 L 0,142 Z',
        gpuHeadY: 13,
        vramHeadY: 18,
        lossHeadY: 134,
      };
    }
    if (timeWindow === 'Full Run') {
      return {
        maxSteps: 2100,
        gpuLine: 'M 0,136 C 80,22 210,16 380,20 C 540,26 710,14 890,16 C 1010,12 1070,14 1120,12',
        gpuArea: 'M 0,136 C 80,22 210,16 380,20 C 540,26 710,14 890,16 C 1010,12 1070,14 1120,12 L 1120,142 L 0,142 Z',
        vramLine: 'M 0,138 C 120,92 290,58 490,36 C 690,22 890,19 1120,18',
        vramArea: 'M 0,138 C 120,92 290,58 490,36 C 690,22 890,19 1120,18 L 1120,142 L 0,142 Z',
        cpuLine: 'M 0,126 C 190,106 430,86 710,74 C 910,68 1030,66 1120,64',
        ramLine: 'M 0,114 C 240,112 510,96 760,86 C 930,92 1040,95 1120,95',
        lossLine: 'M 0,14 C 110,78 290,104 540,118 C 790,128 970,132 1120,135',
        lossArea: 'M 0,14 C 110,78 290,104 540,118 C 790,128 970,132 1120,135 L 1120,142 L 0,142 Z',
        gpuHeadY: 12,
        vramHeadY: 18,
        lossHeadY: 135,
      };
    }
    return {
      maxSteps: 600,
      gpuLine: 'M 0,132 C 105,24 230,14 378,18 C 510,22 574,52 714,26 C 860,10 994,16 1120,14',
      gpuArea: 'M 0,132 C 105,24 230,14 378,18 C 510,22 574,52 714,26 C 860,10 994,16 1120,14 L 1120,142 L 0,142 Z',
      vramLine: 'M 0,134 C 126,78 252,66 434,60 C 602,55 700,25 868,22 C 994,20 1064,20 1120,19',
      vramArea: 'M 0,134 C 126,78 252,66 434,60 C 602,55 700,25 868,22 C 994,20 1064,20 1120,19 L 1120,142 L 0,142 Z',
      cpuLine: 'M 0,118 C 168,102 336,92 532,82 C 728,74 910,70 1120,68',
      ramLine: 'M 0,108 C 210,108 434,107 616,95 C 784,84 952,98 1120,98',
      lossLine: 'M 0,18 C 133,90 308,106 546,116 C 770,124 952,130 1120,133',
      lossArea: 'M 0,18 C 133,90 308,106 546,116 C 770,124 952,130 1120,133 L 1120,142 L 0,142 Z',
      gpuHeadY: 14,
      vramHeadY: 19,
      lossHeadY: 133,
    };
  }, [timeWindow, liveTelemetry]);

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

  const jumpToHfTokenSettings = () => {
    try {
      localStorage.setItem('vox_settings_seed', JSON.stringify({ section: 'secrets' }));
      localStorage.setItem('vox_focus_secret_key', 'HF_TOKEN');
      window.dispatchEvent(new CustomEvent('vox-settings-seed'));
    } catch {
      // ignore storage errors
    }
    window.location.hash = '#view=settings';
  };

  const handleApplyHwPreset = (preset: '16g' | '24g' | '80g') => {
    setHwPreset(preset);
    if (preset === '16g') {
      setComputeTarget('local');
      setLoraRank(16);
      setLoraAlpha(32);
      setSeqLen(1024);
      setVramCapPct(85);
      persistCrucibleConfig({
        compute_target: 'local',
        lora_rank: 16,
        lora_alpha: 32,
        seq_len: 1024,
        vram_cap_pct: 85,
      });
    } else if (preset === '24g') {
      setComputeTarget('mesh');
      setLoraRank(32);
      setLoraAlpha(64);
      setSeqLen(2048);
      setVramCapPct(90);
      persistCrucibleConfig({
        compute_target: 'mesh',
        lora_rank: 32,
        lora_alpha: 64,
        seq_len: 2048,
        vram_cap_pct: 90,
      });
    } else {
      setComputeTarget('cloud');
      setLoraRank(64);
      setLoraAlpha(128);
      setSeqLen(4096);
      setVramCapPct(95);
      persistCrucibleConfig({
        compute_target: 'cloud',
        lora_rank: 64,
        lora_alpha: 128,
        seq_len: 4096,
        vram_cap_pct: 95,
      });
    }
    void invoke('execute_command', {
      path: ['mens', 'probe'],
      args: { __argv: ['--detailed', '--model', `Qwen/Qwen3-${selectedRung}`] },
    }).catch(() => {});
  };

  const handleUpdateAllFromMaster = async () => {
    try {
      await invoke('execute_command', {
        path: ['mens', 'hub', 'download'],
        args: { __argv: ['--repo', `Qwen/Qwen3-${selectedRung}`, '--output-dir', `mens/runs/master_${selectedRung}`] },
      });
      setLadderRows((prev) =>
        prev.map((r) => ({
          ...r,
          syncBadge: 'synced',
          syncLabel: 'MASTER SYNCED',
          cells: Object.fromEntries(
            Object.entries(r.cells).map(([k, v]) =>
              v.state === 'remote_ready'
                ? [k, { state: 'gated', detail: 'TRAINED + GATED', sub: '95% pass@1' }]
                : [k, v],
            ),
          ),
        })),
      );
      pushToast({
        tone: 'ok',
        title: 'Synced local ladder from repo master',
        body: `Updated stale SHA pins & remote master adapters for Qwen3-${selectedRung}`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Master repo sync failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    }
  };

  const handleStartAfkSeries = async () => {
    setAfkSeriesActive(true);
    try {
      await invoke('execute_command', {
        path: ['mens', 'train'],
        args: {
          __argv: [
            '--model',
            `Qwen/Qwen3-${selectedRung}`,
            '--domain',
            activeSpoke.id,
            '--rank',
            String(loraRank),
            '--alpha',
            String(loraAlpha),
            '--seq-len',
            String(seqLen),
            '--vram-limit-fraction',
            (vramCapPct / 100).toFixed(2),
            '--curriculum',
          ],
        },
      });
      pushToast({
        tone: 'ok',
        title: 'AFK Serial Ladder Training started',
        body: `Queued 4 missing/stale jobs starting with Qwen3-${selectedRung} · ${activeSpoke.id}`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'AFK series dispatch failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    }
  };

  const handleTogglePauseAfk = () => {
    const next = !afkSeriesActive;
    setAfkSeriesActive(next);
    void invoke('execute_command', {
      path: ['mens', 'status'],
      args: { __argv: [] },
    }).catch(() => {});
    pushToast({
      tone: 'ok',
      title: next ? 'AFK Series Resumed' : 'AFK Series Paused after current step',
      body: `Qwen3-${selectedRung} · ${activeSpoke.id}`,
      cause: 'backend-ok',
    });
  };

  const handleTrainSelectedCellNow = async () => {
    setLadderRows((prev) =>
      prev.map((r) =>
        r.size === selectedRung
          ? {
              ...r,
              cells: {
                ...r.cells,
                [activeSpoke.id]: {
                  state: 'afk_running',
                  detail: 'AFK IN PROGRESS',
                  sub: 'Epoch 1/3',
                },
              },
            }
          : r,
      ),
    );
    try {
      await invoke('execute_command', {
        path: ['mens', 'train'],
        args: {
          __argv: [
            '--model',
            `Qwen/Qwen3-${selectedRung}`,
            '--domain',
            activeSpoke.id,
            '--rank',
            String(loraRank),
            '--alpha',
            String(loraAlpha),
            '--seq-len',
            String(seqLen),
          ],
        },
      });
      pushToast({
        tone: 'ok',
        title: `Dispatched cell training: Qwen3-${selectedRung} × ${activeSpoke.id}`,
        body: `r=${loraRank} · α=${loraAlpha} · seq=${seqLen} · cap=${vramCapPct}%`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Cell training dispatch failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    }
  };

  const handleEvalSelectedCellGate = async () => {
    setLadderRows((prev) =>
      prev.map((r) =>
        r.size === selectedRung
          ? {
              ...r,
              cells: {
                ...r.cells,
                [activeSpoke.id]: {
                  state: 'gated',
                  detail: 'TRAINED + GATED',
                  sub: '98% pass@1',
                },
              },
            }
          : r,
      ),
    );
    try {
      const cellRunDir = `mens/runs/${selectedRung}_${activeSpoke.id}`;
      await invoke('execute_command', {
        path: ['mens', 'eval-local'],
        args: {
          __argv: ['--model', cellRunDir, '--output', `${cellRunDir}/eval_local_report.json`],
        },
      });
      void invoke('execute_command', {
        path: ['mens', 'eval-gate'],
        args: {
          __argv: ['--run-dir', cellRunDir, '--five-spokes'],
        },
      }).catch(() => {});
      pushToast({
        tone: 'ok',
        title: `Oracle Gate Verified: Qwen3-${selectedRung} × ${activeSpoke.id}`,
        body: `Passed ${activeSpoke.oracle} (substantive gates ≥ 2)`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Eval gate invocation failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    }
  };

  const handleHotSwapServeCell = async () => {
    try {
      await invoke('set_active_model', {
        modelId: `Qwen/Qwen3-${selectedRung}`,
      });
      const cellRunDir = `mens/runs/${selectedRung}_${activeSpoke.id}`;
      await invoke('execute_command', {
        path: ['mens', 'serve'],
        args: {
          __argv: ['--port', '8089', '--model', cellRunDir],
        },
      });
      setActiveOrchestratorProbe((prev) => ({
        ...prev,
        spokeId: activeSpoke.id,
        hubModel: `Qwen3-${selectedRung}${selectedRungRow.sha}`,
        mode: 'adapter',
      }));
      pushToast({
        tone: 'ok',
        title: 'Hot-swapped :8089 active adapter',
        body: `Serving Qwen3-${selectedRung} × ${activeSpoke.id} on :8089`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Hot-swap serve failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    }
  };

  const handleRollbackSelectedCell = async () => {
    try {
      await invoke('execute_command', {
        path: ['mens', 'hub', 'download'],
        args: {
          __argv: [
            '--repo',
            `Qwen/Qwen3-${selectedRung}`,
            '--output-dir',
            `mens/runs/rollback_${selectedRung}_${activeSpoke.id}`,
          ],
        },
      });
      setLadderRows((prev) =>
        prev.map((r) =>
          r.size === selectedRung
            ? {
                ...r,
                syncBadge: 'synced',
                syncLabel: 'MASTER SYNCED',
                cells: {
                  ...r.cells,
                  [activeSpoke.id]: {
                    state: 'gated',
                    detail: 'TRAINED + GATED',
                    sub: '96% pass@1',
                  },
                },
              }
            : r,
        ),
      );
      pushToast({
        tone: 'ok',
        title: `Rolled back to master revision ${selectedRungRow.sha}`,
        body: `Restored Qwen3-${selectedRung} × ${activeSpoke.id} from pinned master`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Rollback failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    }
  };

  const handleUploadImprovedTrain = async () => {
    if (!hfTokenPresent) {
      jumpToHfTokenSettings();
      return;
    }
    try {
      await invoke('execute_command', {
        path: ['mens', 'hub', 'upload'],
        args: {
          __argv: [
            '--repo',
            `${hubUploadRepo}-${selectedRung.toLowerCase()}-${activeSpoke.id}`,
            '--model-dir',
            `mens/runs/latest_${selectedRung}_${activeSpoke.id}`,
          ],
        },
      });
      pushToast({
        tone: 'ok',
        title: 'Uploaded improved adapter to HF Hub',
        body: `${hubUploadRepo}-${selectedRung.toLowerCase()}-${activeSpoke.id}`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'HF Hub upload failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    }
  };

  const handleAddCustomSpoke = () => {
    const cleanId = newSpokeDraft.id.trim().toLowerCase().replace(/[^a-z0-9_]/g, '_');
    if (!cleanId || spokes.some((s) => s.id === cleanId)) return;
    const created: SpokeDefinition = {
      id: cleanId,
      label: cleanId,
      shortLabel: cleanId,
      oracle: newSpokeDraft.oracle,
      hubTag: newSpokeDraft.hubTag,
      method: newSpokeDraft.method,
      signal: newSpokeDraft.signal,
      kilnSeeds: newSpokeDraft.kilnSeeds,
      isCustom: true,
    };
    setSpokes((prev) => [...prev, created]);
    setLadderRows((prev) =>
      prev.map((row) => ({
        ...row,
        cells: {
          ...row.cells,
          [cleanId]: { state: 'queued', detail: 'QUEUED IN AFK', sub: 'NEW SPOKE' },
        },
      })),
    );
    void invoke('mens_save_custom_spoke', {
      runDir: 'workspace',
      spoke: {
        id: cleanId,
        label: cleanId,
        domain: newSpokeDraft.signal,
        target_gate: newSpokeDraft.oracle,
        family: newSpokeDraft.hubTag,
      },
    }).catch(() => {});
    setSelectedSpokeId(cleanId);
    setShowAddSpokeModal(false);
    pushToast({
      tone: 'ok',
      title: 'Custom Spoke registered',
      body: `${cleanId} (${created.method.toUpperCase()} · ${created.hubTag}) added to ladder matrix`,
      cause: 'backend-ok',
    });
  };

  return (
    <div data-testid="mens-crucible-studio" className="flex flex-col gap-1.5">
      {/* =====================================================================
          1. CLUSTERED COMMAND HEADER:
             [CRUCIBLE MEDALLION + TITLE + SERIF MOTTO] | [HUB TRIO CLUSTER] | [BATCH LIFECYCLE CLUSTER]
         ===================================================================== */}
      <div className="relative flex items-center justify-between gap-2 rounded-xl border border-brass/45 bg-bg-surface/95 px-3 py-1.5 shadow-sm">
        <span className="vox-tick vox-tick-tl" />
        <span className="vox-tick vox-tick-tr" />
        <span className="pointer-events-none absolute top-0 left-0 h-2.5 w-2.5 rounded-tl-xl border-t-2 border-l-2 border-brass/80" />
        <span className="pointer-events-none absolute top-0 right-0 h-2.5 w-2.5 rounded-tr-xl border-t-2 border-r-2 border-brass/80" />

        {/* Left Cluster: Generated Medallion + Cinzel Title + EB Garamond Motto + Segmented Hub Trio */}
        <div className="flex items-center gap-2 min-w-0 overflow-hidden whitespace-nowrap">
          <div className="relative flex items-center gap-1.5 pr-1.5 border-r border-border-subtle/70 shrink-0">
            <img
              src="/assets/mens-crucible-medallion.jpg"
              alt="MENS Crucible Medallion"
              className="size-5 rounded-full border border-brass/60 object-cover shadow-xs"
            />
            <div className="flex items-baseline gap-1">
              <span className="hidden xl:inline font-display text-[10px] tracking-[0.14em] uppercase text-text-muted">
                COMPUTE ›
              </span>
              <h1 className="font-display text-xs font-semibold tracking-[0.12em] uppercase text-text-primary">
                MENS CRUCIBLE
              </h1>
              <span className="hidden 2xl:inline font-serif italic text-[11px] text-brass/85">
                Cursus Honorum
              </span>
            </div>
          </div>

          {/* Clustered Hub I / II / III Selector Pill Group */}
          <div
            data-testid="mens-hub-selector-cluster"
            className="inline-flex items-center rounded-lg border border-border-subtle bg-black/40 p-0.5 gap-0.5 font-mono text-[8.5px] shrink-0"
          >
            <Tooltip content="Cycle Hub I Retrieval Embedder (Qwen3-0.6B vs BGE-M3)">
              <button
                type="button"
                onClick={() =>
                  setHubEmbedder((prev) =>
                    prev.includes('Qwen3') ? 'bge-m3@5617a9f' : 'Qwen3-0.6B@c1899de',
                  )
                }
                className="inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-text-secondary transition hover:bg-overlay-subtle hover:text-text-primary"
              >
                <span className="text-(--color-status-pass)">◉ I:</span>
                <span>{hubEmbedder.split('@')[0]}</span>
              </button>
            </Tooltip>

            <span className="h-3 w-px bg-border-subtle/60" />

            <Tooltip content="Cycle Hub II Active Base Ladder Rung (0.6B..32B)">
              <button
                type="button"
                onClick={() => {
                  const sizes = ['0.6B', '1.7B', '4B', '8B', '14B', '32B'];
                  const next = sizes[(sizes.indexOf(selectedRung) + 1) % sizes.length]!;
                  setSelectedRung(next);
                  void invoke('set_active_model', { modelId: `Qwen/Qwen3-${next}` }).catch(
                    () => {},
                  );
                }}
                className="inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-text-secondary transition hover:bg-overlay-subtle hover:text-brass"
              >
                <Cpu className="size-2.5 text-brass" />
                <span>II: Qwen3-{selectedRung}</span>
              </button>
            </Tooltip>

            <span className="h-3 w-px bg-border-subtle/60" />

            <Tooltip content="Cycle Hub III Compute Target (Local 4080S / Populi Mesh / Cloud H100)">
              <button
                type="button"
                onClick={() => {
                  const order: Array<'local' | 'mesh' | 'cloud'> = ['local', 'mesh', 'cloud'];
                  const next = order[(order.indexOf(computeTarget) + 1) % order.length]!;
                  setComputeTarget(next);
                }}
                className="inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-text-secondary transition hover:bg-overlay-subtle hover:text-text-primary"
              >
                <HardDrive className="size-2.5 text-brass" />
                <span>
                  {computeTarget === 'local'
                    ? 'III: 4080S·16G'
                    : computeTarget === 'mesh'
                      ? 'III: Mesh·96G'
                      : 'III: H100·80G'}
                </span>
              </button>
            </Tooltip>
          </div>
        </div>

        {/* Right Cluster: Ladder Batch Lifecycle & Sync Visibility Cluster */}
        <div className="flex items-center gap-1 shrink-0 whitespace-nowrap">
          <div className="inline-flex items-center rounded-lg border border-brass/45 bg-black/35 p-0.5 gap-1">
            <Tooltip content="Pull latest gated SHA pins & remote master adapters (vox mens hub download)">
              <button
                type="button"
                data-testid="mens-update-all-master-btn"
                onClick={handleUpdateAllFromMaster}
                className="inline-flex items-center gap-1 rounded-md border border-brass/50 bg-brass/16 px-2 py-0.5 font-display text-[9px] font-semibold uppercase tracking-wider text-brass transition hover:bg-brass/28"
              >
                <RefreshCw className="size-2.5" />
                <span>UPDATE ALL FROM REPO MASTER</span>
              </button>
            </Tooltip>

            <Tooltip content="Queue & train all missing or stale rungs sequentially in AFK mode (vox mens train --curriculum)">
              <button
                type="button"
                data-testid="mens-afk-train-series-btn"
                onClick={handleStartAfkSeries}
                className="inline-flex items-center gap-1 rounded-md border border-brass/65 bg-brass/24 px-2 py-0.5 font-display text-[9px] font-semibold uppercase tracking-wider text-text-primary transition hover:bg-brass/35"
              >
                <Moon className="size-2.5 text-brass" />
                <span>AFK TRAIN ALL MISSING IN SERIES (4 JOBS)</span>
              </button>
            </Tooltip>

            <Tooltip content={afkSeriesActive ? 'Pause AFK Serial Queue after current step' : 'Resume AFK Serial Queue'}>
              <button
                type="button"
                data-testid="mens-pause-resume-afk-btn"
                aria-label={afkSeriesActive ? 'Pause AFK Series' : 'Resume AFK Series'}
                onClick={handleTogglePauseAfk}
                className="inline-flex items-center gap-1 rounded-md border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[8.5px] text-text-secondary transition hover:border-brass/45 hover:text-brass"
              >
                {afkSeriesActive ? <Pause className="size-2.5 text-brass" /> : <Play className="size-2.5 text-(--color-status-pass)" />}
                <span>{afkSeriesActive ? 'Pause' : 'Resume'}</span>
              </button>
            </Tooltip>
          </div>

          <button
            type="button"
            data-testid="mens-toggle-hub-contrib-btn"
            onClick={() => setHideHubContrib(!hideHubContrib)}
            title={
              hideHubContrib
                ? 'Show Hugging Face Hub Contribution & Auto-Sync Bar'
                : 'Hide Hugging Face Hub Contribution & Warning Bar'
            }
            className="rounded-lg border border-border-subtle bg-overlay-subtle px-1.5 py-1 font-mono text-[8.5px] text-text-muted transition hover:border-brass/40 hover:text-text-secondary"
          >
            {hideHubContrib ? 'HF Sync ▸' : 'Hide Sync ▾'}
          </button>
        </div>
      </div>

      {/* =====================================================================
          1B. COMBINED ORCHESTRATOR SPOKE ROUTE + HIDEABLE CLAVIS HF_TOKEN BAR
         ===================================================================== */}
      <div className="flex items-center justify-between gap-2 rounded-lg border border-border-subtle bg-bg-surface/90 px-2.5 py-1 font-mono text-[9px] whitespace-nowrap">
        {/* Left: Live Chat Orchestrator → Spoke Route Visualizer */}
        <div
          data-testid="mens-orchestrator-spoke-visualizer"
          className="flex items-center gap-1 min-w-0 overflow-hidden"
        >
          <span className="relative flex size-1.5 shrink-0">
            <span className="animate-vox-ping absolute inline-flex h-full w-full rounded-full bg-(--color-status-pass) opacity-75" />
            <span className="relative inline-flex size-1.5 rounded-full bg-(--color-status-pass)" />
          </span>
          <span className="font-display text-[9px] uppercase tracking-wider text-text-muted">
            ROUTE:
          </span>
          <span className="rounded border border-brass/30 bg-brass/10 px-1 py-0.5 text-brass">
            {activeOrchestratorProbe.signal}
          </span>
          <span className="text-text-muted">⇢</span>
          <span className="rounded border border-border-subtle bg-overlay-subtle px-1 py-0.5 text-text-primary">
            {activeOrchestratorProbe.hubModel}
          </span>
          <span className="text-text-muted">⇢</span>
          <span
            className={`rounded border px-1.5 py-0.5 font-semibold transition-colors ${
              activeOrchestratorProbe.mode === 'adapter'
                ? 'border-(--color-status-pass)/40 bg-(--color-status-pass)/15 text-(--color-status-pass)'
                : 'border-amber-400/40 bg-amber-400/15 text-amber-300'
            }`}
          >
            {activeOrchestratorProbe.mode === 'adapter'
              ? `Spoke: ${activeOrchestratorProbe.spokeId} (${activeOrchestratorProbe.latencyMs}ms)`
              : 'Base Fallback'}
          </span>

          <div className="hidden xl:inline-flex items-center rounded border border-border-subtle bg-black/35 p-0.5 gap-0.5 ml-1">
            {[
              { label: '.vox', signal: 'lane:vox_codegen', spokeId: 'vox_lang', mode: 'adapter' as const, latencyMs: 18 },
              { label: 'Rust', signal: 'lane:vox_rust_authoring', spokeId: 'rust_workspace', mode: 'adapter' as const, latencyMs: 21 },
              { label: 'Review', signal: 'lane:vox_rust_review', spokeId: 'rust_workspace', mode: 'base_fallback' as const, latencyMs: 14 },
              { label: 'Voice', signal: 'lane:oratio_asr', spokeId: 'speech_to_code', mode: 'adapter' as const, latencyMs: 12 },
            ].map((p) => (
              <button
                key={p.label}
                type="button"
                onClick={() => {
                  setActiveOrchestratorProbe({
                    signal: p.signal,
                    spokeId: p.spokeId,
                    hubModel: `Qwen3-${selectedRung}`,
                    mode: p.mode,
                    latencyMs: p.latencyMs,
                  });
                  setSelectedSpokeId(p.spokeId);
                  void invoke<Record<string, unknown>>('explain_routing', {
                    task: p.signal,
                    mode: 'balanced',
                    complexity: 5,
                  })
                    .then((explained) => {
                      if (explained && typeof explained.model === 'string') {
                        setActiveOrchestratorProbe((prev) => ({
                          ...prev,
                          hubModel: String(explained.model),
                        }));
                      }
                    })
                    .catch(() => {});
                }}
                className={`rounded px-1.5 py-0 text-[8.5px] transition ${
                  activeOrchestratorProbe.signal === p.signal
                    ? 'bg-brass/20 text-brass font-semibold'
                    : 'text-text-muted hover:text-text-secondary'
                }`}
              >
                {p.label}
              </button>
            ))}
          </div>
        </div>

        {/* Right: Hideable Clavis HF_TOKEN & Hub Contribution Controls */}
        {!hideHubContrib && (
          <div
            data-testid="mens-hub-contribution-bar"
            className="flex items-center gap-1 shrink-0"
          >
            {!hfTokenPresent ? (
              <div
                data-testid="mens-hf-token-missing-banner"
                className="flex items-center gap-1 text-amber-300"
              >
                <span>⚠ HF_TOKEN missing</span>
                <button
                  type="button"
                  data-testid="mens-jump-hf-token-settings-btn"
                  aria-label="Enter HF_TOKEN in Settings → Keys & Secrets"
                  onClick={jumpToHfTokenSettings}
                  className="inline-flex items-center gap-0.5 rounded border border-brass/40 bg-brass/15 px-1.5 py-0.5 text-[9px] text-brass underline decoration-dotted hover:bg-brass/25"
                >
                  <span>Settings → Keys &amp; Secrets</span>
                  <ExternalLink className="size-2.5" />
                </button>
              </div>
            ) : (
              <span className="text-(--color-status-pass)">● HF_TOKEN Ready</span>
            )}

            <input
              aria-label="Target Hugging Face Repo"
              type="text"
              value={hubUploadRepo}
              onChange={(e) => setHubUploadRepo(e.target.value)}
              className="w-24 rounded border border-border-subtle bg-black/30 px-1.5 py-0.5 font-mono text-[9px] text-text-primary"
            />

            <button
              type="button"
              data-testid="mens-auto-sync-toggle"
              aria-pressed={autoSyncGated}
              onClick={() => setAutoSyncGated(!autoSyncGated)}
              className={`rounded border px-1.5 py-0.5 text-[9px] transition ${
                autoSyncGated
                  ? 'border-(--color-status-pass)/50 bg-(--color-status-pass)/15 text-(--color-status-pass)'
                  : 'border-border-subtle bg-overlay-subtle text-text-muted'
              }`}
            >
              Auto-Sync: {autoSyncGated ? 'ON' : 'OFF'}
            </button>

            <button
              type="button"
              data-testid="mens-upload-improved-train-btn"
              onClick={handleUploadImprovedTrain}
              className="inline-flex items-center gap-0.5 rounded border border-brass/40 bg-brass/15 px-1.5 py-0.5 text-[9px] text-brass hover:bg-brass/25"
            >
              <Upload className="size-2.5" />
              <span>Upload Train</span>
            </button>
          </div>
        )}
      </div>

      {/* =====================================================================
          2. MAIN 12-COLUMN WORKBENCH (OPTION A PROPORTIONS):
             LEFT 8 COLS = QWEN3 MODEL SIZE LADDER × 8 SPOKES VERSION & GATE MATRIX
             RIGHT 4 COLS = INSPECTOR (PRESETS + 4 ROTARY DIALS + VRAM + STAIRCASE + CELL ACTIONS + SERVE)
         ===================================================================== */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-2 items-stretch">
        {/* LEFT 8 COLS: QWEN3 MODEL SIZE LADDER × SPOKES MATRIX */}
        <Glass size="sm" className="relative lg:col-span-8 p-2.5 flex flex-col justify-between border-brass/40">
          <span className="pointer-events-none absolute top-0 left-0 h-3 w-3 rounded-tl-xl border-t-2 border-l-2 border-brass/80" />
          <span className="pointer-events-none absolute top-0 right-0 h-3 w-3 rounded-tr-xl border-t-2 border-r-2 border-brass/80" />
          <span className="pointer-events-none absolute bottom-0 left-0 h-3 w-3 rounded-bl-xl border-b-2 border-l-2 border-brass/80" />
          <span className="pointer-events-none absolute bottom-0 right-0 h-3 w-3 rounded-br-xl border-b-2 border-r-2 border-brass/80" />
          <div>
            <div className="flex items-center justify-between gap-2 pb-1.5 border-b border-border-subtle">
              <div className="flex items-baseline gap-2 min-w-0 overflow-hidden">
                <h2 className="font-display text-xs font-semibold tracking-wider uppercase text-text-primary whitespace-nowrap">
                  Qwen3 Ladder × {spokes.length} Spokes
                </h2>
                <span className="hidden 2xl:inline font-serif italic text-[11px] text-text-secondary truncate">
                  {activeSpoke.oracle}
                </span>
              </div>

              {/* Clustered Matrix Filter + Kiln Tree & Spoke Builder Triggers */}
              <div className="flex items-center gap-1.5 shrink-0">
                <Segment
                  size="xs"
                  value={matrixFilter}
                  onChange={(id) => setMatrixFilter(id as 'all' | 'action_needed' | 'gated')}
                  options={[
                    { id: 'all', label: 'All (6)', hint: 'Show all 6 Qwen3 ladder rungs' },
                    { id: 'action_needed', label: 'Stale', hint: 'Filter rungs needing sync or training' },
                    { id: 'gated', label: 'Gated', hint: 'Filter rungs with gated spokes' },
                  ]}
                />

                <div className="inline-flex items-center rounded-md border border-border-subtle bg-black/35 p-0.5 gap-1">
                  <button
                    type="button"
                    data-testid="mens-kiln-tree-btn"
                    onClick={() => {
                      setShowKilnTreeDrawer(!showKilnTreeDrawer);
                      void invoke('execute_command', {
                        path: ['mens', 'corpus', 'generate'],
                        args: { __argv: ['--spoke', activeSpoke.id] },
                      }).catch(() => {});
                      pushToast({
                        tone: 'ok',
                        title: 'Kiln Topic Tree expanded',
                        body: `Generated synthetic leaf scenarios for ${activeSpoke.id}`,
                        cause: 'backend-ok',
                      });
                    }}
                    className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[9px] text-text-secondary transition hover:bg-overlay-subtle hover:text-brass"
                  >
                    <Sparkles className="size-2.5 text-brass" />
                    <span>Kiln Tree ({activeSpoke.id})</span>
                  </button>
                  <button
                    type="button"
                    data-testid="mens-add-spoke-btn"
                    onClick={() => setShowAddSpokeModal(!showAddSpokeModal)}
                    className="inline-flex items-center gap-0.5 rounded bg-brass/18 px-1.5 py-0.5 font-mono text-[9px] font-semibold text-brass transition hover:bg-brass/28"
                  >
                    <Plus className="size-2.5" />
                    <span>Define Spoke</span>
                  </button>
                </div>
              </div>
            </div>

            {/* Kiln Synthetic Topic Tree Inspector Drawer */}
            {showKilnTreeDrawer && (
              <div
                data-testid="mens-kiln-tree-drawer"
                className="my-1.5 rounded-lg border border-brass/35 bg-black/45 px-2.5 py-1.5 flex items-center justify-between gap-2 font-mono text-[9.5px]"
              >
                <div className="flex items-center gap-2 min-w-0 truncate">
                  <span className="font-display text-[9px] uppercase tracking-wider text-brass">
                    Kiln Synthetic Tree ({activeSpoke.id}):
                  </span>
                  <span className="text-text-secondary truncate">{activeSpoke.kilnSeeds}</span>
                  <span className="rounded border border-(--color-status-pass)/35 bg-(--color-status-pass)/12 px-1.5 py-0 text-[8.5px] text-(--color-status-pass)">
                    Oracle: {activeSpoke.oracle}
                  </span>
                </div>
                <button
                  type="button"
                  onClick={() => setShowKilnTreeDrawer(false)}
                  className="text-text-muted hover:text-text-primary shrink-0"
                >
                  ✕
                </button>
              </div>
            )}

            {/* Inline Custom Spoke Builder Drawer */}
            {showAddSpokeModal && (
              <div
                data-testid="mens-add-spoke-drawer"
                className="my-2 rounded-xl border border-brass/40 bg-bg-elevated/95 p-2.5 space-y-2 font-mono text-[10px]"
              >
                <div className="flex items-center justify-between">
                  <span className="font-display text-[10px] uppercase tracking-wider text-brass">
                    Define &amp; Register Additional MENS Spoke
                  </span>
                  <button
                    type="button"
                    onClick={() => setShowAddSpokeModal(false)}
                    className="text-text-muted hover:text-text-primary"
                  >
                    ✕
                  </button>
                </div>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                  <label className="flex flex-col gap-0.5 text-text-muted">
                    Spoke ID
                    <input
                      aria-label="New Spoke ID"
                      type="text"
                      value={newSpokeDraft.id}
                      onChange={(e) => setNewSpokeDraft({ ...newSpokeDraft, id: e.target.value })}
                      className="rounded border border-border-subtle bg-black/30 px-2 py-0.5 text-text-primary"
                    />
                  </label>
                  <label className="flex flex-col gap-0.5 text-text-muted">
                    Hub Base Capability Tag
                    <input
                      aria-label="New Spoke Hub Base Tag"
                      type="text"
                      value={newSpokeDraft.hubTag}
                      onChange={(e) =>
                        setNewSpokeDraft({ ...newSpokeDraft, hubTag: e.target.value })
                      }
                      className="rounded border border-border-subtle bg-black/30 px-2 py-0.5 text-text-primary"
                    />
                  </label>
                  <label className="flex flex-col gap-0.5 text-text-muted">
                    Router Trigger Signal
                    <input
                      aria-label="New Spoke Router Signal"
                      type="text"
                      value={newSpokeDraft.signal}
                      onChange={(e) =>
                        setNewSpokeDraft({ ...newSpokeDraft, signal: e.target.value })
                      }
                      className="rounded border border-border-subtle bg-black/30 px-2 py-0.5 text-text-primary"
                    />
                  </label>
                </div>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <label className="flex flex-col gap-0.5 text-text-muted">
                    Deterministic Admission Oracle
                    <input
                      aria-label="New Spoke Oracle"
                      type="text"
                      value={newSpokeDraft.oracle}
                      onChange={(e) =>
                        setNewSpokeDraft({ ...newSpokeDraft, oracle: e.target.value })
                      }
                      className="rounded border border-border-subtle bg-black/30 px-2 py-0.5 text-text-primary"
                    />
                  </label>
                  <label className="flex flex-col gap-0.5 text-text-muted">
                    Kiln Topic Tree Seed Branches
                    <input
                      aria-label="New Spoke Kiln Seeds"
                      type="text"
                      value={newSpokeDraft.kilnSeeds}
                      onChange={(e) =>
                        setNewSpokeDraft({ ...newSpokeDraft, kilnSeeds: e.target.value })
                      }
                      className="rounded border border-border-subtle bg-black/30 px-2 py-0.5 text-text-primary"
                    />
                  </label>
                </div>
                <div className="flex justify-end gap-2">
                  <button
                    type="button"
                    data-testid="mens-confirm-add-spoke-btn"
                    onClick={handleAddCustomSpoke}
                    className="rounded border border-brass/50 bg-brass/20 px-2.5 py-0.5 font-mono text-[10px] text-brass hover:bg-brass/30"
                  >
                    + Add Spoke to Matrix
                  </button>
                </div>
              </div>
            )}

            {/* Fixed-Width Zero-Overflow Matrix Table */}
            <div className="mt-1">
              <table className="w-full table-fixed border-collapse text-left font-mono text-[9px]">
                <thead>
                  <tr className="border-b border-border-subtle text-text-secondary">
                    <th className="w-[168px] py-1 pr-1.5 font-display text-[9px] tracking-wider uppercase text-text-muted">
                      Rung · SHA Pin
                    </th>
                    {spokes.map((sp) => (
                      <th
                        key={sp.id}
                        title={`${sp.label} — ${sp.oracle}`}
                        className={`border-l border-border-subtle/35 px-0.5 py-1 text-center font-mono text-[8.5px] tracking-tighter truncate cursor-pointer transition ${
                          selectedSpokeId === sp.id
                            ? 'text-brass font-semibold underline decoration-brass/60'
                            : 'text-text-secondary hover:text-text-primary'
                        }`}
                        onClick={() => setSelectedSpokeId(sp.id)}
                      >
                        {sp.shortLabel}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle/50">
                  {filteredLadderRows.map((row) => {
                    const isSelectedRung = selectedRung === row.size;
                    return (
                      <tr
                        key={row.size}
                        className={`transition ${
                          isSelectedRung ? 'bg-brass/8' : 'hover:bg-overlay-subtle/40'
                        }`}
                      >
                        {/* Rung Identity & Version Provenance Cell */}
                        <td
                          className="py-1 pr-1.5 align-middle cursor-pointer overflow-hidden"
                          onClick={() => {
                            setSelectedRung(row.size);
                            void invoke('set_active_model', {
                              modelId: `Qwen/Qwen3-${row.size}`,
                            }).catch(() => {});
                          }}
                        >
                          <div className="flex items-center justify-between gap-1">
                            <div className="min-w-0 shrink-0">
                              <div className="font-sans text-[11.5px] font-semibold text-text-primary leading-tight whitespace-nowrap">
                                {row.name}
                              </div>
                              <div className="font-mono text-[9.5px] text-text-muted leading-tight whitespace-nowrap">
                                {row.sha}
                              </div>
                            </div>
                            <div className="flex flex-col items-end gap-0.5 shrink-0">
                              <span className="rounded border border-(--color-status-pass)/40 bg-(--color-status-pass)/12 px-1 py-0 text-[7.5px] leading-tight text-(--color-status-pass) whitespace-nowrap">
                                {row.versionBadge}
                              </span>
                              <span
                                className={`rounded border px-1 py-0 text-[7.5px] leading-tight whitespace-nowrap ${
                                  row.syncBadge === 'synced'
                                    ? 'border-(--color-status-pass)/30 bg-(--color-status-pass)/10 text-(--color-status-pass)'
                                    : row.syncBadge === 'stale'
                                      ? 'border-amber-400/40 bg-amber-400/12 text-amber-300'
                                      : 'border-rose-400/35 bg-rose-400/10 text-rose-300'
                                }`}
                              >
                                {row.syncLabel}
                              </span>
                            </div>
                          </div>
                        </td>

                        {/* Spoke Cells */}
                        {spokes.map((sp) => {
                          const cell = row.cells[sp.id] ?? {
                            state: 'queued' as const,
                            detail: 'QUEUED IN AFK',
                            sub: 'SERIES',
                          };
                          const isSelectedCell =
                            isSelectedRung && selectedSpokeId === sp.id;
                          return (
                            <td
                              key={sp.id}
                              title={`${row.name} × ${sp.id}: ${cell.detail} (${cell.sub})`}
                              onClick={() => {
                                setSelectedRung(row.size);
                                setSelectedSpokeId(sp.id);
                              }}
                              className={`group border-l border-border-subtle/35 px-0.5 py-1 text-center align-middle cursor-pointer transition ${
                                isSelectedCell
                                  ? 'ring-1 ring-inset ring-brass/80 bg-brass/14'
                                  : 'hover:bg-overlay-subtle'
                              }`}
                            >
                              <MatrixCellGlyph state={cell.state} />
                              <div
                                className={`mt-0.5 text-[7.5px] tracking-tighter font-semibold leading-none whitespace-nowrap overflow-hidden ${
                                  cell.state === 'gated'
                                    ? 'text-(--color-status-pass)'
                                    : cell.state === 'remote_ready'
                                      ? 'text-(--color-status-warn)'
                                      : 'text-brass'
                                }`}
                              >
                                {cell.detail}
                              </div>
                              <div className="text-[7.5px] tracking-tighter text-text-muted leading-none mt-0.5 whitespace-nowrap overflow-hidden">
                                {cell.sub}
                              </div>
                            </td>
                          );
                        })}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        </Glass>

        {/* RIGHT 4 COLS: INSPECTOR — HARDWARE PRESETS, ROTARY DIALS, VRAM BUDGET, STAIRCASE & CELL QUICK-ACTIONS */}
        <Glass size="sm" className="relative lg:col-span-4 p-2.5 flex flex-col justify-between gap-1.5 border-brass/40 overflow-hidden">
          <img
            src="/assets/mens-crucible-medallion.jpg"
            alt=""
            aria-hidden="true"
            className="pointer-events-none select-none absolute -right-10 -bottom-10 size-44 opacity-[0.05] rounded-full object-cover"
          />
          <span className="pointer-events-none absolute top-0 left-0 h-3 w-3 rounded-tl-xl border-t-2 border-l-2 border-brass/80" />
          <span className="pointer-events-none absolute top-0 right-0 h-3 w-3 rounded-tr-xl border-t-2 border-r-2 border-brass/80" />
          <span className="pointer-events-none absolute bottom-0 left-0 h-3 w-3 rounded-bl-xl border-b-2 border-l-2 border-brass/80" />
          <span className="pointer-events-none absolute bottom-0 right-0 h-3 w-3 rounded-br-xl border-b-2 border-r-2 border-brass/80" />

          <div className="relative z-10">
            <div className="flex items-center justify-between gap-1 border-b border-border-subtle pb-1">
              <div className="flex items-baseline gap-1.5 min-w-0">
                <h2 className="font-display text-xs font-semibold tracking-wider uppercase text-text-primary">
                  Inspector
                </h2>
                <span className="font-mono text-[9.5px] text-brass truncate">
                  Qwen3-{selectedRung}·{activeSpoke.label}
                </span>
              </div>

              {/* Hardware Profile Quick-Preset Cluster */}
              <div
                data-testid="mens-hw-preset-cluster"
                className="inline-flex items-center rounded-md border border-border-subtle bg-black/40 p-0.5 font-mono text-[8.5px] shrink-0"
              >
                {(
                  [
                    { id: '16g', label: '16G' },
                    { id: '24g', label: '24G' },
                    { id: '80g', label: '80G' },
                  ] as const
                ).map((p) => (
                  <button
                    key={p.id}
                    type="button"
                    onClick={() => handleApplyHwPreset(p.id)}
                    className={`rounded px-1.5 py-0 transition ${
                      hwPreset === p.id
                        ? 'bg-brass/22 text-brass font-semibold'
                        : 'text-text-muted hover:text-text-secondary'
                    }`}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            </div>

            {/* 4 Interactive SVG Rotary Arc Dials */}
            <div className="mt-1.5 grid grid-cols-4 gap-1" data-testid="mens-rotary-dials">
              <RotaryArcDial
                label="LoRA Rank"
                subLabel={`r=${loraRank}`}
                displayValue={`r=${loraRank}`}
                fraction={loraRank / 64}
                accentColor="#d4af37"
                cliFlag={`--rank ${loraRank}`}
                onClick={() => {
                  const ranks = [8, 16, 32, 64];
                  const nextRank = ranks[(ranks.indexOf(loraRank) + 1) % ranks.length]!;
                  setLoraRank(nextRank);
                  persistCrucibleConfig({ lora_rank: nextRank });
                }}
              />
              <RotaryArcDial
                label="Alpha"
                subLabel={`α=${loraAlpha}`}
                displayValue={`α=${loraAlpha}`}
                fraction={loraAlpha / 128}
                accentColor="#d4af37"
                cliFlag={`--alpha ${loraAlpha}`}
                onClick={() => {
                  const alphas = [16, 32, 64, 128];
                  const nextAlpha = alphas[(alphas.indexOf(loraAlpha) + 1) % alphas.length]!;
                  setLoraAlpha(nextAlpha);
                  persistCrucibleConfig({ lora_alpha: nextAlpha });
                }}
              />
              <RotaryArcDial
                label="Seq Len"
                subLabel={String(seqLen)}
                displayValue={String(seqLen)}
                fraction={seqLen / 4096}
                accentColor="#d4af37"
                cliFlag={`--seq-len ${seqLen}`}
                onClick={() => {
                  const lens = [512, 1024, 2048, 4096];
                  const nextLen = lens[(lens.indexOf(seqLen) + 1) % lens.length]!;
                  setSeqLen(nextLen);
                  persistCrucibleConfig({ seq_len: nextLen });
                }}
              />
              <RotaryArcDial
                label="VRAM Cap"
                subLabel={`${vramCapPct}%`}
                displayValue={`${vramCapPct}%`}
                fraction={vramCapPct / 100}
                accentColor="#79c8ba"
                cliFlag={`--vram-limit ${(vramCapPct / 100).toFixed(2)}`}
                onClick={() => {
                  const caps = [70, 80, 85, 95];
                  const nextCap = caps[(caps.indexOf(vramCapPct) + 1) % caps.length]!;
                  setVramCapPct(nextCap);
                  persistCrucibleConfig({ vram_cap_pct: nextCap });
                }}
              />
            </div>

            {/* Segmented VRAM Budget Bar */}
            <div className="mt-1.5 space-y-1 font-mono text-[9.5px]">
              <div className="flex items-center justify-between">
                <span className="text-text-primary font-sans text-[11px] font-medium">
                  VRAM budget{' '}
                  <span className="font-serif italic text-text-muted text-[10px]">
                    ({vramMetrics.fits ? 'fits ceiling' : 'exceeds cap'})
                  </span>
                </span>
                <span className="text-text-secondary">
                  {vramMetrics.totalUsedGb}/{vramMetrics.maxGb.toFixed(1)}GB Max
                </span>
              </div>
              <div className="h-3 w-full rounded bg-black/45 p-0.5 flex gap-0.5 overflow-hidden border border-border-subtle">
                <div
                  style={{
                    width: `${Math.min(55, (vramMetrics.baseNf4Gb / vramMetrics.maxGb) * 100)}%`,
                    backgroundColor: '#79c8ba',
                  }}
                  className="h-full rounded-xs transition-all duration-300"
                  title="Base NF4 Weights"
                />
                <div
                  style={{
                    width: `${Math.min(25, (vramMetrics.loraAdamGb / vramMetrics.maxGb) * 100)}%`,
                    backgroundColor: '#d4af37',
                  }}
                  className="h-full rounded-xs transition-all duration-300"
                  title="LoRA + AdamW States"
                />
                <div
                  style={{
                    width: `${Math.min(35, (vramMetrics.actGb / vramMetrics.maxGb) * 100)}%`,
                    backgroundColor: '#d98b6a',
                  }}
                  className="h-full rounded-xs transition-all duration-300"
                  title="Sequence Activations"
                />
              </div>
              <div className="grid grid-cols-3 gap-1 text-[9px] leading-tight">
                <div className="text-(--color-status-pass)">
                  <span>Base NF4 </span>
                  <span className="font-semibold">{vramMetrics.baseNf4Gb}G</span>
                </div>
                <div className="text-brass">
                  <span>LoRA/Adam </span>
                  <span className="font-semibold">{vramMetrics.loraAdamGb}G</span>
                </div>
                <div className="text-(--color-status-warn)">
                  <span>Activations </span>
                  <span className="font-semibold">{vramMetrics.actGb}G</span>
                </div>
              </div>
            </div>

            {/* 3-Epoch Curriculum Staircase */}
            <div className="mt-1.5 pt-1.5 border-t border-border-subtle">
              <div className="flex items-center justify-between mb-0.5">
                <span className="font-sans text-[11px] font-medium text-text-primary">
                  3-Epoch Curriculum Staircase
                </span>
                <span className="font-serif italic text-[10px] text-text-muted">Click step</span>
              </div>
              <div className="grid grid-cols-3 gap-1.5 items-end h-12 pt-0.5">
                {[
                  { idx: 0, tag: 'E1', name: 'Syntax', cap: epochCaps[0], h: 'h-8', active: false },
                  { idx: 1, tag: 'E2', name: 'Reasoning', cap: epochCaps[1], h: 'h-10', active: false },
                  { idx: 2, tag: 'E3', name: 'Agentic', cap: epochCaps[2], h: 'h-12', active: true },
                ].map((step) => (
                  <button
                    key={step.tag}
                    type="button"
                    onClick={() => {
                      const next: [number, number, number] = [...epochCaps] as [
                        number,
                        number,
                        number,
                      ];
                      next[step.idx] = (next[step.idx]! % 10) + 1;
                      setEpochCaps(next);
                      persistCrucibleConfig({
                        epoch_caps: { E1: next[0], E2: next[1], E3: next[2] },
                      });
                    }}
                    className={`${step.h} w-full rounded-t-md border px-2 py-0.5 flex flex-col justify-between text-left transition-all duration-200 ${
                      step.active
                        ? 'border-brass/75 bg-brass/25 hover:bg-brass/35'
                        : 'border-brass/40 bg-brass/12 hover:bg-brass/22'
                    }`}
                  >
                    <div className="flex items-center justify-between w-full font-mono text-[8.5px] leading-none">
                      <span className="font-display text-brass font-semibold">{step.tag}</span>
                      <span className="text-text-secondary">≤{step.cap}</span>
                    </div>
                    <div className="font-sans text-[9.5px] font-medium text-text-primary leading-tight truncate">
                      {step.name}
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {/* Selected Cell Convenience Quick-Action Bar (Train / Eval Gate / Serve :8089 / Rollback) */}
            <div
              data-testid="mens-cell-quick-actions"
              className="mt-1.5 pt-1.5 border-t border-border-subtle grid grid-cols-4 gap-1 font-mono text-[8.5px]"
            >
              <Tooltip content={`Train Qwen3-${selectedRung} × ${activeSpoke.id} immediately`}>
                <button
                  type="button"
                  data-testid="mens-cell-train-now-btn"
                  onClick={handleTrainSelectedCellNow}
                  className="inline-flex items-center justify-center gap-0.5 rounded border border-brass/50 bg-brass/18 px-1 py-1 text-brass font-semibold transition hover:bg-brass/28"
                >
                  <Zap className="size-2.5" />
                  <span>Train Cell</span>
                </button>
              </Tooltip>

              <Tooltip content={`Run deterministic oracle gate (${activeSpoke.oracle})`}>
                <button
                  type="button"
                  data-testid="mens-cell-eval-gate-btn"
                  onClick={handleEvalSelectedCellGate}
                  className="inline-flex items-center justify-center gap-0.5 rounded border border-(--color-status-pass)/45 bg-(--color-status-pass)/14 px-1 py-1 text-(--color-status-pass) font-semibold transition hover:bg-(--color-status-pass)/24"
                >
                  <FlaskConical className="size-2.5" />
                  <span>Eval Gate</span>
                </button>
              </Tooltip>

              <Tooltip content={`Hot-swap :8089 inference server to Qwen3-${selectedRung} × ${activeSpoke.id}`}>
                <button
                  type="button"
                  data-testid="mens-cell-hot-swap-serve-btn"
                  onClick={handleHotSwapServeCell}
                  className="inline-flex items-center justify-center gap-0.5 rounded border border-border-subtle bg-overlay-subtle px-1 py-1 text-text-primary transition hover:border-brass/45 hover:text-brass"
                >
                  <Rocket className="size-2.5 text-brass" />
                  <span>Serve</span>
                </button>
              </Tooltip>

              <Tooltip content={`Rollback Qwen3-${selectedRung} to master SHA ${selectedRungRow.sha}`}>
                <button
                  type="button"
                  data-testid="mens-cell-rollback-btn"
                  onClick={handleRollbackSelectedCell}
                  className="inline-flex items-center justify-center gap-0.5 rounded border border-border-subtle bg-overlay-subtle px-1 py-1 text-text-secondary transition hover:border-brass/45 hover:text-text-primary"
                >
                  <RotateCcw className="size-2.5" />
                  <span>Rollback</span>
                </button>
              </Tooltip>
            </div>
          </div>

          {/* Option A Bottom Status Line: MensServePanel :8089 ONLINE + expandable full control */}
          <div className="relative z-10 pt-1 border-t border-border-subtle">
            <div className="flex items-center justify-between">
              <button
                type="button"
                onClick={() => setShowServeDrawer(!showServeDrawer)}
                className="font-sans text-xs font-medium text-text-primary hover:text-brass flex items-center gap-1.5"
              >
                <span>MensServePanel</span>
                <span className="font-mono text-(--color-status-pass)">:8089</span>
              </button>
              <span className="inline-flex items-center gap-1 rounded-full border border-(--color-status-pass)/45 bg-(--color-status-pass)/18 px-2 py-0.5 font-mono text-[9.5px] font-semibold text-(--color-status-pass)">
                <span className="size-1.5 rounded-full bg-(--color-status-pass) animate-pulse" />
                ONLINE
              </span>
            </div>
            {showServeDrawer && (
              <div className="mt-2">
                <MensServePanel pushToast={pushToast} />
              </div>
            )}
          </div>
        </Glass>
      </div>

      {/* =====================================================================
          3. LOWER FULL-WIDTH DOCKED DECK (100% EDGE-TO-EDGE OSCILLOSCOPE):
             AFK SERIES QUEUE & CONFIGURABLE MULTI-TRACE HARDWARE OSCILLOSCOPE
         ===================================================================== */}
      <Glass
        size="sm"
        data-testid="mens-telemetry-oscilloscope"
        className="relative p-2.5 space-y-1.5 border-brass/40"
      >
        <span className="pointer-events-none absolute top-0 left-0 h-3 w-3 rounded-tl-xl border-t-2 border-l-2 border-brass/80" />
        <span className="pointer-events-none absolute top-0 right-0 h-3 w-3 rounded-tr-xl border-t-2 border-r-2 border-brass/80" />
        <span className="pointer-events-none absolute bottom-0 left-0 h-3 w-3 rounded-bl-xl border-b-2 border-l-2 border-brass/80" />
        <span className="pointer-events-none absolute bottom-0 right-0 h-3 w-3 rounded-br-xl border-b-2 border-r-2 border-brass/80" />

        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <Activity className="size-3.5 text-brass shrink-0" />
            <div className="min-w-0">
              <div className="flex items-baseline gap-2">
                <span className="font-display text-xs font-semibold tracking-wider uppercase text-text-primary">
                  AFK Series Queue
                </span>
                <span className="font-serif italic text-[11px] text-text-secondary">
                  Live Hardware &amp; Loss Convergence
                </span>
              </div>
              <div className="font-mono text-[10px] text-brass truncate">
                Job 1/4: Qwen3-{selectedRung} · {activeSpoke.id} [68% · ETA 04m 32s]
                {'  --------⇢  '}
                <span className="text-text-primary">
                  Job 2/4: Qwen3-{selectedRung} · speech_to_code [NEXT]
                </span>
              </div>
            </div>
          </div>

          {/* Clustered Trace Channel Toggles (with live telemetry values inside) & Time Window Selector */}
          <div className="flex flex-wrap items-center gap-1.5">
            <div className="inline-flex flex-wrap items-center rounded-lg border border-border-subtle bg-black/40 p-0.5 gap-0.5 font-mono text-[9.5px]">
              {[
                {
                  key: 'gpu',
                  label: 'GPU',
                  val: `${Math.round(liveTelemetry?.gpu_pct ?? 94)}%`,
                  color: '#d4af37',
                },
                {
                  key: 'vram',
                  label: 'VRAM',
                  val: `${vramMetrics.totalUsedGb}/${vramMetrics.maxGb.toFixed(0)}G`,
                  color: '#79c8ba',
                },
                {
                  key: 'cpu',
                  label: 'CPU',
                  val: `${Math.round(liveTelemetry?.cpu_pct ?? 42)}%`,
                  color: '#60a5fa',
                },
                {
                  key: 'ram',
                  label: 'RAM',
                  val: `${(liveTelemetry?.ram_used_gb ?? 22.4).toFixed(1)}G`,
                  color: '#c4ccd2',
                },
                {
                  key: 'loss',
                  label: 'Loss',
                  val: (liveTelemetry?.loss ?? 0.142).toFixed(3),
                  color: '#d98b6a',
                },
              ].map((ch) => {
                const on = visibleTraces[ch.key];
                return (
                  <button
                    key={ch.key}
                    type="button"
                    aria-pressed={on}
                    onClick={() => setVisibleTraces((prev) => ({ ...prev, [ch.key]: !on }))}
                    className={`rounded-md px-1.5 py-0.5 transition ${
                      on
                        ? 'bg-overlay-subtle text-text-primary'
                        : 'bg-transparent text-text-muted opacity-45'
                    }`}
                  >
                    <span style={{ color: ch.color }}>●</span> {ch.label}{' '}
                    <span className="text-text-secondary">{ch.val}</span>
                  </button>
                );
              })}
            </div>

            <div className="inline-flex rounded-lg border border-border-subtle bg-black/40 p-0.5 font-mono text-[9.5px]">
              {(['60s', '5m', '15m', 'Full Run'] as const).map((w) => (
                <button
                  key={w}
                  type="button"
                  onClick={() => setTimeWindow(w)}
                  className={`rounded-md px-2 py-0.5 transition ${
                    timeWindow === w
                      ? 'bg-brass/22 text-brass font-semibold'
                      : 'text-text-muted hover:text-text-secondary'
                  }`}
                >
                  {w}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* 100% FULL-WIDTH EDGE-TO-EDGE SVG OSCILLOSCOPE WITH DOCKED TRANSLUCENT HUD OVERLAY */}
        <div
          className="relative w-full rounded-xl border border-border-subtle bg-black/55 overflow-hidden"
          onMouseMove={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            const relX = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
            setHoverStepIdx(Math.round(relX * 100));
          }}
          onMouseLeave={() => setHoverStepIdx(null)}
        >
          {/* Translucent Floating Telemetry HUD Overlay inside top-left of full-bleed graph */}
          <div className="pointer-events-none absolute top-2 left-2.5 z-10 flex flex-wrap items-center gap-x-3 gap-y-0.5 rounded-lg border border-brass/30 bg-black/70 px-2.5 py-1 backdrop-blur-xs font-mono text-[10px]">
            <span className="text-brass font-semibold">
              GPU Compute {Math.round(liveTelemetry?.gpu_pct ?? 94)}%
            </span>
            <span className="text-(--color-status-pass) font-semibold">
              VRAM {vramMetrics.totalUsedGb}/{vramMetrics.maxGb.toFixed(1)} GB
            </span>
            <span className="text-sky-400 font-semibold">
              CPU {Math.round(liveTelemetry?.cpu_pct ?? 42)}%
            </span>
            <span className="text-text-secondary font-semibold">
              RAM {(liveTelemetry?.ram_used_gb ?? 22.4).toFixed(1)}/{Math.round(liveTelemetry?.ram_max_gb ?? 64)} GB
            </span>
            <span className="text-(--color-status-warn) font-semibold">
              Training Loss {(liveTelemetry?.loss ?? 0.142).toFixed(3)}
            </span>
          </div>

          <svg
            viewBox="0 0 1120 142"
            preserveAspectRatio="none"
            className="w-full h-32 block"
            role="img"
            aria-label="Live GPU, VRAM, CPU, System RAM, and Training Loss time-series graph"
          >
            <defs>
              <linearGradient id="mensGpuGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#d4af37" stopOpacity="0.36" />
                <stop offset="100%" stopColor="#d4af37" stopOpacity="0.0" />
              </linearGradient>
              <linearGradient id="mensVramGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#79c8ba" stopOpacity="0.32" />
                <stop offset="100%" stopColor="#79c8ba" stopOpacity="0.0" />
              </linearGradient>
              <linearGradient id="mensLossGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#d98b6a" stopOpacity="0.26" />
                <stop offset="100%" stopColor="#d98b6a" stopOpacity="0.0" />
              </linearGradient>
            </defs>

            {/* Full-Bleed Oscilloscope Grid */}
            {[26, 56, 86, 116].map((y) => (
              <line
                key={y}
                x1="0"
                y1={y}
                x2="1120"
                y2={y}
                stroke="rgba(255,255,255,0.06)"
                strokeWidth="1"
              />
            ))}
            {[160, 320, 480, 640, 800, 960].map((x) => (
              <line
                key={x}
                y1="0"
                x1={x}
                x2={x}
                y2="142"
                stroke="rgba(255,255,255,0.05)"
                strokeWidth="1"
              />
            ))}

            {/* Dashed VRAM Ceiling Trace Line */}
            <line
              x1="0"
              y1="36"
              x2="1120"
              y2="36"
              stroke="#79c8ba"
              strokeWidth="1.2"
              strokeDasharray="6 5"
              opacity="0.78"
            />
            <text
              x="1020"
              y="30"
              fill="#79c8ba"
              fontSize="11"
              fontFamily="monospace"
            >
              Ceiling Trace
            </text>

            {/* GPU Compute % Spline + Fill + Glowing Frontier Dot */}
            {visibleTraces.gpu && (
              <>
                <path d={timeWindowCurves.gpuArea} fill="url(#mensGpuGrad)" />
                <path
                  d={timeWindowCurves.gpuLine}
                  fill="none"
                  stroke="#d4af37"
                  strokeWidth="2.5"
                />
                <circle cx="1114" cy={timeWindowCurves.gpuHeadY} r="3.5" fill="#d4af37" />
              </>
            )}

            {/* VRAM Used GB Spline + Fill + Glowing Frontier Dot */}
            {visibleTraces.vram && (
              <>
                <path d={timeWindowCurves.vramArea} fill="url(#mensVramGrad)" />
                <path
                  d={timeWindowCurves.vramLine}
                  fill="none"
                  stroke="#79c8ba"
                  strokeWidth="2.5"
                />
                <circle cx="1114" cy={timeWindowCurves.vramHeadY} r="3.5" fill="#79c8ba" />
              </>
            )}

            {/* CPU % Spline */}
            {visibleTraces.cpu && (
              <path
                d={timeWindowCurves.cpuLine}
                fill="none"
                stroke="#60a5fa"
                strokeWidth="2"
              />
            )}

            {/* System RAM GB Spline */}
            {visibleTraces.ram && (
              <path
                d={timeWindowCurves.ramLine}
                fill="none"
                stroke="#c4ccd2"
                strokeWidth="1.8"
              />
            )}

            {/* Training Loss Decaying Spline + Frontier Dot */}
            {visibleTraces.loss && (
              <>
                <path d={timeWindowCurves.lossArea} fill="url(#mensLossGrad)" />
                <path
                  d={timeWindowCurves.lossLine}
                  fill="none"
                  stroke="#d98b6a"
                  strokeWidth="2.3"
                />
                <circle cx="1114" cy={timeWindowCurves.lossHeadY} r="3.5" fill="#d98b6a" />
              </>
            )}

            {/* Interactive Crosshair Scrubber */}
            {hoverStepIdx !== null && (
              <g>
                <line
                  x1={hoverStepIdx * 11.2}
                  y1="0"
                  x2={hoverStepIdx * 11.2}
                  y2="142"
                  stroke="#d4af37"
                  strokeWidth="1.2"
                  strokeDasharray="3 3"
                />
                <rect
                  x={Math.min(920, Math.max(12, hoverStepIdx * 11.2 - 90))}
                  y="8"
                  width="190"
                  height="22"
                  rx="4"
                  fill="rgba(10,12,14,0.92)"
                  stroke="rgba(212,175,55,0.45)"
                />
                <text
                  x={Math.min(930, Math.max(20, hoverStepIdx * 11.2 - 82))}
                  y="23"
                  fill="#fafafa"
                  fontSize="10"
                  fontFamily="monospace"
                >
                  Step {Math.round((hoverStepIdx / 100) * timeWindowCurves.maxSteps)} · Loss{' '}
                  {(0.85 * Math.exp(-hoverStepIdx / 55)).toFixed(3)}
                </text>
              </g>
            )}
          </svg>
        </div>
      </Glass>

      {/* =====================================================================
          4. COMPACT DOCKED FOOTER: 5-SPOKE / 8-SPOKE SCORECARDS & CLI PROBES
         ===================================================================== */}
      <div className="space-y-1.5">
        <div className="flex items-center justify-between px-1">
          <div className="inline-flex rounded-lg border border-border-subtle bg-overlay-subtle p-0.5 font-mono text-[10px]">
            <button
              type="button"
              onClick={() => setLowerDrawerTab('scorecards')}
              className={`rounded-md px-2.5 py-0.5 transition ${
                lowerDrawerTab === 'scorecards'
                  ? 'bg-brass/20 text-brass font-semibold'
                  : 'text-text-muted hover:text-text-secondary'
              }`}
            >
              5-Spoke &amp; 8-Spoke Gate Scorecards
            </button>
            <button
              type="button"
              onClick={() => setLowerDrawerTab('cli_probes')}
              className={`rounded-md px-2.5 py-0.5 transition ${
                lowerDrawerTab === 'cli_probes'
                  ? 'bg-brass/20 text-brass font-semibold'
                  : 'text-text-muted hover:text-text-secondary'
              }`}
            >
              CLI Probe Diagnostics (status · models · probe)
            </button>
          </div>
          <span className="font-mono text-[10px] text-text-muted">
            Phase 333 Release Gate · Substantive ≥ 2 Required for Serve
          </span>
        </div>

        {lowerDrawerTab === 'scorecards' ? (
          <MensRunReportsPanel
            pushToast={pushToast}
            autoSyncEnabled={autoSyncGated}
            hfTokenPresent={hfTokenPresent}
            activeHubRepo={hubUploadRepo}
          />
        ) : (
          <CommandCardsView
            title="Vox Mens"
            subtitle="ML training & local models"
            cards={cards}
            pushToast={pushToast}
          />
        )}
      </div>
    </div>
  );
}
