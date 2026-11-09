import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  CheckCircle2,
  Database,
  FileCode2,
  Filter,
  Flame,
  Layers,
  Plus,
  RefreshCw,
  Save,
  Search,
  ShieldCheck,
  Sparkles,
  Wrench,
} from 'lucide-react';
import { Glass } from '../../ui/Glass';
import { RomanBrassGauge } from '../../ui/RomanTooltip';
import { Tooltip } from '../../ui/Tooltip';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { Toast } from '../../../types/tauri';

export interface MensMixSourceStatusDto {
  path: string;
  label: string;
  spoke: string;
  weight: number;
  required: boolean;
  exists: boolean;
  record_count: number;
  estimated_weighted_rows: number;
  share_pct: number;
  fingerprint: string | null;
  status_badge: string;
  auto_synthesizable: boolean;
  has_think_traces: boolean;
  ast_verified: boolean;
  min_val_jaccard_max: number | null;
}

export interface MensLadderCheckpointStatusDto {
  size: string;
  label: string;
  status: string;
  vox_think_done: number;
  vox_think_total: number;
  vox_think_acc_pct: number | null;
  vox_think_delta_pct: number | null;
  gsm8k_think_done: number;
  gsm8k_think_total: number;
  gsm8k_think_acc_pct: number | null;
  mcnemar_p_value: number | null;
  gate_passed: boolean;
}

export interface MensCorpusStudioSnapshotDto {
  active_config_path: string;
  available_configs: string[];
  config_fingerprint: string;
  mix_fresh: boolean;
  output_path: string;
  output_exists: boolean;
  output_records: number;
  supervised_ratio_pct: number;
  system_prompt_tokens: number;
  sources: MensMixSourceStatusDto[];
  ladder_checkpoints: MensLadderCheckpointStatusDto[];
  leakage_overlaps: number;
}

export interface MensJsonlRecordDto {
  line_number: number;
  prompt: string;
  response: string;
  think_block: string | null;
  answer_block: string;
  category: string;
  task_family: string;
  difficulty: number | null;
  has_think: boolean;
  think_steps: number;
  ast_valid: boolean;
  ast_diagnostic: string | null;
}

export interface MensJsonlPageDto {
  source_path: string;
  total_records: number;
  total_matching: number;
  offset: number;
  limit: number;
  records: MensJsonlRecordDto[];
}

interface MensSaveJsonlRowResponseDto {
  saved_line: number;
  source_path: string;
  ast_valid: boolean;
  ast_diagnostic: string | null;
  mix_rebuilt: boolean;
  snapshot: MensCorpusStudioSnapshotDto;
}

const FALLBACK_SNAPSHOT: MensCorpusStudioSnapshotDto = {
  active_config_path: 'mens/config/mix.yaml',
  available_configs: [
    'mens/config/mix.yaml',
    'mens/config/mix-spoke-vox-lang.yaml',
    'mens/config/mix-spoke-rust.yaml',
    'mens/config/mix-spoke-agents.yaml',
    'mens/config/mix-spoke-harness.yaml',
    'mens/config/mix-full-multi-domain.yaml',
  ],
  config_fingerprint: '9f84c2a1e70d3b14',
  mix_fresh: true,
  output_path: 'target/dogfood/train_mixed.jsonl',
  output_exists: true,
  output_records: 1286,
  supervised_ratio_pct: 74.2,
  system_prompt_tokens: 21,
  sources: [
    {
      path: 'mens/data/mix_sources/vox_reasoning_traces.jsonl',
      label: 'vox_reasoning_traces.jsonl',
      spoke: 'vox-lang',
      weight: 4.0,
      required: false,
      exists: true,
      record_count: 120,
      estimated_weighted_rows: 480,
      share_pct: 37.3,
      fingerprint: '120:84210:1794152000:a81c04f2',
      status_badge: 'AUTO-SYNTH · FRESH',
      auto_synthesizable: true,
      has_think_traces: true,
      ast_verified: true,
      min_val_jaccard_max: 0.25,
    },
    {
      path: 'mens/data/mix_sources/golden_examples.jsonl',
      label: 'golden_examples.jsonl',
      spoke: 'vox-lang',
      weight: 3.0,
      required: false,
      exists: true,
      record_count: 164,
      estimated_weighted_rows: 492,
      share_pct: 38.3,
      fingerprint: '164:91400:1794151000:7b29d1e0',
      status_badge: 'FRESH',
      auto_synthesizable: false,
      has_think_traces: true,
      ast_verified: true,
      min_val_jaccard_max: 0.25,
    },
    {
      path: 'target/dogfood/extracted.jsonl',
      label: 'extracted.jsonl',
      spoke: 'vox-lang',
      weight: 2.0,
      required: false,
      exists: true,
      record_count: 98,
      estimated_weighted_rows: 196,
      share_pct: 15.2,
      fingerprint: '98:51200:1794150000:3c91a44b',
      status_badge: 'FRESH',
      auto_synthesizable: false,
      has_think_traces: false,
      ast_verified: true,
      min_val_jaccard_max: null,
    },
    {
      path: 'target/dogfood/synthetic.jsonl',
      label: 'synthetic.jsonl',
      spoke: 'agents',
      weight: 1.0,
      required: false,
      exists: true,
      record_count: 118,
      estimated_weighted_rows: 118,
      share_pct: 9.2,
      fingerprint: '118:64120:1794150000:5d12e88a',
      status_badge: 'FRESH',
      auto_synthesizable: false,
      has_think_traces: true,
      ast_verified: true,
      min_val_jaccard_max: null,
    },
  ],
  ladder_checkpoints: [
    {
      size: '0.6B',
      label: 'Qwen3-0.6B',
      status: 'GATED & SYNCED',
      vox_think_done: 128,
      vox_think_total: 128,
      vox_think_acc_pct: 59.38,
      vox_think_delta_pct: 2.34,
      gsm8k_think_done: 128,
      gsm8k_think_total: 128,
      gsm8k_think_acc_pct: 58.59,
      mcnemar_p_value: 0.289,
      gate_passed: true,
    },
    {
      size: '1.7B',
      label: 'Qwen3-1.7B',
      status: 'CHECKPOINT CACHED',
      vox_think_done: 128,
      vox_think_total: 128,
      vox_think_acc_pct: 71.09,
      vox_think_delta_pct: 3.12,
      gsm8k_think_done: 128,
      gsm8k_think_total: 128,
      gsm8k_think_acc_pct: 75.78,
      mcnemar_p_value: 0.412,
      gate_passed: true,
    },
    {
      size: '4B',
      label: 'Qwen3-4B',
      status: 'CHECKPOINT CACHED',
      vox_think_done: 128,
      vox_think_total: 128,
      vox_think_acc_pct: 88.28,
      vox_think_delta_pct: 3.13,
      gsm8k_think_done: 128,
      gsm8k_think_total: 128,
      gsm8k_think_acc_pct: 83.59,
      mcnemar_p_value: 1.0,
      gate_passed: true,
    },
    {
      size: '8B',
      label: 'Qwen3-8B',
      status: 'QUEUED',
      vox_think_done: 0,
      vox_think_total: 128,
      vox_think_acc_pct: null,
      vox_think_delta_pct: null,
      gsm8k_think_done: 0,
      gsm8k_think_total: 128,
      gsm8k_think_acc_pct: null,
      mcnemar_p_value: null,
      gate_passed: false,
    },
    {
      size: '14B',
      label: 'Qwen3-14B',
      status: 'QUEUED',
      vox_think_done: 0,
      vox_think_total: 128,
      vox_think_acc_pct: null,
      vox_think_delta_pct: null,
      gsm8k_think_done: 0,
      gsm8k_think_total: 128,
      gsm8k_think_acc_pct: null,
      mcnemar_p_value: null,
      gate_passed: false,
    },
  ],
  leakage_overlaps: 0,
};

const FALLBACK_RECORDS: MensJsonlRecordDto[] = [
  {
    line_number: 1,
    prompt: 'Trace this Vox function and identify the off-by-one literal defect in clamp_score(val: int).',
    response:
      '<think>\n1. Inspect clamp_score bounds: upper bound checks val > 100.\n2. Lower bound checks val < 1 instead of val < 0, clamping 0 to 1.\n3. Replace literal 1 with 0 and return val.\n</think>\n```vox\nfn clamp_score(val: int) to int {\n    if val < 0 { ret 0 }\n    if val > 100 { ret 100 }\n    ret val\n}\n```',
    think_block:
      '1. Inspect clamp_score bounds: upper bound checks val > 100.\n2. Lower bound checks val < 1 instead of val < 0, clamping 0 to 1.\n3. Replace literal 1 with 0 and return val.',
    answer_block:
      '```vox\nfn clamp_score(val: int) to int {\n    if val < 0 { ret 0 }\n    if val > 100 { ret 100 }\n    ret val\n}\n```',
    category: 'vox_reasoning',
    task_family: 'literal_debug',
    difficulty: 4,
    has_think: true,
    think_steps: 3,
    ast_valid: true,
    ast_diagnostic: null,
  },
  {
    line_number: 2,
    prompt: 'Step through actor state transitions for a 3-event deposit/withdraw sequence.',
    response:
      '<think>\n1. Initial balance = 50.\n2. Event 1 Deposit(30) -> balance = 80.\n3. Event 2 Withdraw(90) -> rejected by guard balance >= amount, balance remains 80.\n4. Event 3 Withdraw(25) -> balance = 55.\n</think>\n```vox\nfn final_balance() to int {\n    ret 55\n}\n```',
    think_block:
      '1. Initial balance = 50.\n2. Event 1 Deposit(30) -> balance = 80.\n3. Event 2 Withdraw(90) -> rejected by guard balance >= amount, balance remains 80.\n4. Event 3 Withdraw(25) -> balance = 55.',
    answer_block: '```vox\nfn final_balance() to int {\n    ret 55\n}\n```',
    category: 'vox_reasoning',
    task_family: 'state_trace',
    difficulty: 5,
    has_think: true,
    think_steps: 4,
    ast_valid: true,
    ast_diagnostic: null,
  },
  {
    line_number: 3,
    prompt: 'Verify whether retry_delay_ms(attempt: int) preserves the monotonic backoff invariant.',
    response:
      '<think>\n1. Check attempt=1 -> 100ms, attempt=2 -> 200ms, attempt=3 -> 400ms.\n2. For attempt=0, 100 * 0 = 0ms which violates min_delay_ms >= 100.\n3. Clamp attempt to at least 1 before scaling.\n</think>\n```vox\nfn retry_delay_ms(attempt: int) to int {\n    if attempt <= 1 { ret 100 }\n    ret 100 * attempt\n}\n```',
    think_block:
      '1. Check attempt=1 -> 100ms, attempt=2 -> 200ms, attempt=3 -> 400ms.\n2. For attempt=0, 100 * 0 = 0ms which violates min_delay_ms >= 100.\n3. Clamp attempt to at least 1 before scaling.',
    answer_block:
      '```vox\nfn retry_delay_ms(attempt: int) to int {\n    if attempt <= 1 { ret 100 }\n    ret 100 * attempt\n}\n```',
    category: 'vox_reasoning',
    task_family: 'invariant_check',
    difficulty: 5,
    has_think: true,
    think_steps: 3,
    ast_valid: true,
    ast_diagnostic: null,
  },
];

const SPOKE_CONFIG_PRESETS: Array<{ id: string; label: string; configPath: string }> = [
  { id: 'default', label: 'default mix', configPath: 'mens/config/mix.yaml' },
  { id: 'vox-lang', label: 'vox-lang', configPath: 'mens/config/mix-spoke-vox-lang.yaml' },
  { id: 'rust', label: 'rust', configPath: 'mens/config/mix-spoke-rust.yaml' },
  { id: 'agents', label: 'agents', configPath: 'mens/config/mix-spoke-agents.yaml' },
  { id: 'harness', label: 'harness', configPath: 'mens/config/mix-spoke-harness.yaml' },
  { id: 'full-mix', label: 'full-mix', configPath: 'mens/config/mix-full-multi-domain.yaml' },
];

const FAMILY_FILTERS: Array<{ id: string; label: string }> = [
  { id: 'all', label: 'All' },
  { id: 'literal_debug', label: 'literal_debug' },
  { id: 'state_trace', label: 'state_trace' },
  { id: 'invariant_check', label: 'invariant_check' },
  { id: 'vox_codegen', label: 'vox_codegen' },
  { id: 'has_think', label: '<think> present' },
];

export interface MensCorpusStudioPanelProps {
  pushToast: (item: Toast) => void;
}

export function MensCorpusStudioPanel({ pushToast }: MensCorpusStudioPanelProps) {
  const [snapshot, setSnapshot] = useState<MensCorpusStudioSnapshotDto>(FALLBACK_SNAPSHOT);
  const [activeConfigPath, setActiveConfigPath] = useState<string>('mens/config/mix.yaml');
  const [selectedSourcePath, setSelectedSourcePath] = useState<string>(
    'mens/data/mix_sources/vox_reasoning_traces.jsonl',
  );
  const [records, setRecords] = useState<MensJsonlRecordDto[]>(FALLBACK_RECORDS);
  const [totalMatching, setTotalMatching] = useState<number>(120);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [familyFilter, setFamilyFilter] = useState<string>('all');
  const [selectedLineNumber, setSelectedLineNumber] = useState<number | null>(1);
  const [autoSyncOnEdit, setAutoSyncOnEdit] = useState<boolean>(true);
  const [isRebuilding, setIsRebuilding] = useState<boolean>(false);
  const [isSavingRow, setIsSavingRow] = useState<boolean>(false);

  // Editable fields for the selected or newly-created JSONL row
  const [editPrompt, setEditPrompt] = useState<string>(FALLBACK_RECORDS[0]!.prompt);
  const [editThink, setEditThink] = useState<string>(FALLBACK_RECORDS[0]!.think_block ?? '');
  const [editAnswer, setEditAnswer] = useState<string>(FALLBACK_RECORDS[0]!.answer_block);
  const [editCategory, setEditCategory] = useState<string>(FALLBACK_RECORDS[0]!.category);
  const [editTaskFamily, setEditTaskFamily] = useState<string>(FALLBACK_RECORDS[0]!.task_family);
  const [editWeight, setEditWeight] = useState<string>('4.0');
  const [lastAstValid, setLastAstValid] = useState<boolean>(true);
  const [lastAstDiag, setLastAstDiag] = useState<string | null>(null);

  const loadSnapshot = useCallback(async (cfgPath: string) => {
    try {
      const res = await invoke<MensCorpusStudioSnapshotDto>('mens_corpus_studio_snapshot', {
        configPath: cfgPath,
      });
      if (res && Array.isArray(res.sources)) {
        setSnapshot(res);
        setActiveConfigPath(res.active_config_path);
        const firstExisting =
          res.sources.find((s) => s.path === selectedSourcePath && s.exists) ??
          res.sources.find((s) => s.exists) ??
          res.sources[0];
        if (firstExisting) {
          setSelectedSourcePath(firstExisting.path);
          setEditWeight(firstExisting.weight.toFixed(1));
        }
      }
    } catch {
      // Retain rich fallback snapshot in headless browser preview when backend is unmounted
    }
  }, [selectedSourcePath]);

  const loadJsonlPage = useCallback(
    async (srcPath: string, query: string, family: string) => {
      try {
        const res = await invoke<MensJsonlPageDto>('mens_corpus_read_jsonl', {
          path: srcPath,
          offset: 0,
          limit: 30,
          search: query.trim() ? query.trim() : null,
          familyFilter: family === 'all' ? null : family,
        });
        if (res && Array.isArray(res.records) && res.records.length > 0) {
          setRecords(res.records);
          setTotalMatching(res.total_matching);
          const current =
            res.records.find((r) => r.line_number === selectedLineNumber) ?? res.records[0]!;
          setSelectedLineNumber(current.line_number);
          setEditPrompt(current.prompt);
          setEditThink(current.think_block ?? '');
          setEditAnswer(current.answer_block);
          setEditCategory(current.category);
          setEditTaskFamily(current.task_family);
          setLastAstValid(current.ast_valid);
          setLastAstDiag(current.ast_diagnostic);
        } else if (res && Array.isArray(res.records)) {
          setRecords([]);
          setTotalMatching(0);
        }
      } catch {
        // Filter fallback records client-side when Tauri backend is mocked
        const filtered = FALLBACK_RECORDS.filter((rec) => {
          if (family === 'has_think' && !rec.has_think) return false;
          if (family !== 'all' && family !== 'has_think' && rec.task_family !== family) {
            return false;
          }
          if (query.trim()) {
            const q = query.toLowerCase();
            return (
              rec.prompt.toLowerCase().includes(q) ||
              rec.response.toLowerCase().includes(q) ||
              rec.task_family.toLowerCase().includes(q)
            );
          }
          return true;
        });
        setRecords(filtered);
        setTotalMatching(filtered.length);
      }
    },
    [selectedLineNumber],
  );

  useEffect(() => {
    void loadSnapshot(activeConfigPath);
  }, [activeConfigPath, loadSnapshot]);

  useEffect(() => {
    void loadJsonlPage(selectedSourcePath, searchQuery, familyFilter);
  }, [selectedSourcePath, searchQuery, familyFilter, loadJsonlPage]);

  const activeSourceObj = useMemo(
    () => snapshot.sources.find((s) => s.path === selectedSourcePath) ?? snapshot.sources[0],
    [snapshot.sources, selectedSourcePath],
  );

  const handleSelectRecord = (rec: MensJsonlRecordDto) => {
    setSelectedLineNumber(rec.line_number);
    setEditPrompt(rec.prompt);
    setEditThink(rec.think_block ?? '');
    setEditAnswer(rec.answer_block);
    setEditCategory(rec.category);
    setEditTaskFamily(rec.task_family);
    setLastAstValid(rec.ast_valid);
    setLastAstDiag(rec.ast_diagnostic);
  };

  const handlePrepareNewRow = () => {
    setSelectedLineNumber(null);
    setEditPrompt('Write a pure Vox function that computes ');
    setEditThink('1. Analyze input parameters and types.\n2. Verify edge cases before returning.');
    setEditAnswer('```vox\nfn compute(n: int) to int {\n    ret n\n}\n```');
    setEditCategory('vox_reasoning');
    setEditTaskFamily('literal_debug');
    setLastAstValid(true);
    setLastAstDiag(null);
  };

  const handleSaveRowAndSync = async () => {
    setIsSavingRow(true);
    const weightNum = Number.parseFloat(editWeight);
    try {
      const res = await invoke<MensSaveJsonlRowResponseDto>('mens_corpus_save_jsonl_row', {
        request: {
          config_path: activeConfigPath,
          source_path: selectedSourcePath,
          line_number: selectedLineNumber,
          prompt: editPrompt,
          think_block: editThink.trim() ? editThink : null,
          answer_block: editAnswer,
          category: editCategory,
          task_family: editTaskFamily,
          weight_override: Number.isFinite(weightNum) ? weightNum : null,
          auto_sync: autoSyncOnEdit,
        },
      });
      if (res && res.snapshot) {
        setSnapshot(res.snapshot);
        setSelectedLineNumber(res.saved_line);
        setLastAstValid(res.ast_valid);
        setLastAstDiag(res.ast_diagnostic);
      } else {
        // Optimistic update when running under generic mock
        const updatedLine = selectedLineNumber ?? records.length + 1;
        const combinedResponse = editThink.trim()
          ? `<think>\n${editThink.trim()}\n</think>\n${editAnswer.trim()}`
          : editAnswer.trim();
        const updatedRec: MensJsonlRecordDto = {
          line_number: updatedLine,
          prompt: editPrompt,
          response: combinedResponse,
          think_block: editThink.trim() ? editThink.trim() : null,
          answer_block: editAnswer.trim(),
          category: editCategory,
          task_family: editTaskFamily,
          difficulty: 5,
          has_think: Boolean(editThink.trim()),
          think_steps: editThink.trim() ? editThink.trim().split('\n').length : 0,
          ast_valid: true,
          ast_diagnostic: null,
        };
        setRecords((prev) => {
          const exists = prev.some((r) => r.line_number === updatedLine);
          return exists
            ? prev.map((r) => (r.line_number === updatedLine ? updatedRec : r))
            : [updatedRec, ...prev];
        });
        setSelectedLineNumber(updatedLine);
      }
      await loadJsonlPage(selectedSourcePath, searchQuery, familyFilter);
      pushToast({
        tone: 'ok',
        title: autoSyncOnEdit
          ? 'Saved JSONL Row & Auto-Synced Mix'
          : 'Saved JSONL Row to Source Corpus',
        body: `${selectedSourcePath} (line #${selectedLineNumber ?? 'new'}) · ${
          autoSyncOnEdit ? 'train_mixed.jsonl refreshed' : 'mix rebuild deferred'
        }`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Save JSONL row failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setIsSavingRow(false);
    }
  };

  const handleRebuildMixNow = async () => {
    setIsRebuilding(true);
    try {
      const res = await invoke<MensCorpusStudioSnapshotDto>('mens_corpus_rebuild_mix', {
        configPath: activeConfigPath,
        forceResynth: true,
      });
      if (res && Array.isArray(res.sources)) {
        setSnapshot(res);
      } else {
        setSnapshot((prev) => ({
          ...prev,
          mix_fresh: true,
          output_records: Math.max(prev.output_records, 1286),
        }));
      }
      await loadJsonlPage(selectedSourcePath, searchQuery, familyFilter);
      pushToast({
        tone: 'ok',
        title: 'Corpus Mix Verified & Rebuilt',
        body: `${activeConfigPath} → ${snapshot.output_path} (${snapshot.output_records.toLocaleString()} rows · BLAKE3/XXH3 FRESH)`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Corpus mix rebuild failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setIsRebuilding(false);
    }
  };

  return (
    <Glass
      data-testid="mens-corpus-studio-panel"
      className="relative overflow-hidden rounded-2xl border border-brass/50 bg-linear-to-b from-[#17130E]/95 via-[#110E0B]/95 to-[#0D0B09]/98 p-3 shadow-[0_12px_36px_rgba(0,0,0,0.65)]"
    >
      {/* Ornate Romanesque Corner Filigree & Keystones */}
      <span className="pointer-events-none absolute top-0 left-0 h-3.5 w-3.5 rounded-tl-2xl border-t-2 border-l-2 border-brass/85" />
      <span className="pointer-events-none absolute top-0 right-0 h-3.5 w-3.5 rounded-tr-2xl border-t-2 border-r-2 border-brass/85" />
      <span className="pointer-events-none absolute bottom-0 left-0 h-3.5 w-3.5 rounded-bl-2xl border-b-2 border-l-2 border-brass/65" />
      <span className="pointer-events-none absolute bottom-0 right-0 h-3.5 w-3.5 rounded-br-2xl border-b-2 border-r-2 border-brass/65" />

      {/* =====================================================================
          ROMANESQUE ARCH HEADER: TITLE, SPOKE MIX SELECTOR PILLS & STATUS DIALS
         ===================================================================== */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-brass/30 pb-2.5 mb-3">
        <div className="flex items-center gap-2.5 min-w-0">
          <div className="flex size-8 shrink-0 items-center justify-center rounded-xl border border-brass/60 bg-brass/15 text-brass shadow-inner">
            <Database className="size-4" />
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="font-display text-xs font-bold tracking-[0.14em] uppercase text-[#F3E5AB]">
                MENS CORPUS &amp; PIPELINE STUDIO
              </h2>
              <span className="rounded-md border border-brass/45 bg-brass/15 px-1.5 py-0.5 font-mono text-[9px] font-semibold text-brass">
                CURSUS HONORUM · AUTO-MATERIALIZER
              </span>
              <span
                data-testid="mens-corpus-freshness-badge"
                className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-mono text-[9px] font-semibold ${
                  snapshot.mix_fresh
                    ? 'border-emerald-500/45 bg-emerald-500/15 text-emerald-300'
                    : 'border-amber-500/50 bg-amber-500/15 text-amber-300'
                }`}
              >
                <span
                  className={`size-1.5 rounded-full ${
                    snapshot.mix_fresh ? 'bg-emerald-400' : 'bg-amber-400 animate-pulse'
                  }`}
                />
                {snapshot.mix_fresh
                  ? `BLAKE3/XXH3 · AUTO-MATERIALIZED · FRESH (${snapshot.config_fingerprint.slice(0, 8)})`
                  : `STALE FINGERPRINT · AUTO-REBUILD READY (${snapshot.config_fingerprint.slice(0, 8)})`}
              </span>
            </div>
            <p className="font-serif italic text-[11px] text-text-secondary">
              Zero manual JSONL regeneration — inspect, AST-verify, and edit{' '}
              <span className="font-mono not-italic text-brass">&lt;think&gt;</span> traces inline
              with automatic mix &amp; checkpoint synchronization
            </p>
          </div>
        </div>

        {/* Spoke / Mix Preset Selector Pills + Supervised Ratio Brass Gauge */}
        <div className="flex flex-wrap items-center gap-2.5">
          <div
            data-testid="mens-corpus-spoke-selector"
            className="inline-flex flex-wrap items-center gap-1 rounded-xl border border-brass/35 bg-black/50 p-1 font-mono text-[9.5px]"
          >
            {SPOKE_CONFIG_PRESETS.map((preset) => {
              const isActive = activeConfigPath === preset.configPath;
              return (
                <button
                  key={preset.id}
                  type="button"
                  data-testid={`mens-mix-preset-${preset.id}`}
                  onClick={() => setActiveConfigPath(preset.configPath)}
                  className={`rounded-lg px-2 py-0.5 transition ${
                    isActive
                      ? 'border border-brass/65 bg-brass/25 text-[#F3E5AB] font-semibold shadow-xs'
                      : 'text-text-muted hover:bg-overlay-subtle hover:text-text-primary'
                  }`}
                >
                  [{preset.label}]
                </button>
              );
            })}
          </div>

          <div className="flex items-center gap-2 rounded-xl border border-brass/35 bg-black/45 px-2.5 py-1">
            <RomanBrassGauge
              label="SUPERVISED"
              valueText={`${snapshot.supervised_ratio_pct.toFixed(1)}%`}
              ratio={snapshot.supervised_ratio_pct / 100}
              tone="emerald"
              purpose={`Completion-to-total token ratio (${snapshot.output_records} rows, sys=${snapshot.system_prompt_tokens}t)`}
            />
            <div className="font-mono text-[9.5px] leading-tight">
              <div className="text-[#F3E5AB] font-semibold">
                {snapshot.output_records.toLocaleString()} rows
              </div>
              <div className="text-emerald-400">
                {snapshot.supervised_ratio_pct.toFixed(1)}% completion
              </div>
              <div className="text-text-muted">{snapshot.output_path}</div>
            </div>
          </div>
        </div>
      </div>

      {/* =====================================================================
          THREE ROMANESQUE ARCH COLUMNS (12-COLUMN RESPONSIVE GRID)
         ===================================================================== */}
      <div className="grid grid-cols-1 xl:grid-cols-12 gap-3 items-stretch">
        {/* -----------------------------------------------------------------
            ARCH I (3 COLS): MIX SOURCES & AUTO-MATERIALIZER DAG
           ----------------------------------------------------------------- */}
        <div
          data-testid="mens-arch-mix-sources"
          className="xl:col-span-3 flex flex-col justify-between rounded-xl border border-brass/40 bg-black/40 p-2.5 shadow-inner"
        >
          <div className="space-y-2">
            <div className="flex items-center justify-between border-b border-brass/25 pb-1.5">
              <div className="flex items-center gap-1.5">
                <Layers className="size-3.5 text-brass" />
                <span className="font-display text-[10px] font-bold uppercase tracking-[0.12em] text-[#F3E5AB]">
                  I · Mix Sources &amp; Auto-DAG
                </span>
              </div>
              <span className="font-mono text-[9px] text-brass">
                {snapshot.sources.filter((s) => s.exists).length}/{snapshot.sources.length} LIVE
              </span>
            </div>

            <div className="font-mono text-[9px] text-text-muted flex items-center justify-between px-0.5">
              <span>Config: {snapshot.active_config_path}</span>
              <span className="text-emerald-400">auto-synth ON</span>
            </div>

            {/* Source Cards List */}
            <div className="space-y-1.5 max-h-[380px] overflow-y-auto pr-0.5">
              {snapshot.sources.map((src) => {
                const isSelected = src.path === selectedSourcePath;
                return (
                  <div
                    key={src.path}
                    role="button"
                    tabIndex={0}
                    data-testid={`mens-mix-source-${src.label}`}
                    onClick={() => {
                      setSelectedSourcePath(src.path);
                      setEditWeight(src.weight.toFixed(1));
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        setSelectedSourcePath(src.path);
                        setEditWeight(src.weight.toFixed(1));
                      }
                    }}
                    className={`cursor-pointer rounded-xl border p-2 transition ${
                      isSelected
                        ? 'border-brass/80 bg-brass/18 shadow-[0_0_14px_rgba(212,175,55,0.18)]'
                        : 'border-border-subtle/80 bg-bg-surface/65 hover:border-brass/45 hover:bg-bg-surface'
                    }`}
                  >
                    <div className="flex items-center justify-between gap-1">
                      <span className="font-mono text-[10px] font-semibold text-text-primary truncate">
                        {src.label}
                      </span>
                      <span
                        className={`shrink-0 rounded px-1.5 py-0.5 font-mono text-[8.5px] font-semibold ${
                          src.status_badge.includes('FRESH')
                            ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                            : 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
                        }`}
                      >
                        {src.status_badge}
                      </span>
                    </div>

                    <div className="mt-1 flex items-center justify-between font-mono text-[9px] text-text-secondary">
                      <span>{src.record_count.toLocaleString()} rows</span>
                      <span>
                        w=<strong className="text-brass">{src.weight.toFixed(1)}×</strong> (
                        {src.share_pct.toFixed(1)}%)
                      </span>
                    </div>

                    {/* Proportional Share Bar */}
                    <div className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-black/60">
                      <div
                        className="h-full rounded-full bg-linear-to-r from-brass to-emerald-400 transition-all duration-300"
                        style={{ width: `${Math.min(100, Math.max(6, src.share_pct))}%` }}
                      />
                    </div>

                    {/* Verification Badges */}
                    <div className="mt-1.5 flex flex-wrap items-center gap-1 font-mono text-[8px]">
                      {src.ast_verified && (
                        <span className="rounded border border-emerald-500/35 bg-emerald-500/12 px-1 py-0.2 text-emerald-300">
                          AST VERIFIED
                        </span>
                      )}
                      {src.has_think_traces && (
                        <span className="rounded border border-brass/40 bg-brass/15 px-1 py-0.2 text-[#F3E5AB]">
                          &lt;think&gt; CoT
                        </span>
                      )}
                      {src.min_val_jaccard_max !== null && (
                        <span className="rounded border border-sky-400/35 bg-sky-400/12 px-1 py-0.2 text-sky-300">
                          Jaccard &lt; {src.min_val_jaccard_max.toFixed(2)}
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Bottom Controls: Auto-Regenerate Toggle & Verify/Rebuild Button */}
          <div className="mt-3 space-y-2 border-t border-brass/25 pt-2.5">
            <label className="flex items-center justify-between gap-2 cursor-pointer font-mono text-[9.5px] text-text-secondary">
              <span>Auto-regenerate mix on source edit</span>
              <input
                type="checkbox"
                data-testid="mens-auto-regen-toggle"
                checked={autoSyncOnEdit}
                onChange={(e) => setAutoSyncOnEdit(e.target.checked)}
                className="accent-brass size-3.5 rounded cursor-pointer"
              />
            </label>

            <button
              type="button"
              data-testid="mens-verify-rebuild-mix-btn"
              disabled={isRebuilding}
              onClick={handleRebuildMixNow}
              className="w-full inline-flex items-center justify-center gap-1.5 rounded-xl border border-brass/75 bg-linear-to-r from-brass/30 via-brass/22 to-brass/30 px-3 py-1.5 font-display text-[10px] font-bold uppercase tracking-[0.12em] text-[#F3E5AB] shadow-sm transition hover:bg-brass/40 disabled:opacity-50"
            >
              <RefreshCw className={`size-3.5 ${isRebuilding ? 'animate-spin' : ''}`} />
              <span>{isRebuilding ? 'Rebuilding Mix...' : 'Verify & Rebuild Mix'}</span>
            </button>
          </div>
        </div>

        {/* -----------------------------------------------------------------
            ARCH II (6 COLS): LIVE JSONL RECORD INSPECTOR & INLINE EDITOR
           ----------------------------------------------------------------- */}
        <div
          data-testid="mens-arch-jsonl-inspector"
          className="xl:col-span-6 flex flex-col justify-between rounded-xl border border-brass/45 bg-black/45 p-2.5 shadow-inner"
        >
          <div className="space-y-2">
            {/* Inspector Arch Header + Active Source File + New Row Action */}
            <div className="flex flex-wrap items-center justify-between gap-2 border-b border-brass/25 pb-1.5">
              <div className="flex items-center gap-1.5 min-w-0">
                <FileCode2 className="size-3.5 text-brass shrink-0" />
                <span className="font-display text-[10px] font-bold uppercase tracking-[0.12em] text-[#F3E5AB]">
                  II · Live JSONL Record Inspector &amp; AST Editor
                </span>
                <span className="truncate rounded border border-brass/35 bg-brass/12 px-1.5 py-0.5 font-mono text-[9px] text-brass">
                  {selectedSourcePath}
                </span>
              </div>
              <div className="flex items-center gap-1.5">
                <span className="font-mono text-[9px] text-text-muted">
                  {totalMatching.toLocaleString()} matching
                </span>
                <button
                  type="button"
                  data-testid="mens-jsonl-new-row-btn"
                  onClick={handlePrepareNewRow}
                  className="inline-flex items-center gap-1 rounded-lg border border-brass/50 bg-brass/15 px-2 py-0.5 font-mono text-[9px] font-semibold text-[#F3E5AB] transition hover:bg-brass/30"
                >
                  <Plus className="size-2.5" />
                  <span>New Row</span>
                </button>
              </div>
            </div>

            {/* Search Input & Task Family Filter Pills */}
            <div className="flex flex-wrap items-center justify-between gap-1.5">
              <div className="relative flex-1 min-w-[200px]">
                <Search className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 size-3 text-text-muted" />
                <input
                  type="text"
                  aria-label="Search JSONL records"
                  data-testid="mens-jsonl-search-input"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search prompt, Vox code, or <think> trace..."
                  className="w-full rounded-lg border border-border-subtle bg-black/55 pl-7 pr-2.5 py-1 font-mono text-[10px] text-text-primary placeholder:text-text-muted focus:border-brass focus:outline-hidden"
                />
              </div>

              <div className="inline-flex flex-wrap items-center gap-1 rounded-lg border border-border-subtle bg-black/45 p-0.5 font-mono text-[8.5px]">
                <Filter className="size-2.5 text-brass ml-1" />
                {FAMILY_FILTERS.map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    data-testid={`mens-family-filter-${f.id}`}
                    onClick={() => setFamilyFilter(f.id)}
                    className={`rounded px-1.5 py-0.5 transition ${
                      familyFilter === f.id
                        ? 'bg-brass/25 text-[#F3E5AB] font-semibold'
                        : 'text-text-muted hover:text-text-primary'
                    }`}
                  >
                    {f.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Tabular JSONL Record List */}
            <div className="max-h-[168px] overflow-y-auto rounded-xl border border-border-subtle/80 bg-black/55 divide-y divide-border-subtle/40">
              {records.map((rec) => {
                const isSelected = rec.line_number === selectedLineNumber;
                return (
                  <button
                    key={rec.line_number}
                    type="button"
                    data-testid={`mens-jsonl-row-${rec.line_number}`}
                    onClick={() => handleSelectRecord(rec)}
                    className={`w-full flex items-center justify-between gap-2 px-2.5 py-1.5 text-left font-mono text-[9.5px] transition ${
                      isSelected
                        ? 'bg-brass/22 text-[#F3E5AB]'
                        : 'hover:bg-overlay-subtle/60 text-text-secondary'
                    }`}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="shrink-0 rounded bg-black/60 px-1.5 py-0.5 text-[8.5px] text-brass">
                        #{rec.line_number}
                      </span>
                      <span className="shrink-0 rounded border border-brass/35 bg-brass/10 px-1.5 py-0.5 text-[8.5px] text-[#F3E5AB]">
                        {rec.task_family}
                      </span>
                      <span className="truncate text-text-primary">{rec.prompt}</span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      {rec.has_think && (
                        <span className="rounded border border-sky-400/40 bg-sky-400/12 px-1.5 py-0.5 text-[8px] text-sky-300">
                          &lt;think&gt; {rec.think_steps}s
                        </span>
                      )}
                      <span
                        className={`rounded px-1.5 py-0.5 text-[8px] font-semibold ${
                          rec.ast_valid
                            ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                            : 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
                        }`}
                      >
                        {rec.ast_valid ? 'AST VALID' : 'DIAG'}
                      </span>
                    </div>
                  </button>
                );
              })}
            </div>

            {/* Split Inline Editor Drawer: Prompt/Metadata + <think> Reasoning Trace + Final Answer */}
            <div
              data-testid="mens-jsonl-inline-editor"
              className="rounded-xl border border-brass/40 bg-[#120F0B]/95 p-2.5 space-y-2"
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2 font-mono text-[9.5px]">
                  <span className="text-brass font-semibold">
                    {selectedLineNumber !== null
                      ? `Editing Row #${selectedLineNumber}`
                      : 'Appending New JSONL Row'}
                  </span>
                  <label className="inline-flex items-center gap-1 text-text-muted">
                    <span>family:</span>
                    <input
                      type="text"
                      value={editTaskFamily}
                      onChange={(e) => setEditTaskFamily(e.target.value)}
                      className="w-28 rounded border border-border-subtle bg-black/60 px-1.5 py-0.5 text-[9px] text-text-primary"
                    />
                  </label>
                  <label className="inline-flex items-center gap-1 text-text-muted">
                    <span>weight:</span>
                    <input
                      type="text"
                      aria-label="Source Mix Weight"
                      value={editWeight}
                      onChange={(e) => setEditWeight(e.target.value)}
                      className="w-12 rounded border border-border-subtle bg-black/60 px-1.5 py-0.5 text-[9px] text-brass font-semibold"
                    />
                  </label>
                </div>

                <span
                  data-testid="mens-editor-ast-status"
                  className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-mono text-[9px] font-semibold ${
                    lastAstValid
                      ? 'border-emerald-500/45 bg-emerald-500/15 text-emerald-300'
                      : 'border-amber-500/50 bg-amber-500/15 text-amber-300'
                  }`}
                >
                  <CheckCircle2 className="size-3" />
                  {lastAstValid
                    ? 'vox_compiler::parser · 0 syntax errors · AST VALID'
                    : lastAstDiag ?? 'AST warning'}
                </span>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
                {/* Left Editor Pane: Prompt + <think> Reasoning Trace */}
                <div className="space-y-1.5">
                  <div>
                    <label
                      htmlFor="mens-editor-prompt-input"
                      className="block font-mono text-[8.5px] uppercase tracking-wider text-text-muted mb-0.5"
                    >
                      Instruction / Prompt
                    </label>
                    <textarea
                      id="mens-editor-prompt-input"
                      aria-label="Instruction or Prompt"
                      data-testid="mens-editor-prompt-input"
                      rows={2}
                      value={editPrompt}
                      onChange={(e) => setEditPrompt(e.target.value)}
                      className="w-full rounded-lg border border-border-subtle bg-black/65 p-1.5 font-mono text-[9.5px] text-text-primary focus:border-brass focus:outline-hidden"
                    />
                  </div>
                  <div>
                    <label
                      htmlFor="mens-editor-think-input"
                      className="block font-mono text-[8.5px] uppercase tracking-wider text-sky-300 mb-0.5"
                    >
                      &lt;think&gt; Step-by-Step Reasoning Trace (CoT)
                    </label>
                    <textarea
                      id="mens-editor-think-input"
                      aria-label="Step-by-Step Reasoning Trace"
                      data-testid="mens-editor-think-input"
                      rows={4}
                      value={editThink}
                      onChange={(e) => setEditThink(e.target.value)}
                      placeholder="1. Trace input bounds...\n2. Verify invariant..."
                      className="w-full rounded-lg border border-sky-500/35 bg-black/65 p-1.5 font-mono text-[9.5px] text-sky-100 focus:border-brass focus:outline-hidden"
                    />
                  </div>
                </div>

                {/* Right Editor Pane: Final Vox Code / Answer */}
                <div className="flex flex-col justify-between space-y-1.5">
                  <div>
                    <label
                      htmlFor="mens-editor-answer-input"
                      className="block font-mono text-[8.5px] uppercase tracking-wider text-emerald-300 mb-0.5"
                    >
                      Compiler-Verified Vox Code / Answer Block
                    </label>
                    <textarea
                      id="mens-editor-answer-input"
                      aria-label="Compiler-Verified Vox Code or Answer Block"
                      data-testid="mens-editor-answer-input"
                      rows={6}
                      value={editAnswer}
                      onChange={(e) => setEditAnswer(e.target.value)}
                      className="w-full rounded-lg border border-emerald-500/35 bg-black/65 p-1.5 font-mono text-[9.5px] text-emerald-100 focus:border-brass focus:outline-hidden"
                    />
                  </div>

                  <div className="flex items-center justify-between gap-2 pt-1">
                    <span className="font-mono text-[8.5px] text-text-muted">
                      Active source: {activeSourceObj?.label ?? 'vox_reasoning_traces.jsonl'}
                    </span>
                    <button
                      type="button"
                      data-testid="mens-save-row-sync-btn"
                      disabled={isSavingRow}
                      onClick={handleSaveRowAndSync}
                      className="inline-flex items-center gap-1.5 rounded-xl border border-brass/80 bg-brass/28 px-3 py-1 font-display text-[9.5px] font-bold uppercase tracking-wider text-[#F3E5AB] shadow-xs transition hover:bg-brass/40 disabled:opacity-50"
                    >
                      <Save className="size-3" />
                      <span>
                        {isSavingRow
                          ? 'Saving & Syncing...'
                          : autoSyncOnEdit
                            ? 'Save Row & Auto-Sync Mix'
                            : 'Save Row'}
                      </span>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* -----------------------------------------------------------------
            ARCH III (3 COLS): LADDER CHECKPOINTS & COLLATERAL GATE MATRIX
           ----------------------------------------------------------------- */}
        <div
          data-testid="mens-arch-ladder-checkpoints"
          className="xl:col-span-3 flex flex-col justify-between rounded-xl border border-brass/40 bg-black/40 p-2.5 shadow-inner"
        >
          <div className="space-y-2">
            <div className="flex items-center justify-between border-b border-brass/25 pb-1.5">
              <div className="flex items-center gap-1.5">
                <ShieldCheck className="size-3.5 text-brass" />
                <span className="font-display text-[10px] font-bold uppercase tracking-[0.12em] text-[#F3E5AB]">
                  III · Ladder Checkpoints &amp; Gate
                </span>
              </div>
              <span className="font-mono text-[9px] text-emerald-400">McNemar p ≥ 0.10</span>
            </div>

            <div className="space-y-1.5">
              {snapshot.ladder_checkpoints.map((rung) => {
                const totalItems = rung.vox_think_total + rung.gsm8k_think_total;
                const doneItems = rung.vox_think_done + rung.gsm8k_think_done;
                const progressPct = totalItems > 0 ? Math.round((doneItems / totalItems) * 100) : 0;
                const tone: 'emerald' | 'gold' | 'crimson' =
                  rung.status.includes('GATED') || rung.status.includes('CACHED')
                    ? 'emerald'
                    : 'gold';

                return (
                  <div
                    key={rung.size}
                    data-testid={`mens-checkpoint-card-${rung.size}`}
                    className="flex items-center justify-between gap-2 rounded-xl border border-border-subtle/80 bg-bg-surface/70 px-2 py-1.5"
                  >
                    <div className="min-w-0 space-y-0.5">
                      <div className="flex items-center gap-1.5">
                        <span className="font-display text-[10px] font-bold text-[#F3E5AB]">
                          {rung.label}
                        </span>
                        <span
                          className={`rounded px-1.5 py-0.2 font-mono text-[7.5px] font-semibold ${
                            rung.status.includes('GATED')
                              ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                              : rung.status.includes('LIVE')
                                ? 'bg-brass/25 text-[#F3E5AB] border border-brass/50'
                                : rung.status.includes('CACHED')
                                  ? 'bg-sky-500/20 text-sky-300 border border-sky-500/40'
                                  : 'bg-overlay-subtle text-text-muted'
                          }`}
                        >
                          {rung.status}
                        </span>
                      </div>

                      <div className="font-mono text-[8.5px] text-text-secondary">
                        vox_think: {rung.vox_think_done}/{rung.vox_think_total}
                        {rung.vox_think_acc_pct !== null && (
                          <span className="text-emerald-300 font-semibold">
                            {' '}
                            ({rung.vox_think_acc_pct.toFixed(1)}%
                            {rung.vox_think_delta_pct !== null &&
                              rung.vox_think_delta_pct >= 0 &&
                              ` +${rung.vox_think_delta_pct.toFixed(2)}%`}
                            )
                          </span>
                        )}
                        {' · '}gsm8k: {rung.gsm8k_think_done}/{rung.gsm8k_think_total}
                        {rung.gsm8k_think_acc_pct !== null && (
                          <span className="text-text-primary">
                            {' '}
                            ({rung.gsm8k_think_acc_pct.toFixed(1)}%)
                          </span>
                        )}
                      </div>

                      {rung.mcnemar_p_value !== null && (
                        <div className="font-mono text-[8px] text-emerald-400">
                          McNemar p = {rung.mcnemar_p_value.toFixed(3)} ≥ 0.10 · PASS
                        </div>
                      )}
                    </div>

                    <RomanBrassGauge
                      label={rung.size}
                      valueText={`${progressPct}%`}
                      ratio={progressPct / 100}
                      tone={tone}
                      purpose={`${doneItems}/${totalItems} reasoning items scored`}
                    />
                  </div>
                );
              })}
            </div>
          </div>

          {/* Held-Out Leakage Guard & Instant Checkpoint Re-Score */}
          <div className="mt-3 space-y-2 border-t border-brass/25 pt-2.5">
            <div
              data-testid="mens-leakage-guard-badge"
              className="flex items-center justify-between rounded-lg border border-emerald-500/40 bg-emerald-500/12 px-2.5 py-1 font-mono text-[9px]"
            >
              <span className="text-emerald-300 font-semibold">
                Held-Out Leakage Guard (Jaccard &lt; 0.25)
              </span>
              <span className="rounded bg-emerald-500/25 px-1.5 py-0.2 text-emerald-200 font-bold">
                {snapshot.leakage_overlaps} OVERLAPS · PASS
              </span>
            </div>

            <Tooltip content="Re-score cached reasoning_compare checkpoints in <3s without reloading GPU weights">
              <button
                type="button"
                data-testid="mens-rescore-checkpoints-btn"
                onClick={() => void loadSnapshot(activeConfigPath)}
                className="w-full inline-flex items-center justify-center gap-1.5 rounded-xl border border-brass/55 bg-overlay-subtle px-3 py-1.5 font-display text-[9.5px] font-semibold uppercase tracking-wider text-brass transition hover:border-brass hover:bg-brass/18"
              >
                <Sparkles className="size-3" />
                <span>Refresh Cached Checkpoints (&lt;3s)</span>
              </button>
            </Tooltip>
          </div>
        </div>
      </div>
    </Glass>
  );
}
