import React, { useState, useRef, useEffect, useMemo } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { Glass } from '../../ui/Glass';
import { Icon } from '../../ui/Icons';
import { Popover } from '../../ui/Popover';
import { Tooltip } from '../../ui/Tooltip';
import { voxTransport } from '../../../transport';
import { steerChatQueue } from '../../../transport/t3Wave3Rpc';
import { buildSlashEntries, type SlashEntry } from '../../../lib/slashCommands';
import { computeIntelliSenseItems, type IntelliSenseItem } from '../../../lib/intellisenseEngine';
import { IntelliSensePopover } from '../../ui/IntelliSensePopover';
import {
  COMPOSER_HISTORY_CAP,
  LOQUELA_FILE_PICKER_DEBOUNCE_MS,
  LOQUELA_FILE_PICKER_LIMIT,
  LOQUELA_MODEL_SEARCH_DEBOUNCE_MS,
  LOQUELA_MODEL_SEARCH_LIMIT,
  LOQUELA_TIER_MODEL_COUNT,
} from '../../../config/constants';
import {
  filterPickerModels,
  isRoutingTierId,
  normalizeModelCard,
  shortModelLabel,
  type PickerModel,
  type ProviderStatus,
} from '../../../lib/modelPicker';
import { ModelPickerSearch } from '../Chat/ModelPickerSearch';
import type {
  ActiveSkill,
  CatalogEntry,
  ChatPayload,
  ComposerAgentRef,
  ComposerInteractionMode,
  KbScopeMode,
  OversightLevelId,
  Toast,
} from '../../../types/tauri';
import {
  isAppSlashCommand,
  resolveInternalModeSlash,
} from '../../../lib/slashRouter';
import { DriveConsole } from './DriveConsole';
import { CLUTCH_DETENTS, defaultControl, publishControlState, publishModelOverride, type ClutchId, type ControlState, type RiskId } from '../../../lib/driveConsole';
import { PromptHistoryController } from '../../../lib/chat/promptHistory';
import { registerLoquelaDriveApi } from '../../../lib/useDriveBus';
import type { DriveExecution } from '../../../lib/axisDrive';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { IntentPanel } from './IntentPanel';
import {
  EMPTY_INTENT,
  hasIntent,
  composeDescription,
  effortToPriority,
  type IntentFields,
} from '../../../lib/intentSpec';
import {
  getTerminalBridgeState,
  onChatAttention,
  subscribeTerminalBridge,
  TERM_MENTION_OPTIONS,
  type TerminalBridgeState,
} from '../../../lib/terminalBridge';

// LQ_MODES kept for slash command hint lookup (/plan, /verify, /act).
// The Segment UI has been replaced by DriveConsole. Remove in Track D when
// mode transitions are fully migrated to clutch+risk.
const LQ_MODES = [
  { id: "plan",   label: "Plan",   hint: "Augur drafts a plan, no side effects" },
  { id: "act",    label: "Act",    hint: "Plan → execute under risk gates" },
  { id: "verify", label: "Verify", hint: "Re-run with stricter doubt + property tests" },
];

// Static fallback tiers shown before the runtime model list loads.
// Cost is `null` (unknown) — real per-1k pricing is injected from listModels()
// once available. We never display a fabricated price.
const LQ_TIERS = [
  { id: "auto", label: "Auto", detail: "tier-router decides · 0% markup", cost: null as number | null, lat: null as number | null },
  { id: "local", label: "Local · Mens", detail: "on-device · $0 · Free", cost: 0 as number | null, lat: null as number | null },
  { id: "mesh", label: "Mesh · Peers", detail: "peers · $0 · Free", cost: 0 as number | null, lat: null as number | null },
  { id: "cloud", label: "Cloud", detail: "cloud tier · 0% Vox markup", cost: null as number | null, lat: null as number | null },
];

const ROUTING_TIERS = [
  { id: "auto", label: "Auto", detail: "live routing summary · 0% markup", cost: null as number | null, lat: null as number | null },
  { id: "local", label: "Local · Mens", detail: "on-device / VOX_LOCAL · $0 · Free", cost: 0 as number | null, lat: null as number | null },
  { id: "mesh", label: "Mesh · Peers", detail: "peers · $0 · Free", cost: 0 as number | null, lat: null as number | null },
  { id: "cloud", label: "Cloud", detail: "cloud tier · 0% Vox markup", cost: null as number | null, lat: null as number | null },
];

interface ChipData {
  id: string;
  kind: 'file' | 'skill' | 'agent' | 'branch' | 'url' | 'image' | 'term';
  label: string;
  meta?: string;
}

const CHIP_ICON_MAP: Record<ChipData['kind'], React.ComponentType<{ className?: string }>> = {
  file: Icon.file,
  skill: Icon.bolt,
  agent: Icon.users,
  branch: Icon.branch,
  url: Icon.link,
  image: Icon.eye,
  term: Icon.terminal,
};

function Chip({
  chip,
  onRemove,
  onEdit,
}: {
  chip: ChipData;
  onRemove: (c: ChipData) => void;
  onEdit?: (c: ChipData, nextLabel: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(chip.label);
  const IconCmp = CHIP_ICON_MAP[chip.kind] ?? Icon.file;
  const tone = chip.kind === "file"   ? "border-(--color-status-warn)/25 text-(--color-status-warn) bg-(--color-status-warn)/5"
            : chip.kind === "skill"  ? "border-brass/30 text-brass bg-brass/5"
            : chip.kind === "agent"  ? "border-violet-400/25 text-violet-300 bg-violet-400/5"
            : chip.kind === "branch" ? "border-(--color-status-pass)/25 text-(--color-status-pass) bg-(--color-status-pass)/5"
            : chip.kind === "term"   ? "border-(--color-status-pass)/30 text-(--color-status-pass) bg-(--color-status-pass)/10"
            :                          "border-border-subtle text-text-secondary bg-overlay-subtle";
  const commit = () => {
    const trimmed = draft.trim();
    if (trimmed && trimmed !== chip.label) {
      onEdit?.(chip, trimmed);
    }
    setEditing(false);
  };
  return (
    <span data-testid={`loquela-chip-${chip.kind}`} className={`group inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 font-mono text-[11px] ${tone}`}>
      <IconCmp className="size-3" />
      {editing ? (
        <input
          autoFocus
          data-testid={`loquela-chip-edit-input-${chip.id}`}
          aria-label={`Edit context chip ${chip.label}`}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') commit();
            if (e.key === 'Escape') {
              setDraft(chip.label);
              setEditing(false);
            }
          }}
          className="w-32 rounded border border-brass/40 bg-surface-base/80 px-1 py-0.5 font-mono text-[11px] text-text-primary outline-hidden"
        />
      ) : (
        <button
          type="button"
          data-testid={`loquela-chip-edit-${chip.id}`}
          aria-label={`Edit ${chip.label}`}
          onClick={() => {
            setDraft(chip.label);
            setEditing(true);
          }}
          className="truncate max-w-[180px] text-left hover:underline"
        >
          {chip.label}
        </button>
      )}
      {chip.meta && <span className="text-text-muted">· {chip.meta}</span>}
      <button type="button" aria-label={`Remove ${chip.label}`} onClick={() => onRemove(chip)} className="ml-0.5 opacity-40 hover:opacity-100"><Icon.x className="size-2.5" aria-hidden="true" /></button>
    </span>
  );
}

export interface SessionBudgetDisplay {
  spent: number;
  cap: number;
  source?: string;
}

export interface SlashCommandContext {
  setText: (text: string) => void;
}

interface LoquelaProps {
  chips: ChipData[];
  setChips: React.Dispatch<React.SetStateAction<ChipData[]>>;
  onSubmit: (payload: ChatPayload) => void;
  activeSkill: ActiveSkill | null;
  setActiveSkill: (s: ActiveSkill | null) => void;
  skills: CatalogEntry[];
  toast?: (t: Toast) => void;
  agents?: ComposerAgentRef[];
  sessionBudget?: SessionBudgetDisplay;
  /** When true, the slash command was handled and should not be inserted into the composer. */
  onSlashCommand?: (cmd: string, ctx: SlashCommandContext) => boolean | Promise<boolean>;
  queueDepth?: number;
  onOpenTasks?: () => void;
  /** True when a submitted task is still in progress — flips Send button to Stop. */
  taskInProgress?: boolean;
  /** The in-flight task id (numeric orchestrator id) used when interrupting. */
  currentTaskId?: number;
  /** Called when the user clicks Stop; should interrupt the active orchestrator task. */
  onInterrupt?: (taskId?: number) => void;
  /** True when the current agent (not just the task) is paused. */
  agentPaused?: boolean;
  /** The agent to resume; only meaningful when agentPaused is true. */
  currentAgent?: { id: string } | null;
  /** Called when the user clicks Resume. */
  onResume?: (agent: { id: string }) => void;
  /**
   * Rendered at the far right of the toolbar row, after the cost/budget
   * display — e.g. the model-route picker. An opaque slot (like `composer`
   * in ChatSurface) so Loquela doesn't need to know about chat-specific
   * model state.
   */
  trailingSlot?: React.ReactNode;
  /** Rendered inside the Risk popover (App passes the Check replies toggle). */
  riskSlot?: React.ReactNode;
  /** Lift a concrete model pick into App `chatModelOverride`. */
  onModelPick?: (modelId: string | null) => void;
  /** Current App-level override so "Run on" stays in sync with ChatModelPicker. */
  selectedModelId?: string | null;
  /** Optional active session token count to trigger the >24k Compact Context chip (HUMANE-03). */
  contextTokens?: number;
}

/** Send modes. `plan` is a real execution (buildChatTurn → `execution: 'plan'`), so it gets a menu entry. */
const SEND_MODES: { id: 'chat' | 'task' | 'plan'; label: string; hint: string }[] = [
  { id: 'chat', label: 'Quick chat', hint: 'Synchronous reply, no background task' },
  { id: 'task', label: 'Background task', hint: 'Dispatch as an autonomous task, not blocking' },
  { id: 'plan', label: 'Plan', hint: 'Draft a plan first' },
];

const OVERSIGHT_CYCLE: OversightLevelId[] = ['relaxed', 'guided', 'balanced', 'cautious', 'strict'];
const KB_SCOPE_CYCLE: KbScopeMode[] = ['auto', 'docs', 'code', 'memory', 'verified', 'off'];
const TIER_CYCLE = ['auto', 'local', 'mesh', 'cloud'] as const;
const CLUTCH_CYCLE: ClutchId[] = ['free', 'efficiency', 'balanced', 'genius'];
const RISK_CYCLE: RiskId[] = ['high', 'moderate', 'low'];
const SEND_MODE_CYCLE: ('chat' | 'task' | 'plan')[] = ['chat', 'task', 'plan'];

const TIER_BUTTON_TONE: Record<string, string> = {
  local: 'border-(--color-status-pass)/40 bg-(--color-status-pass)/12 text-(--color-status-pass) hover:bg-(--color-status-pass)/18',
  mesh: 'border-(--color-status-info)/40 bg-(--color-status-info)/12 text-(--color-status-info) hover:bg-(--color-status-info)/18',
  cloud: 'border-violet-400/40 bg-violet-400/12 text-violet-300 hover:bg-violet-400/18',
  auto: 'border-brass/35 bg-brass/10 text-text-secondary hover:border-brass/55',
};

const SEND_MODE_TONE: Record<'chat' | 'task' | 'plan', string> = {
  chat: 'border-brass/35 bg-brass/10 text-brass hover:bg-brass/18',
  task: 'border-(--color-status-info)/40 bg-(--color-status-info)/12 text-(--color-status-info) hover:bg-(--color-status-info)/18',
  plan: 'border-violet-400/40 bg-violet-400/12 text-violet-300 hover:bg-violet-400/18',
};

const OVERSIGHT_TONE: Record<OversightLevelId, string> = {
  relaxed: 'border-(--color-status-warn)/35 bg-(--color-status-warn)/10 text-(--color-status-warn)',
  guided: 'border-(--color-status-info)/35 bg-(--color-status-info)/10 text-(--color-status-info)',
  balanced: 'border-brass/35 bg-brass/10 text-brass',
  cautious: 'border-(--color-status-pass)/35 bg-(--color-status-pass)/10 text-(--color-status-pass)',
  strict: 'border-(--color-status-pass)/50 bg-(--color-status-pass)/16 text-(--color-status-pass)',
};

const KB_SCOPE_TONE: Record<KbScopeMode, string> = {
  auto: 'border-brass/35 bg-brass/10 text-brass',
  docs: 'border-(--color-status-info)/40 bg-(--color-status-info)/12 text-(--color-status-info)',
  code: 'border-(--color-status-pass)/40 bg-(--color-status-pass)/12 text-(--color-status-pass)',
  memory: 'border-violet-400/40 bg-violet-400/12 text-violet-300',
  verified: 'border-(--color-status-pass)/45 bg-(--color-status-pass)/14 text-(--color-status-pass)',
  off: 'border-border-subtle bg-overlay-subtle text-text-muted',
};

export function Loquela({
  chips,
  setChips,
  onSubmit,
  activeSkill,
  setActiveSkill,
  skills,
  toast,
  agents = [],
  sessionBudget,
  onSlashCommand,
  queueDepth,
  onOpenTasks,
  taskInProgress = false,
  currentTaskId,
  onInterrupt,
  agentPaused = false,
  currentAgent,
  onResume,
  trailingSlot,
  riskSlot,
  onModelPick,
  selectedModelId = null,
  contextTokens = 0,
}: LoquelaProps) {
  const embedded = useIsEmbeddedSurface();
  const [text, setText] = useState("");
  const [mode, setMode] = useState<ComposerInteractionMode>("act");
  const [tier, setTier] = useState("auto");
  const [tierQuery, setTierQuery] = useState('');
  const [oversightLevel, setOversightLevel] = useState<OversightLevelId>('balanced');
  const [kbScope, setKbScope] = useState<KbScopeMode>('auto');
  const [control, setControl] = useState<ControlState>(defaultControl);
  useEffect(() => {
    publishControlState(control);
  }, [control]);
  useEffect(() => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent('vox:kb-scope-change', {
          detail: { kbScope, oversightLevel },
        }),
      );
    }
  }, [kbScope, oversightLevel]);
  useEffect(() => {
    const onAtlasMention = (ev: Event) => {
      const custom = ev as CustomEvent<{ token?: string }>;
      const token = custom.detail?.token;
      if (token) {
        setText((prev) => (prev.trim().length > 0 ? `${prev.trim()} ${token} ` : `${token} `));
      }
    };
    window.addEventListener('vox:atlas-mention', onAtlasMention);
    return () => window.removeEventListener('vox:atlas-mention', onAtlasMention);
  }, []);
  useEffect(() => {
    const onBrowserContext = (ev: Event) => {
      const custom = ev as CustomEvent<{
        url?: string;
        pageId?: string;
        refCount?: number;
        summary?: string;
        token?: string;
      }>;
      const detail = custom.detail;
      if (!detail) return;
      const chipId = `browser-${detail.pageId ?? 'active'}`;
      const label = detail.url ?? detail.pageId ?? 'browser-session';
      const meta =
        typeof detail.refCount === 'number'
          ? `${detail.refCount} refs`
          : 'songhand';
      setChips((prev) => {
        const filtered = prev.filter((c) => c.id !== chipId);
        return [...filtered, { id: chipId, kind: 'url', label, meta }];
      });
    };
    window.addEventListener('vox:browser-context', onBrowserContext);
    return () => window.removeEventListener('vox:browser-context', onBrowserContext);
  }, [setChips]);
  useEffect(() => {
    const pinned = selectedModelId ?? (isRoutingTierId(tier) ? null : tier);
    publishModelOverride(pinned);
  }, [selectedModelId, tier]);
  // True once the user has manually changed clutch/risk via DriveConsole —
  // guards the mount-time fetch below from clobbering that choice if it
  // resolves after the user has already interacted with the control.
  const userTouchedControlRef = useRef(false);

  // The hardcoded defaultControl() above is only a cold-start fallback — the
  // real default is whatever the backend policy resolver would pick for an
  // interactive chat task, so fetch that on mount and adopt it if it differs
  // (unless the user has already made their own choice in the meantime).
  useEffect(() => {
    invoke<{ clutch: ClutchId; risk: RiskId }>('resolve_default_task_policy', {
      category: 'Chat',
      source: 'interactive',
    })
      .then((resolved) => {
        // Guard against a malformed/empty IPC response (e.g. an unmocked
        // `invoke` in a test harness resolving to `null`) — only adopt a
        // well-formed result, otherwise keep the hardcoded fallback.
        if (!userTouchedControlRef.current && resolved?.clutch && resolved?.risk) {
          setControl(resolved);
        }
      })
      .catch(() => {
        // Backend unavailable (e.g. cold start) — keep the local hardcoded
        // default rather than blocking the composer on this fetch.
      });
  }, []);

  const [skillOpen, setSkillOpen] = useState(false);
  const [tierOpen,  setTierOpen]  = useState(false);
  // Explicit, user-visible choice between the synchronous "quick chat" reply
  // path and dispatching this as a background orchestrator task. Both go to
  // the same `chat_turn` command; this only picks `execution`.
  const [executionMode, setExecutionMode] = useState<'chat' | 'task' | 'plan'>('chat');

  const syncSessionPosture = (overrides: Partial<{
    clutch: ClutchId;
    risk: RiskId;
    tier: string;
    mode: ComposerInteractionMode;
    execution_mode: 'chat' | 'task' | 'plan';
    oversight_level: OversightLevelId;
    kb_scope: KbScopeMode;
  }>) => {
    const nextPosture = {
      clutch: overrides.clutch ?? control.clutch,
      risk: overrides.risk ?? control.risk,
      tier: overrides.tier ?? (isRoutingTierId(tier) ? tier : 'auto'),
      mode: overrides.mode ?? mode,
      execution_mode: overrides.execution_mode ?? executionMode,
      oversight_level: overrides.oversight_level ?? oversightLevel,
      kb_scope: overrides.kb_scope ?? kbScope,
    };
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent('vox:chat-posture-change', { detail: nextPosture }),
      );
      window.dispatchEvent(
        new CustomEvent('vox:session-posture-change', { detail: { posture: nextPosture } }),
      );
    }
    void Promise.resolve(
      invoke('set_session_chat_posture', {
        sessionId: null,
        posture: nextPosture,
      }),
    ).catch(() => {
      // Non-fatal in cold-start / unit test mocks.
    });
  };

  useEffect(() => {
    void Promise.resolve(
      invoke<{
        clutch?: ClutchId | null;
        risk?: RiskId | null;
        tier?: string | null;
        mode?: ComposerInteractionMode | null;
        execution_mode?: 'chat' | 'task' | 'plan' | null;
        oversight_level?: OversightLevelId | null;
        kb_scope?: KbScopeMode | null;
      }>('get_session_chat_posture', { sessionId: null }),
    )
      .then((saved) => {
        if (!saved || typeof saved !== 'object' || Array.isArray(saved)) return;
        if (!userTouchedControlRef.current && (saved.clutch || saved.risk)) {
          setControl((c) => ({
            ...c,
            ...(saved.clutch ? { clutch: saved.clutch } : {}),
            ...(saved.risk ? { risk: saved.risk } : {}),
          }));
        }
        if (saved.tier && isRoutingTierId(saved.tier)) setTier(saved.tier);
        if (saved.mode === 'plan' || saved.mode === 'act' || saved.mode === 'verify') {
          setMode(saved.mode);
        }
        if (
          saved.execution_mode === 'chat' ||
          saved.execution_mode === 'task' ||
          saved.execution_mode === 'plan'
        ) {
          setExecutionMode(saved.execution_mode);
        }
        if (saved.oversight_level && OVERSIGHT_CYCLE.includes(saved.oversight_level)) {
          setOversightLevel(saved.oversight_level);
        }
        if (saved.kb_scope && KB_SCOPE_CYCLE.includes(saved.kb_scope)) {
          setKbScope(saved.kb_scope);
        }
      })
      .catch(() => {});
  }, []);
  useEffect(() => {
    registerLoquelaDriveApi({
      setChatModelOverride: (id) => onModelPick?.(id),
      setTier,
      setClutch: (clutch) => {
        userTouchedControlRef.current = true;
        setControl(c => ({ ...c, clutch: clutch as ClutchId }));
      },
      setRisk: (risk) => {
        userTouchedControlRef.current = true;
        setControl(c => ({ ...c, risk: risk as RiskId }));
      },
      setMode: (nextMode: string) => {
        setMode(nextMode === 'plan' || nextMode === 'verify' ? nextMode : 'act');
      },
      setDryRun: () => undefined,
      setExecution: (execution: DriveExecution) => {
        setExecutionMode(execution === 'background' ? 'task' : execution === 'plan' ? 'plan' : 'chat');
      },
      setContextFiles: (files) => {
        setChips(files.map((label, i) => ({ id: `drive-${i}`, kind: 'file' as const, label })));
      },
    });
    return () => registerLoquelaDriveApi(null);
  }, [onModelPick, setChips]);
  const [modeOpen, setModeOpen] = useState(false);
  const [slashOpen, setSlashOpen] = useState(false);
  const [atOpen,    setAtOpen]    = useState(false);
  const [fileSuggestions, setFileSuggestions] = useState<string[]>([]);
  const [fileSuggestionsLoading, setFileSuggestionsLoading] = useState(false);
  const filePickerDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [slashIdx,  setSlashIdx]  = useState(0);
  const [focused,   setFocused]   = useState(false);
  const [history,   setHistory]   = useState<string[]>([]);
  const [histIdx,   setHistIdx]   = useState(-1);
  const promptHistoryRef = useRef<PromptHistoryController>(new PromptHistoryController());
  const [expanded,  setExpanded]  = useState(false);
  const [runtimeTiers, setRuntimeTiers] = useState(LQ_TIERS);
  const [modelLoadStatus, setModelLoadStatus] = useState<'idle' | 'loading' | 'ok' | 'empty' | 'error'>('idle');
  const [liveSearchTiers, setLiveSearchTiers] = useState<typeof LQ_TIERS | null>(null);
  const [liveSearchStatus, setLiveSearchStatus] = useState<'idle' | 'loading' | 'ok' | 'error'>('idle');
  const modelSearchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [intent, setIntent] = useState<IntentFields>(EMPTY_INTENT);
  const [intentOpen, setIntentOpen] = useState(false);
  const [keybindHudOpen, setKeybindHudOpen] = useState(false);
  const [postureOpen, setPostureOpen] = useState(false);

  const taRef = useRef<HTMLTextAreaElement>(null);
  const tierRootRef = useRef<HTMLDivElement>(null);
  const postureRootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!postureOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setPostureOpen(false);
    };
    const onPointerDown = (e: PointerEvent) => {
      if (!postureRootRef.current?.contains(e.target as Node)) setPostureOpen(false);
    };
    window.addEventListener('keydown', onKey);
    document.addEventListener('pointerdown', onPointerDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onPointerDown);
    };
  }, [postureOpen]);

  useEffect(() => {
    if (!tierOpen && !modeOpen && !skillOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setTierOpen(false);
        setModeOpen(false);
        setSkillOpen(false);
      }
    };
    const onPointerDown = (e: PointerEvent) => {
      if (tierOpen && !tierRootRef.current?.contains(e.target as Node)) setTierOpen(false);
    };
    window.addEventListener('keydown', onKey);
    document.addEventListener('pointerdown', onPointerDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onPointerDown);
    };
  }, [tierOpen, modeOpen, skillOpen]);

  useEffect(() => {
    const ta = taRef.current; if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = Math.min(expanded ? 360 : 200, ta.scrollHeight) + "px";
  }, [text, expanded]);

  useEffect(() => {
    const trimmed = text.trimStart();
    setSlashOpen(trimmed.startsWith("/") && !trimmed.includes(" "));
    const m = text.match(/@([^\s]*)$/);
    setAtOpen(!!m);
    setSlashIdx(0);
  }, [text]);

  const atQuery = useMemo(() => {
    const m = text.match(/@([^\s]*)$/);
    return m?.[1] ?? "";
  }, [text]);

  const isPathLikeAtQuery = (q: string) =>
    q.length === 0 || /[./\\-]/.test(q) || q.includes("/");

  useEffect(() => {
    if (!atOpen) {
      setFileSuggestions([]);
      setFileSuggestionsLoading(false);
      if (filePickerDebounceRef.current) clearTimeout(filePickerDebounceRef.current);
      return;
    }
    if (filePickerDebounceRef.current) clearTimeout(filePickerDebounceRef.current);
    filePickerDebounceRef.current = setTimeout(async () => {
      setFileSuggestionsLoading(true);
      try {
        const paths = await invoke<string[]>("list_repo_files", {
          query: atQuery || null,
          limit: LOQUELA_FILE_PICKER_LIMIT,
        });
        setFileSuggestions(Array.isArray(paths) ? paths : []);
      } catch {
        setFileSuggestions([]);
      } finally {
        setFileSuggestionsLoading(false);
      }
    }, LOQUELA_FILE_PICKER_DEBOUNCE_MS);
    return () => {
      if (filePickerDebounceRef.current) clearTimeout(filePickerDebounceRef.current);
    };
  }, [atOpen, atQuery]);

  // Refresh the model/tier list on focus + every 60s so it never goes stale (C2).
  useEffect(() => {
    let cancelled = false;
    const loadTiers = () => {
      setModelLoadStatus('loading');
      Promise.all([
        voxTransport.listModels(LOQUELA_TIER_MODEL_COUNT),
        invoke<ProviderStatus[]>('inference_provider_status').catch(() => []),
      ]).then(([models, statuses]) => {
        if (cancelled) return;
        if (!Array.isArray(models)) {
          setModelLoadStatus('error');
          return;
        }
        if (models.length === 0) {
          setRuntimeTiers(ROUTING_TIERS);
          setModelLoadStatus('empty');
          return;
        }
        const statusRows = Array.isArray(statuses) ? statuses : [];
        const available = filterPickerModels(
          models
            .map((m: { id?: string; model_id?: string; display_name?: string; provider?: string; provider_type?: string }) =>
              normalizeModelCard(m),
            )
            .filter((m): m is PickerModel => m != null),
          statusRows,
        );
        const dynamic = available.map(m => ({
          id: m.id,
          label: m.label,
          detail: m.providerType || m.provider || 'runtime',
          cost: null as number | null,
          lat: null as number | null,
        }));
        setRuntimeTiers([...ROUTING_TIERS, ...dynamic]);
        setModelLoadStatus('ok');
        if (!isRoutingTierId(tier) && !dynamic.some(d => d.id === tier)) {
          setTier('auto');
        }
      }).catch(() => {
        if (!cancelled) setModelLoadStatus('error');
      });
    };
    loadTiers();
    // Embedded mini-render: one initial load only — no 60s poll, no focus refresh.
    if (embedded) return () => { cancelled = true; };
    const interval = setInterval(loadTiers, 60_000);
    const onFocus = () => loadTiers();
    window.addEventListener('focus', onFocus);
    return () => {
      cancelled = true;
      clearInterval(interval);
      window.removeEventListener('focus', onFocus);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refresh model tiers on mount and focus
  }, [embedded]);

  // Live keyed-provider autocomplete (OpenRouter TTL + Anthropic/Google/Mens) —
  // does not rely on the disk model-catalog cache.
  useEffect(() => {
    if (!tierOpen) {
      setLiveSearchTiers(null);
      setLiveSearchStatus('idle');
      if (modelSearchDebounceRef.current) clearTimeout(modelSearchDebounceRef.current);
      return;
    }
    const q = tierQuery.trim();
    if (!q) {
      setLiveSearchTiers(null);
      setLiveSearchStatus('idle');
      return;
    }
    if (modelSearchDebounceRef.current) clearTimeout(modelSearchDebounceRef.current);
    modelSearchDebounceRef.current = setTimeout(() => {
      setLiveSearchStatus('loading');
      voxTransport
        .searchModels(q, LOQUELA_MODEL_SEARCH_LIMIT)
        .then((models: unknown) => {
          if (!Array.isArray(models)) {
            setLiveSearchStatus('error');
            return;
          }
          const rows = (models as Array<{
            id?: string;
            model_id?: string;
            display_name?: string;
            provider?: string;
            provider_type?: string;
          }>)
            .map(m => normalizeModelCard(m))
            .filter((m): m is PickerModel => m != null)
            .map(m => ({
              id: m.id,
              label: m.label,
              detail: m.providerType || m.provider || 'live',
              cost: null as number | null,
              lat: null as number | null,
            }));
          setLiveSearchTiers(rows);
          setLiveSearchStatus('ok');
        })
        .catch(() => {
          setLiveSearchTiers(null);
          setLiveSearchStatus('error');
        });
    }, LOQUELA_MODEL_SEARCH_DEBOUNCE_MS);
    return () => {
      if (modelSearchDebounceRef.current) clearTimeout(modelSearchDebounceRef.current);
    };
  }, [tierOpen, tierQuery]);

  const allSlash = useMemo(() => buildSlashEntries(skills), [skills]);
  const filteredSlash = useMemo(() => {
    const q = text.trimStart().toLowerCase();
    return allSlash.filter(s => s.cmd.startsWith(q));
  }, [text, allSlash]);

  const filteredAt = useMemo(() => {
    const q = atQuery.toLowerCase();
    return agents.filter(a => a.id.toLowerCase().includes(q) || a.codename?.toLowerCase().includes(q));
  }, [atQuery, agents]);

  const [termBridge, setTermBridge] = useState<TerminalBridgeState>(getTerminalBridgeState);
  useEffect(() => subscribeTerminalBridge(setTermBridge), []);

  useEffect(() => {
    return onChatAttention((chipOrPrompt) => {
      setChips((prev) => {
        const id = `term-${chipOrPrompt}-${Date.now()}`;
        if (prev.some((c) => c.label === chipOrPrompt)) return prev;
        return [...prev, { id, kind: 'term', label: chipOrPrompt }];
      });
    });
  }, [setChips]);

  const filteredTermOptions = useMemo(() => {
    const q = atQuery.toLowerCase();
    return TERM_MENTION_OPTIONS.filter(opt => opt.token.slice(1).startsWith(q) || opt.label.toLowerCase().includes(q));
  }, [atQuery]);

  const showFileSuggestions = atOpen && (isPathLikeAtQuery(atQuery) || fileSuggestions.length > 0);
  const showAtPopover = atOpen && (filteredTermOptions.length > 0 || filteredAt.length > 0 || showFileSuggestions || fileSuggestionsLoading);

  const [intellisenseForceOpen, setIntellisenseForceOpen] = useState(false);

  const intellisenseItems = useMemo(() => {
    if (!slashOpen && !atOpen && !intellisenseForceOpen) return [];
    return computeIntelliSenseItems({
      input: text,
      surface: 'chat',
      skills,
      repoFiles: fileSuggestions,
      agents,
      termBridge,
      forceOpen: intellisenseForceOpen,
    });
  }, [slashOpen, atOpen, intellisenseForceOpen, text, skills, fileSuggestions, agents, termBridge]);

  const activeIntelliSenseItem =
    intellisenseItems[Math.min(slashIdx, Math.max(0, intellisenseItems.length - 1))] ??
    intellisenseItems[0];
  const loquelaGhost = activeIntelliSenseItem?.ghostSuffix ?? '';

  const tokens = Math.ceil(text.length / 4) + chips.length * 80;
  const effectiveTierId =
    selectedModelId && !isRoutingTierId(selectedModelId) ? selectedModelId : tier;
  const tierObj =
    runtimeTiers.find(t => t.id === effectiveTierId) ||
    runtimeTiers.find(t => t.id === 'auto') ||
    runtimeTiers[runtimeTiers.length - 1];
  const visibleTiers = useMemo(() => {
    const q = tierQuery.trim().toLowerCase();
    if (q && liveSearchTiers !== null) {
      const routingHits = ROUTING_TIERS.filter(
        t =>
          t.id.toLowerCase().includes(q) ||
          t.label.toLowerCase().includes(q) ||
          (t.detail ?? '').toLowerCase().includes(q),
      );
      return [...routingHits, ...liveSearchTiers];
    }
    return runtimeTiers.filter(t => {
      if (!q) return true;
      return (
        t.id.toLowerCase().includes(q) ||
        t.label.toLowerCase().includes(q) ||
        (t.detail ?? '').toLowerCase().includes(q)
      );
    });
  }, [tierQuery, liveSearchTiers, runtimeTiers]);
  const triggerLabel = isRoutingTierId(tierObj.id)
    ? tierObj.label.split(' · ')[0]
    : shortModelLabel(tierObj.id);
  const estCost =
    tierObj?.cost == null ? null : (tokens / 1000) * tierObj.cost;

  const [recording, setRecording] = useState(false);
  const [transcribing, setTranscribing] = useState(false);

  // Add a single locator (file path or URL) to the shared context set as a chip.
  // These chips become the next task's file manifest (App.handleLoquelaSubmit).
  const addContextRef = (ref: string) => {
    const trimmed = ref.trim();
    if (!trimmed) return;
    const isUrl = /^https?:\/\//i.test(trimmed);
    const isTerm = trimmed.startsWith('@term');
    const kind = isTerm ? 'term' : isUrl ? 'url' : 'file';
    const id = `ctx-${kind}-${trimmed}`;
    setChips(cs => cs.find(c => c.id === id)
      ? cs
      : [...cs, { id, kind, label: trimmed }]);
  };

  // Attach local files to the task context via the native OS file-picker
  // (tauri-plugin-dialog). Each chosen path flows to the orchestrator as a
  // FileAffinity through the shared Loquela context set. Falls back to a typed
  // path/URL prompt when the dialog is unavailable (e.g. browser dev mode).
  const attachContext = async () => {
    try {
      const selected = await openFileDialog({ multiple: true, title: 'Attach files to task context' });
      const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
      if (paths.length) {
        paths.forEach(addContextRef);
        toast?.({ tone: 'ok', title: paths.length === 1 ? 'Attached to context' : `${paths.length} attached`, body: paths.join(', '), cmd: 'context.attach', cause: 'backend-ok' });
        taRef.current?.focus();
        return;
      }
      if (selected !== null) return; // dialog opened, user picked nothing
    } catch {
      // Dialog plugin unavailable → fall through to the prompt path.
    }
    const raw = window.prompt('Attach a file path or URL to this task context:');
    const ref = raw?.trim();
    if (!ref) return;
    addContextRef(ref);
    toast?.({ tone: 'ok', title: 'Attached to context', body: ref, cmd: 'context.attach', cause: 'backend-ok' });
    taRef.current?.focus();
  };

  // Attach a URL (the native picker only handles local files).
  const attachUrl = () => {
    const raw = window.prompt('Attach a URL to this task context:');
    const ref = raw?.trim();
    if (!ref) return;
    addContextRef(ref);
    toast?.({ tone: 'ok', title: 'Attached to context', body: ref, cmd: 'context.attach', cause: 'backend-ok' });
    taRef.current?.focus();
  };

  // Microphone capture → on-device transcription. Toggles record/stop; on stop,
  // appends the refined transcript into the composer textarea.
  const toggleMic = async () => {
    if (transcribing) return;
    if (!recording) {
      try {
        await invoke('start_mic_capture');
        setRecording(true);
        toast?.({ tone: 'info', title: 'Recording', body: 'Listening — tap the mic again to stop.', cmd: 'oratio.transcribe', cause: 'backend-ok' });
      } catch (e) {
        toast?.({ tone: 'warn', title: 'Microphone unavailable', body: sanitizeErrorForToast(e), cmd: 'oratio.transcribe', cause: 'backend-error' });
      }
      return;
    }
    // Stop + transcribe.
    setRecording(false);
    setTranscribing(true);
    try {
      const transcript = await invoke<string>('stop_mic_capture_and_transcribe');
      const t = (transcript || '').trim();
      if (t) {
        setText(prev => (prev ? `${prev.replace(/\s*$/, '')} ${t}` : t));
        taRef.current?.focus();
        toast?.({ tone: 'ok', title: 'Transcribed', body: t, cmd: 'oratio.transcribe', cause: 'backend-ok' });
      } else {
        toast?.({ tone: 'info', title: 'No speech detected', body: 'The recording produced no transcript.', cmd: 'oratio.transcribe', cause: 'backend-ok' });
      }
    } catch (e) {
      toast?.({ tone: 'warn', title: 'Transcription failed', body: sanitizeErrorForToast(e), cmd: 'oratio.transcribe', cause: 'backend-error' });
    } finally {
      setTranscribing(false);
    }
  };

  const runSlash = async (entry: SlashEntry) => {
    // Skill entries pin the skill (rides in the payload's active_skill) rather
    // than inserting literal text.
    if (entry.kind === 'skill') {
      const found = skills.find(
        (s) => (s.capability_id ?? s.command) === entry.skillId || s.command === entry.skillId,
      );
      const name = entry.cmd.slice(1);
      setActiveSkill(
        found
          ? { id: found.capability_id ?? found.command, name: found.command, command: found.command }
          : { id: entry.skillId ?? name, name, command: name },
      );
      setText('');
      setSlashOpen(false);
      setIntellisenseForceOpen(false);
      taRef.current?.focus();
      return;
    }
    const cmd = entry.cmd;
    // Internal mode slashes (/plan, /verify, …) flip the composer mode.
    const internalMode = resolveInternalModeSlash(cmd);
    if (internalMode) {
      setMode(internalMode === 'plan' || internalMode === 'verify' ? internalMode : 'act');
      setText('');
      setSlashOpen(false);
      setIntellisenseForceOpen(false);
      taRef.current?.focus();
      return;
    }
    const handled = await onSlashCommand?.(cmd, { setText });
    if (handled) {
      setText('');
      setSlashOpen(false);
      setIntellisenseForceOpen(false);
      taRef.current?.focus();
      return;
    }
    setText(cmd + ' ');
    setSlashOpen(false);
    setIntellisenseForceOpen(false);
    taRef.current?.focus();
  };
  const insertAt = (agent: ComposerAgentRef) => {
    setText(t => t.replace(/@[^\s]*$/, `@${agent.id} `));
    setChips(cs => cs.find(c => c.id === "agent-" + agent.id) ? cs : [...cs, { id: "agent-" + agent.id, kind: "agent", label: `${agent.id} · ${agent.codename ?? agent.id}`, meta: agent.phase }]);
    setAtOpen(false); setIntellisenseForceOpen(false); taRef.current?.focus();
  };

  const insertAtTerm = (token: string, desc?: string) => {
    setText(t => t.replace(/@[^\s]*$/, `${token} `));
    const chipId = `term-${token}`;
    setChips(cs =>
      cs.find(c => c.id === chipId)
        ? cs
        : [...cs, { id: chipId, kind: 'term', label: token, meta: desc ?? termBridge.activeSessionId ?? 'session' }],
    );
    setAtOpen(false);
    setIntellisenseForceOpen(false);
    taRef.current?.focus();
  };

  const insertAtFile = (path: string) => {
    setText(t => t.replace(/@[^\s]*$/, `@${path} `));
    addContextRef(path);
    setAtOpen(false);
    setIntellisenseForceOpen(false);
    taRef.current?.focus();
  };

  const acceptIntelliSenseItem = (item: IntelliSenseItem) => {
    if (item.slashEntry && item.label.startsWith('/')) {
      void runSlash({
        cmd: item.slashEntry.cmd,
        desc: item.slashEntry.desc,
        icon: 'bolt',
        kind: item.slashEntry.kind,
        skillId: item.slashEntry.skillId,
      });
      return;
    }
    if (item.category === 'term' && item.label.startsWith('@term')) {
      insertAtTerm(item.label, item.description);
      return;
    }
    if (item.chip) {
      const chipKind = (
        ['file', 'skill', 'agent', 'branch', 'url', 'image', 'term'].includes(item.chip.kind)
          ? item.chip.kind
          : 'skill'
      ) as ChipData['kind'];
      const chipId = `is-${chipKind}-${item.chip.label}`;
      setChips((cs) =>
        cs.some((c) => c.id === chipId || c.label === item.chip!.label)
          ? cs
          : [
              ...cs,
              {
                id: chipId,
                kind: chipKind,
                label: item.chip!.label,
                meta: item.chip!.meta,
              },
            ],
      );
      setText((t) =>
        /@[^\s]*$/.test(t) ? t.replace(/@[^\s]*$/, `${item.insertText} `) : `${t}${item.insertText} `,
      );
      setAtOpen(false);
      setSlashOpen(false);
      setIntellisenseForceOpen(false);
      taRef.current?.focus();
      return;
    }
    setText(item.insertText + ' ');
    setAtOpen(false);
    setSlashOpen(false);
    setIntellisenseForceOpen(false);
    taRef.current?.focus();
  };

  const canSend = !!text.trim() || !!intent.goal.trim();

  const send = async () => {
    if (!canSend) return;
    // A slash command typed and Entered directly (never opening/selecting
    // from the autocomplete dropdown) skips `runSlash` entirely -- that's
    // the ONLY place `onSlashCommand` was wired. Without this check, e.g.
    // "/spawn fix the login bug" (dropdown closes once trailing text no
    // longer looks like an active command search) fell through to a plain
    // chat submit with the literal "/spawn " prefix still in the text.
    if (isAppSlashCommand(text)) {
      const handled = await onSlashCommand?.(text.trim(), { setText });
      if (handled) {
        promptHistoryRef.current.pushSubmittedPrompt(text);
        setHistory(h => [text.trim(), ...h].slice(0, COMPOSER_HISTORY_CAP));
        setHistIdx(-1);
        setText('');
        setIntent(EMPTY_INTENT);
        setIntentOpen(false);
        return;
      }
    }
    const pickedModel = isRoutingTierId(effectiveTierId) ? null : effectiveTierId;
    let desc = composeDescription(text, intent);
    const extraContext: Array<{ kind: string; ref: string }> = [];
    if (text.includes('@term')) {
      extraContext.push({
        kind: 'terminal',
        ref: termBridge.activeSessionId ?? 'session',
      });
      const recentOutput = termBridge.blocks
        .slice(-2)
        .map((b) => `$ ${b.input}\n${b.output.map((o) => o.text).join('')}`)
        .join('\n');
      desc = desc.replace(
        /@term/g,
        `[Terminal Session: ${termBridge.activeSessionId ?? 'session'}\n${recentOutput}]`,
      );
    }

    const payload: ChatPayload = {
      description: desc,
      priority: effortToPriority(intent.effort),
      active_skill: activeSkill?.id,
      mode,
      tier: isRoutingTierId(effectiveTierId) ? effectiveTierId : 'auto',
      model_override: pickedModel,
      dry_run: false,
      clutch: control.clutch,
      risk: control.risk,
      oversight_level: oversightLevel,
      kb_scope: kbScope,
      context: [...chips.map(c => ({ kind: c.kind, ref: c.label })), ...extraContext],
      // Emitted explicitly for BOTH toggle positions: `buildChatTurn` maps it
      // to `execution`, and both go to the same `chat_turn` command. Never
      // derived from the absence of a sentinel -- the old
      // `task_category: 'chat' | undefined` encoding is gone.
      execution_mode: executionMode,
    } as ChatPayload & { context: Array<{ kind: string; ref: string }> };
    onSubmit(payload);
    promptHistoryRef.current.pushSubmittedPrompt(text);
    setHistory(h => [text.trim(), ...h].slice(0, COMPOSER_HISTORY_CAP));
    setHistIdx(-1);
    setText("");
    setIntent(EMPTY_INTENT);
    setIntentOpen(false);
  };

  const popoverVisible =
    (slashOpen || showAtPopover || intellisenseForceOpen) && intellisenseItems.length > 0;

  const onKey = (e: React.KeyboardEvent) => {
    if (e.altKey && !e.ctrlKey && !e.metaKey) {
      const k = e.key.toLowerCase();
      if (k === '1' || k === '2' || k === '3' || k === '4') {
        e.preventDefault();
        const pickedClutch = CLUTCH_CYCLE[Number(k) - 1];
        userTouchedControlRef.current = true;
        setControl((c) => ({ ...c, clutch: pickedClutch }));
        syncSessionPosture({ clutch: pickedClutch });
        return;
      }
      if (k === 'c') {
        e.preventDefault();
        const nextClutch =
          CLUTCH_CYCLE[(CLUTCH_CYCLE.indexOf(control.clutch) + 1) % CLUTCH_CYCLE.length];
        userTouchedControlRef.current = true;
        setControl((c) => ({ ...c, clutch: nextClutch }));
        syncSessionPosture({ clutch: nextClutch });
        return;
      }
      if (k === 'r') {
        e.preventDefault();
        const nextRisk =
          RISK_CYCLE[(RISK_CYCLE.indexOf(control.risk) + 1) % RISK_CYCLE.length];
        userTouchedControlRef.current = true;
        setControl((c) => ({ ...c, risk: nextRisk }));
        syncSessionPosture({ risk: nextRisk });
        return;
      }
      if (k === 't') {
        e.preventDefault();
        const curTier = isRoutingTierId(tier) ? (tier as (typeof TIER_CYCLE)[number]) : 'auto';
        const nextTier = TIER_CYCLE[(TIER_CYCLE.indexOf(curTier) + 1) % TIER_CYCLE.length];
        setTier(nextTier);
        onModelPick?.(null);
        syncSessionPosture({ tier: nextTier });
        return;
      }
      if (k === 'm') {
        e.preventDefault();
        const nextSend =
          SEND_MODE_CYCLE[
            (SEND_MODE_CYCLE.indexOf(executionMode) + 1) % SEND_MODE_CYCLE.length
          ];
        setExecutionMode(nextSend);
        syncSessionPosture({ execution_mode: nextSend });
        return;
      }
      if (k === 'v') {
        e.preventDefault();
        const nextMode: ComposerInteractionMode = mode === 'verify' ? 'act' : 'verify';
        setMode(nextMode);
        syncSessionPosture({ mode: nextMode });
        return;
      }
      if (k === 'o') {
        e.preventDefault();
        const nextOvs =
          OVERSIGHT_CYCLE[
            (OVERSIGHT_CYCLE.indexOf(oversightLevel) + 1) % OVERSIGHT_CYCLE.length
          ];
        setOversightLevel(nextOvs);
        syncSessionPosture({ oversight_level: nextOvs });
        return;
      }
      if (k === 'k') {
        e.preventDefault();
        const nextKb =
          KB_SCOPE_CYCLE[(KB_SCOPE_CYCLE.indexOf(kbScope) + 1) % KB_SCOPE_CYCLE.length];
        setKbScope(nextKb);
        syncSessionPosture({ kb_scope: nextKb });
        return;
      }
      if (k === '/' || k === '?') {
        e.preventDefault();
        setKeybindHudOpen((o) => !o);
        return;
      }
    }
    if (keybindHudOpen && e.key === 'Escape') {
      e.preventDefault();
      setKeybindHudOpen(false);
      return;
    }
    if (e.ctrlKey && e.key === ' ') {
      e.preventDefault();
      setIntellisenseForceOpen(true);
      return;
    }
    if (popoverVisible && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      e.preventDefault();
      setSlashIdx(i => (i + (e.key === "ArrowDown" ? 1 : -1) + intellisenseItems.length) % Math.max(1, intellisenseItems.length));
      return;
    }
    if (popoverVisible && e.key === "Tab" && activeIntelliSenseItem) {
      e.preventDefault();
      acceptIntelliSenseItem(activeIntelliSenseItem);
      return;
    }
    if (slashOpen && e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (activeIntelliSenseItem) {
        acceptIntelliSenseItem(activeIntelliSenseItem);
      } else {
        const s = filteredSlash[slashIdx];
        if (s) void runSlash(s);
      }
      return;
    }
    if ((atOpen || intellisenseForceOpen) && popoverVisible && e.key === "Enter" && !e.shiftKey && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      if (activeIntelliSenseItem) acceptIntelliSenseItem(activeIntelliSenseItem);
      return;
    }
    if ((slashOpen || atOpen || intellisenseForceOpen) && e.key === "Escape") {
      setSlashOpen(false);
      setAtOpen(false);
      setIntellisenseForceOpen(false);
      return;
    }
    if (e.key === "ArrowUp" && (!text || histIdx >= 0) && promptHistoryRef.current.size() > 0) {
      e.preventDefault();
      const nextVal = promptHistoryRef.current.navigateUp(text);
      const ni = Math.min(history.length - 1, histIdx + 1);
      setHistIdx(ni);
      setText(nextVal);
      return;
    }
    if (e.key === "ArrowDown" && histIdx >= 0) {
      e.preventDefault();
      const nextVal = promptHistoryRef.current.navigateDown(text);
      const ni = histIdx - 1;
      setHistIdx(ni);
      setText(nextVal);
      return;
    }
    if (e.key === "Enter" && e.altKey && taskInProgress && text.trim().length > 0) {
      e.preventDefault();
      void steerChatQueue({
        sessionId: 'default',
        prompt: text.trim(),
        promoteToSteer: true,
      });
      setText('');
      return;
    }
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      if (taskInProgress) { onInterrupt?.(currentTaskId); } else { void send(); }
      return;
    }
    if (e.key === "Enter" && !e.shiftKey && !slashOpen && !atOpen) {
      e.preventDefault();
      if (taskInProgress) { onInterrupt?.(currentTaskId); } else { void send(); }
    }
  };

  const hasFailedTermBlocks = termBridge.blocks.some(
    (b) => b.status === 'Failed' || (b.exit_code ?? 0) !== 0,
  );
  const hasPinnedTermBlocks = termBridge.pinnedBlockIds.length > 0;
  const hasActiveSession = Boolean(termBridge.activeSessionId);
  const latestExit =
    termBridge.blocks.length > 0
      ? termBridge.blocks[termBridge.blocks.length - 1].exit_code
      : null;

  return (
    <div className="pointer-events-auto" data-testid="loquela-composer">
      <Glass className={`relative px-3 py-2 transition ${focused ? "ring-1 ring-brass/30 shadow-lg" : ""}`}>
        {(chips.length > 0 || hasPinnedTermBlocks || hasFailedTermBlocks || hasActiveSession) && (
          <div className="flex flex-wrap items-center gap-1.5 pb-1.5">
            <span className="font-display text-[11px] uppercase tracking-[0.13em] text-text-muted">Context</span>
            {chips.map(c => (
              <Chip
                key={c.id}
                chip={c}
                onRemove={(x) => setChips(chips.filter(y => y.id !== x.id))}
                onEdit={(x, nextLabel) =>
                  setChips(chips.map(y => (y.id === x.id ? { ...y, label: nextLabel } : y)))
                }
              />
            ))}
            {(hasPinnedTermBlocks || hasFailedTermBlocks || hasActiveSession) && (
              <button
                type="button"
                data-testid="loquela-ambient-term-pill"
                onClick={() =>
                  insertAtTerm(
                    hasFailedTermBlocks
                      ? '@term:failed'
                      : hasPinnedTermBlocks
                        ? '@term:pinned'
                        : `@term:${termBridge.activeSessionId}`,
                    hasFailedTermBlocks
                      ? 'failed block'
                      : hasPinnedTermBlocks
                        ? `${termBridge.pinnedBlockIds.length} pinned`
                        : `session ${termBridge.activeSessionId}`,
                  )
                }
                className="inline-flex items-center gap-1 rounded-full border border-(--color-status-pass)/35 bg-(--color-status-pass)/10 px-2 py-0.5 font-mono text-[11px] text-(--color-status-pass) hover:bg-(--color-status-pass)/20"
              >
                +{' '}
                {hasFailedTermBlocks
                  ? '@term:failed'
                  : hasPinnedTermBlocks
                    ? `@term:pinned (${termBridge.pinnedBlockIds.length})`
                    : `term:${termBridge.activeSessionId} · ${termBridge.blocks.length} blocks${latestExit !== null && latestExit !== undefined ? ` · exit ${latestExit}` : ''}`}
              </button>
            )}
          </div>
        )}

        <div className="relative flex items-center gap-2">
          <div className="relative flex-1">
            {loquelaGhost && (
              <span
                aria-hidden="true"
                data-testid="loquela-ghost"
                className="pointer-events-none absolute right-2 top-2 font-mono text-[11px] text-text-muted"
              >
                {activeIntelliSenseItem?.label} ⇥
              </span>
            )}
            <textarea
              id="loquela-composer"
              ref={taRef}
              value={text}
              aria-label="Task composer"
              onChange={e => setText(e.target.value)}
              onKeyDown={onKey}
              onFocus={() => setFocused(true)}
              onBlur={() => setTimeout(() => setFocused(false), 120)}
              rows={1}
              placeholder="Describe a task — e.g. ‘harden cryptographic invariants’ (/ commands, @ context)"
              className={`min-h-[36px] ${expanded ? "max-h-[360px]" : "max-h-[160px]"} w-full resize-none bg-transparent py-1.5 text-[14px] leading-relaxed text-text-primary placeholder:text-text-muted outline-hidden`}
            />

            <IntelliSensePopover
              open={popoverVisible}
              items={intellisenseItems}
              activeIndex={slashIdx}
              onSelectIndex={setSlashIdx}
              onAcceptItem={acceptIntelliSenseItem}
              onClose={() => {
                setSlashOpen(false);
                setAtOpen(false);
                setIntellisenseForceOpen(false);
              }}
            />
          </div>

          {agentPaused && currentAgent && !taskInProgress && (
            <Tooltip content={`Paused agent (${currentAgent.id}) — click to resume`} side="top">
              <button
                type="button"
                data-testid="loquela-agent-paused-banner"
                onClick={() => onResume?.(currentAgent)}
                aria-label="Resume"
                className="inline-flex h-8 shrink-0 items-center gap-1.5 rounded-md border border-(--color-status-warn)/40 bg-(--color-status-warn)/10 px-2.5 text-(--color-status-warn) transition hover:bg-(--color-status-warn)/18"
              >
                <Icon.play className="size-3" />
                <span className="font-display text-[11px] uppercase tracking-[0.13em]">Resume</span>
              </button>
            </Tooltip>
          )}

          {taskInProgress ? (
            <div className="flex shrink-0 items-center gap-1.5">
              {text.trim().length > 0 && (
                <>
                  <button
                    type="button"
                    data-testid="loquela-steer-btn"
                    onClick={() => {
                      const promptText = text.trim();
                      if (!promptText) return;
                      void steerChatQueue({
                        sessionId: 'default',
                        prompt: promptText,
                        promoteToSteer: true,
                      });
                      setText('');
                    }}
                    aria-label="Steer active turn"
                    className="inline-flex h-9 shrink-0 items-center gap-1 rounded-md border border-brass/50 bg-brass/15 px-2.5 font-display text-[11px] uppercase tracking-[0.13em] text-brass transition hover:bg-brass/25"
                  >
                    ⚡ Steer
                  </button>
                  <button
                    type="button"
                    data-testid="loquela-queue-btn"
                    onClick={() => {
                      const promptText = text.trim();
                      if (!promptText) return;
                      void steerChatQueue({
                        sessionId: 'default',
                        prompt: promptText,
                        promoteToSteer: false,
                      });
                      setText('');
                    }}
                    aria-label="Queue follow-up turn"
                    className="inline-flex h-9 shrink-0 items-center gap-1 rounded-md border border-border-subtle bg-overlay-subtle px-2.5 font-display text-[11px] uppercase tracking-[0.13em] text-text-secondary transition hover:border-brass/40 hover:text-brass"
                  >
                    + Queue
                  </button>
                </>
              )}
              <button
                type="button"
                onClick={() => onInterrupt?.(currentTaskId)}
                aria-label="Stop (Enter)"
                className="inline-flex h-9 shrink-0 items-center gap-2 rounded-md border border-(--color-status-fail)/45 bg-(--color-status-fail)/12 px-3 text-(--color-status-fail) transition hover:bg-(--color-status-fail)/18"
              >
                <Icon.stop className="size-3.5" />
                <span className="font-display text-[11px] uppercase tracking-[0.13em]">Stop</span>
                <kbd className="rounded-sm border border-current px-1 text-[11px] opacity-75">↵</kbd>
              </button>
            </div>
          ) : (
            <Tooltip content="Run prompt (Enter; Shift+Enter for new line)" side="top">
              <button
                type="button"
                onClick={() => void send()}
                disabled={!canSend}
                aria-label="Run (Enter)"
                className={`inline-flex h-9 shrink-0 items-center gap-1.5 rounded-md border px-3.5 font-display text-[11px] font-semibold uppercase tracking-[0.13em] transition ${
                  canSend
                    ? 'border-brass bg-brass text-black hover:bg-brass/90'
                    : 'border-brass/65 bg-brass/22 text-brass cursor-not-allowed'
                }`}
              >
                <Icon.send className="size-3.5" />
                Run
                <kbd className="rounded-sm border border-current px-1 text-[11px] opacity-80">↵</kbd>
              </button>
            </Tooltip>
          )}
        </div>

        {intentOpen && (
          <IntentPanel intent={intent} onChange={(p) => setIntent((i) => ({ ...i, ...p }))} showEffort={executionMode === 'task'} />
        )}

        <div
          ref={postureRootRef}
          className="loquela-bar relative mt-1.5 flex flex-wrap items-center gap-1 border-t border-border-subtle/60 pt-1.5 text-[11px] whitespace-nowrap"
        >
          <div className="flex items-center gap-1">
            <Tooltip content="Attach local file(s) to context" side="top">
              <button type="button" data-loquela-toolbar-item="true" aria-label="Attach local file(s) to context" onClick={attachContext} className="flex size-7 shrink-0 items-center justify-center rounded-md border border-border-subtle bg-overlay-subtle text-text-muted hover:text-text-primary hover:border-brass/40 transition">
                <Icon.plus className="size-3.5" aria-hidden="true" />
              </button>
            </Tooltip>
            <Tooltip content="Attach a URL to context" side="top">
              <button type="button" data-loquela-toolbar-item="true" aria-label="Attach a URL to context" onClick={attachUrl} className="flex size-7 shrink-0 items-center justify-center rounded-md border border-border-subtle bg-overlay-subtle text-text-muted hover:text-text-primary hover:border-brass/40 transition">
                <Icon.link className="size-3.5" aria-hidden="true" />
              </button>
            </Tooltip>
            <Tooltip content={transcribing ? 'Transcribing…' : recording ? 'Stop recording & transcribe' : 'Voice input — record & transcribe'} side="top">
              <button
                type="button"
                data-loquela-toolbar-item="true"
                aria-label="Voice input"
                onClick={toggleMic}
                disabled={transcribing}
                aria-pressed={recording}
                className={`flex size-7 shrink-0 items-center justify-center rounded-md border transition ${
                  transcribing
                    ? 'border-border-subtle bg-overlay-subtle text-text-muted cursor-wait'
                    : recording
                    ? 'border-(--color-status-fail)/50 bg-(--color-status-fail)/15 text-(--color-status-fail) animate-pulse'
                    : 'border-border-subtle bg-overlay-subtle text-text-muted hover:text-text-primary hover:border-brass/40'
                }`}
              >
                <Icon.mic className="size-3.5" aria-hidden="true" />
              </button>
            </Tooltip>
          </div>

          <span className="h-4 w-px bg-border-subtle" aria-hidden="true" />

          {/* v4.0 Progressive Disclosure: Unified Execution Posture & Routing Inline Drawer */}
          <div className="contents">
            <Tooltip content="Execution Posture & Routing (Mode · Risk · Oversight · @kb · Research — Alt+/ for shortcuts)" side="top">
              <button
                type="button"
                data-testid="loquela-posture-trigger"
                data-cv-popover-trigger="loquela-posture"
                data-loquela-toolbar-item="true"
                aria-expanded={postureOpen}
                aria-label="Execution posture and routing settings"
                onClick={() => {
                  setPostureOpen((o) => !o);
                  setTierOpen(false);
                  setModeOpen(false);
                  setSkillOpen(false);
                  setKeybindHudOpen(false);
                }}
                className={`inline-flex h-7 items-center gap-1 rounded-full border px-2 font-medium transition ${
                  postureOpen
                    ? 'border-brass/60 bg-brass/20 text-brass shadow-sm'
                    : 'border-brass/35 bg-brass/10 text-brass hover:border-brass/55 hover:bg-brass/16'
                }`}
              >
                <Icon.bolt className="size-3 text-brass" aria-hidden="true" />
                <span>
                  {CLUTCH_DETENTS.find((d) => d.id === control.clutch)?.label ?? control.clutch} · {control.risk}
                </span>
                <Icon.chevR className="size-2.5 rotate-90 text-brass" aria-hidden="true" />
              </button>
            </Tooltip>

            <div
              data-testid="loquela-posture-popover"
              role="region"
              aria-label="Execution Posture & Routing"
              className={
                postureOpen
                  ? 'order-first mb-1.5 w-full basis-full space-y-2 rounded-xl border border-brass/40 bg-bg-elevated/95 p-3 text-[11px] whitespace-normal shadow-md'
                  : 'sr-only'
              }
            >
              <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2">
                <span className="font-display text-[11px] font-semibold uppercase tracking-[0.13em] text-brass">
                  Execution Posture & Routing
                </span>
                <div className="flex flex-wrap items-center gap-1.5">
                  <span
                    data-testid="loquela-live-posture-badge"
                    title="Active session posture — persists in real time across future chat turns, plans, and background tasks (Alt+1..4/C Mode · Alt+R Risk · Alt+T Tier · Alt+M Track · Alt+O Ovs · Alt+K @kb)"
                    className="inline-flex h-6 items-center gap-1.5 rounded-full border border-brass/30 bg-brass/10 px-2.5 font-mono text-[11px] text-brass"
                  >
                    <span className="size-1.5 rounded-full bg-(--color-status-pass)" aria-hidden="true" />
                    <span>
                      Live · {control.clutch}/{control.risk} · {isRoutingTierId(effectiveTierId) ? effectiveTierId : 'pinned'} · @{kbScope}
                    </span>
                  </span>

                  <div className="relative">
                    <button
                      type="button"
                      data-testid="loquela-keybind-hud-toggle"
                      data-keybind="Alt+/"
                      aria-keyshortcuts="Alt+/"
                      aria-expanded={keybindHudOpen}
                      aria-label="Session posture keyboard shortcuts"
                      onClick={() => setKeybindHudOpen((o) => !o)}
                      className="inline-flex h-6 items-center gap-1 rounded-full border border-border-subtle bg-overlay-subtle px-2 font-mono text-[11px] text-text-secondary transition hover:border-brass/50 hover:text-brass"
                    >
                      <span>Keys</span>
                      <span className="text-text-muted">Alt+/</span>
                    </button>
                    <Popover open={keybindHudOpen}>
                      <div
                        data-testid="loquela-keybind-hud-panel"
                        role="dialog"
                        aria-label="Real-time session posture shortcuts"
                        className="w-72 space-y-1.5 p-1 font-mono text-[11px]"
                      >
                        <div className="flex items-center justify-between border-b border-border-subtle pb-1 text-[11px] uppercase tracking-[0.08em] text-text-muted">
                          <span>Real-Time Session Knobs</span>
                          <span className="text-brass">Persisted</span>
                        </div>
                        <button
                          type="button"
                          data-testid="loquela-hud-cycle-clutch"
                          onClick={() => {
                            const nextClutch = CLUTCH_CYCLE[(CLUTCH_CYCLE.indexOf(control.clutch) + 1) % CLUTCH_CYCLE.length];
                            userTouchedControlRef.current = true;
                            setControl((c) => ({ ...c, clutch: nextClutch }));
                            syncSessionPosture({ clutch: nextClutch });
                          }}
                          className="flex w-full items-center justify-between rounded px-1.5 py-1 text-left hover:bg-overlay-subtle"
                        >
                          <span className="text-text-secondary">Mode (Alt+1..4 / Alt+C)</span>
                          <span className="rounded-full border border-brass/30 bg-brass/10 px-2 py-0.5 text-[11px] text-brass">{control.clutch}</span>
                        </button>
                        <button
                          type="button"
                          data-testid="loquela-hud-cycle-risk"
                          onClick={() => {
                            const nextRisk = RISK_CYCLE[(RISK_CYCLE.indexOf(control.risk) + 1) % RISK_CYCLE.length];
                            userTouchedControlRef.current = true;
                            setControl((c) => ({ ...c, risk: nextRisk }));
                            syncSessionPosture({ risk: nextRisk });
                          }}
                          className="flex w-full items-center justify-between rounded px-1.5 py-1 text-left hover:bg-overlay-subtle"
                        >
                          <span className="text-text-secondary">Risk Posture (Alt+R)</span>
                          <span className="rounded-full border border-brass/30 bg-brass/10 px-2 py-0.5 text-[11px] text-brass">{control.risk}</span>
                        </button>
                        <button
                          type="button"
                          data-testid="loquela-hud-cycle-tier"
                          onClick={() => {
                            const curTier = isRoutingTierId(tier) ? (tier as (typeof TIER_CYCLE)[number]) : 'auto';
                            const nextTier = TIER_CYCLE[(TIER_CYCLE.indexOf(curTier) + 1) % TIER_CYCLE.length];
                            setTier(nextTier);
                            onModelPick?.(null);
                            syncSessionPosture({ tier: nextTier });
                          }}
                          className="flex w-full items-center justify-between rounded px-1.5 py-1 text-left hover:bg-overlay-subtle"
                        >
                          <span className="text-text-secondary">Runtime Tier (Alt+T)</span>
                          <span className="rounded-full border border-brass/30 bg-brass/10 px-2 py-0.5 text-[11px] text-brass">{isRoutingTierId(effectiveTierId) ? effectiveTierId : 'pinned'}</span>
                        </button>
                        <button
                          type="button"
                          data-testid="loquela-hud-cycle-track"
                          onClick={() => {
                            const nextSend = SEND_MODE_CYCLE[(SEND_MODE_CYCLE.indexOf(executionMode) + 1) % SEND_MODE_CYCLE.length];
                            setExecutionMode(nextSend);
                            syncSessionPosture({ execution_mode: nextSend });
                          }}
                          className="flex w-full items-center justify-between rounded px-1.5 py-1 text-left hover:bg-overlay-subtle"
                        >
                          <span className="text-text-secondary">Send Track (Alt+M)</span>
                          <span className="rounded-full border border-brass/30 bg-brass/10 px-2 py-0.5 text-[11px] text-brass">{executionMode}</span>
                        </button>
                        <button
                          type="button"
                          data-testid="loquela-hud-cycle-oversight"
                          onClick={() => {
                            const nextOvs = OVERSIGHT_CYCLE[(OVERSIGHT_CYCLE.indexOf(oversightLevel) + 1) % OVERSIGHT_CYCLE.length];
                            setOversightLevel(nextOvs);
                            syncSessionPosture({ oversight_level: nextOvs });
                          }}
                          className="flex w-full items-center justify-between rounded px-1.5 py-1 text-left hover:bg-overlay-subtle"
                        >
                          <span className="text-text-secondary">Oversight (Alt+O)</span>
                          <span className="rounded-full border border-brass/30 bg-brass/10 px-2 py-0.5 text-[11px] text-brass">{oversightLevel}</span>
                        </button>
                        <button
                          type="button"
                          data-testid="loquela-hud-cycle-kb"
                          onClick={() => {
                            const nextKb = KB_SCOPE_CYCLE[(KB_SCOPE_CYCLE.indexOf(kbScope) + 1) % KB_SCOPE_CYCLE.length];
                            setKbScope(nextKb);
                            syncSessionPosture({ kb_scope: nextKb });
                          }}
                          className="flex w-full items-center justify-between rounded px-1.5 py-1 text-left hover:bg-overlay-subtle"
                        >
                          <span className="text-text-secondary">@kb Scope (Alt+K)</span>
                          <span className="rounded-full border border-brass/30 bg-brass/10 px-2 py-0.5 text-[11px] text-brass">@{kbScope}</span>
                        </button>
                      </div>
                    </Popover>
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-1 items-center gap-x-2.5 gap-y-2 pt-0.5 sm:grid-cols-[5.5rem_1fr]">
                <span className="font-mono text-[11px] uppercase tracking-[0.08em] text-text-muted">
                  Mode & Risk
                </span>
                <div className="flex min-w-0 flex-wrap items-center gap-1.5">
                  <DriveConsole
                    control={control}
                    onControlChange={(n) => {
                      userTouchedControlRef.current = true;
                      setControl((c) => {
                        const updated = { ...c, ...n };
                        syncSessionPosture({ clutch: updated.clutch, risk: updated.risk });
                        return updated;
                      });
                    }}
                    spentUsd={sessionBudget?.spent ?? 0}
                    budgetUsd={sessionBudget?.source === 'daemon' ? sessionBudget.cap : 0}
                    riskExtra={riskSlot}
                  />
                </div>

                <span className="font-mono text-[11px] uppercase tracking-[0.08em] text-text-muted">
                  Oversight & @kb
                </span>
                <div className="flex min-w-0 flex-wrap items-center gap-1.5">
                  <div className="relative inline-flex items-center">
                    <select
                      aria-label="Oversight level"
                      aria-keyshortcuts="Alt+O"
                      data-keybind="Alt+O"
                      title="Oversight posture (Alt+O to cycle)"
                      data-testid="loquela-oversight-select"
                      data-unified-knob="true"
                      value={oversightLevel}
                      onChange={(e) => {
                        const next = e.target.value as OversightLevelId;
                        setOversightLevel(next);
                        syncSessionPosture({ oversight_level: next });
                        invoke('invoke_mcp_tool', {
                          tool: 'orch.oversight_set',
                          args: { level: next },
                          permissionMode: null,
                        }).catch((err) => {
                          toast?.({
                            tone: 'warn',
                            title: 'Oversight sync deferred',
                            body: sanitizeErrorForToast(err),
                            cause: 'backend-error',
                          });
                        });
                      }}
                      className={`h-7 cursor-pointer appearance-none rounded-md border pl-2.5 pr-5 font-mono text-[11px] transition hover:brightness-110 focus:outline-hidden focus:ring-1 focus:ring-brass/40 ${OVERSIGHT_TONE[oversightLevel]}`}
                    >
                      <option value="relaxed">Ovs 1: Relaxed</option>
                      <option value="guided">Ovs 2: Guided</option>
                      <option value="balanced">Ovs 3: Balanced</option>
                      <option value="cautious">Ovs 4: Cautious</option>
                      <option value="strict">Ovs 5: Strict</option>
                    </select>
                    <Icon.chevR className="pointer-events-none absolute right-1.5 size-2.5 rotate-90 text-text-muted" aria-hidden="true" />
                  </div>

                  <div className="inline-flex h-7 items-center gap-1.5 rounded-md border border-border-subtle bg-overlay-subtle px-2">
                    <span className="font-mono text-[11px] text-text-muted">
                      Lv{OVERSIGHT_CYCLE.indexOf(oversightLevel) + 1}
                    </span>
                    <input
                      type="range"
                      min={1}
                      max={5}
                      step={1}
                      aria-label="Oversight slider"
                      data-testid="loquela-oversight-slider"
                      value={Math.max(1, OVERSIGHT_CYCLE.indexOf(oversightLevel) + 1)}
                      onChange={(e) => {
                        const idx = Math.max(0, Math.min(4, Number(e.target.value) - 1));
                        const next = OVERSIGHT_CYCLE[idx] ?? 'balanced';
                        setOversightLevel(next);
                        syncSessionPosture({ oversight_level: next });
                        invoke('invoke_mcp_tool', {
                          tool: 'orch.oversight_set',
                          args: { level: next },
                          permissionMode: null,
                        }).catch(() => {});
                      }}
                      className="h-1.5 w-16 cursor-pointer accent-brass"
                    />
                  </div>

                  <div className="relative inline-flex items-center">
                    <select
                      aria-label="KB context scope"
                      aria-keyshortcuts="Alt+K"
                      data-keybind="Alt+K"
                      title="@kb retrieval scope (Alt+K to cycle)"
                      data-testid="loquela-kb-scope-select"
                      data-unified-knob="true"
                      value={kbScope}
                      onChange={(e) => {
                        const nextKb = e.target.value as KbScopeMode;
                        setKbScope(nextKb);
                        syncSessionPosture({ kb_scope: nextKb });
                      }}
                      className={`h-7 cursor-pointer appearance-none rounded-md border pl-2.5 pr-5 font-mono text-[11px] transition hover:brightness-110 focus:outline-hidden focus:ring-1 focus:ring-brass/40 ${KB_SCOPE_TONE[kbScope]}`}
                    >
                      <option value="auto">@kb:auto</option>
                      <option value="docs">@kb:docs</option>
                      <option value="code">@kb:code</option>
                      <option value="memory">@kb:memory</option>
                      <option value="verified">@kb:verified</option>
                      <option value="off">@kb:off</option>
                    </select>
                    <Icon.chevR className="pointer-events-none absolute right-1.5 size-2.5 rotate-90 text-text-muted" aria-hidden="true" />
                  </div>
                </div>

                <span className="font-mono text-[11px] uppercase tracking-[0.08em] text-text-muted">
                  Skill & Checks
                </span>
                <div className="flex min-w-0 flex-wrap items-center gap-1.5">
                  <div className="relative">
                    <button
                      type="button"
                      aria-expanded={skillOpen}
                      aria-label="Choose skill"
                      onClick={() => {
                        setSkillOpen((o) => !o);
                        setTierOpen(false);
                        setModeOpen(false);
                      }}
                      className="inline-flex h-7 items-center gap-1 rounded-md border border-brass/30 bg-brass/10 px-2.5 text-brass hover:bg-brass/18"
                    >
                      <Icon.bolt className="size-3" />
                      <span className="text-brass">Skill</span>{' '}
                      <span>{activeSkill ? (activeSkill.name ?? activeSkill.command ?? activeSkill.id) : 'auto'}</span>
                      <Icon.chevR className="size-2.5 rotate-90 text-brass" />
                    </button>
                    <Popover open={skillOpen}>
                      <div className="max-h-64 overflow-y-auto overscroll-contain">
                        <button
                          type="button"
                          onClick={() => {
                            setActiveSkill(null);
                            setSkillOpen(false);
                          }}
                          className="block w-full rounded-sm px-2 py-1.5 text-left text-[11px] text-text-muted hover:bg-overlay-subtle hover:text-text-primary"
                        >
                          auto
                        </button>
                        {skills.map((s) => {
                          const skillId = s.capability_id ?? s.command;
                          return (
                            <button
                              type="button"
                              key={skillId}
                              onClick={() => {
                                setActiveSkill({ id: skillId, name: s.command, command: s.command });
                                setSkillOpen(false);
                              }}
                              className={`flex w-full items-center justify-between rounded-sm px-2 py-1.5 text-left text-[11px] hover:bg-overlay-subtle ${
                                activeSkill?.id === skillId ? 'bg-overlay-subtle text-brass' : 'text-text-secondary'
                              }`}
                            >
                              <span>{s.command}</span>
                            </button>
                          );
                        })}
                      </div>
                    </Popover>
                  </div>

                  <button
                    type="button"
                    aria-label="Structured intent"
                    aria-expanded={intentOpen}
                    onClick={() => setIntentOpen((o) => !o)}
                    className={`inline-flex h-7 items-center gap-1 rounded-md border px-2.5 transition ${
                      hasIntent(intent) || intentOpen
                        ? 'border-brass/40 bg-brass/12 text-brass'
                        : 'border-border-subtle bg-overlay-subtle text-text-secondary hover:border-white/20'
                    }`}
                  >
                    <Icon.list className="size-3" aria-hidden="true" /> Intent{hasIntent(intent) ? ' ·' : ''}
                  </button>

                  <button
                    type="button"
                    data-testid="loquela-auto-socrates-toggle"
                    aria-pressed={kbScope === 'verified'}
                    onClick={() => {
                      const nextKb: KbScopeMode = kbScope === 'verified' ? 'auto' : 'verified';
                      setKbScope(nextKb);
                      syncSessionPosture({ kb_scope: nextKb });
                    }}
                    className={`inline-flex h-7 items-center gap-2 rounded-md border px-2.5 font-mono text-[11px] transition ${
                      kbScope === 'verified'
                        ? 'border-(--color-status-pass)/50 bg-(--color-status-pass)/15 text-(--color-status-pass)'
                        : 'border-(--color-status-pass)/35 bg-(--color-status-pass)/10 text-(--color-status-pass) hover:border-(--color-status-pass)/60'
                    }`}
                  >
                    <span>Auto Socrates</span>
                    <span
                      className={`inline-flex h-3.5 w-6 items-center rounded-full p-0.5 transition ${
                        kbScope === 'verified' ? 'justify-end bg-(--color-status-pass)' : 'justify-end bg-(--color-status-pass)/80'
                      }`}
                      aria-hidden="true"
                    >
                      <span className="size-2.5 rounded-full bg-white shadow-xs" />
                    </span>
                  </button>

                  {trailingSlot}
                </div>
              </div>
            </div>
          </div>

          {typeof queueDepth === 'number' && queueDepth > 0 && (
            <button
              type="button"
              onClick={onOpenTasks}
              title="Open task list"
              className="inline-flex h-7 items-center gap-1 rounded-full border border-brass/25 bg-brass/10 px-2 font-mono text-[11px] text-brass hover:bg-brass/20 focus:outline-hidden focus:ring-1 focus:ring-brass/40"
            >
              {queueDepth} queued
            </button>
          )}

          <div className="relative flex items-center gap-1.5" ref={tierRootRef}>
            <Tooltip content="Select which model tier to run on (Alt+T to cycle)" side="top">
              <button
                type="button"
                data-loquela-toolbar-item="true"
                data-keybind="Alt+T"
                aria-keyshortcuts="Alt+T"
                aria-expanded={tierOpen}
                aria-label="Choose model tier"
                onClick={() => { setTierOpen(o => !o); setPostureOpen(false); setSkillOpen(false); setModeOpen(false); if (!tierOpen) setTierQuery(''); }}
                className={`inline-flex h-7 items-center gap-1 rounded-full border px-2.5 transition ${
                  TIER_BUTTON_TONE[isRoutingTierId(effectiveTierId) ? effectiveTierId : 'cloud'] ?? TIER_BUTTON_TONE.auto
                }`}
              >
                <Icon.cpu className="size-3 text-(--color-status-info)" /><span className="text-text-muted">Run on</span> <span className="text-text-primary">{triggerLabel}</span>
                <Icon.chevR className="size-2.5 text-text-muted rotate-90" />
              </button>
            </Tooltip>
            {modelLoadStatus === 'error' && (
              <span className="font-mono text-[11px] text-(--color-status-warn)" data-testid="model-load-status">Models unavailable</span>
            )}
            {modelLoadStatus === 'empty' && (
              <span className="font-mono text-[11px] text-text-muted" data-testid="model-load-status">No models loaded</span>
            )}
            <Popover open={tierOpen}>
              <div className="w-[min(22rem,80vw)]">
                <div className="sticky top-0 z-10 bg-bg-base">
                  <ModelPickerSearch value={tierQuery} onChange={setTierQuery} />
                </div>
                <div data-testid="model-picker-scroll" className="max-h-72 overflow-y-auto overscroll-contain custom-scrollbar">
                  {liveSearchStatus === 'loading' && tierQuery.trim() && (
                    <div className="px-2 py-1.5 font-mono text-[11px] text-text-muted" data-testid="model-search-status">
                      Searching live catalogs…
                    </div>
                  )}
                  {liveSearchStatus === 'error' && tierQuery.trim() && (
                    <div className="px-2 py-1.5 font-mono text-[11px] text-(--color-status-warn)" data-testid="model-search-status">
                      Live search failed — showing cached list
                    </div>
                  )}
                  {visibleTiers.map(t => (
                    <button type="button" key={t.id} onClick={() => {
                      setTier(t.id);
                      setTierOpen(false);
                      setTierQuery('');
                      onModelPick?.(isRoutingTierId(t.id) ? null : t.id);
                      if (isRoutingTierId(t.id)) {
                        syncSessionPosture({ tier: t.id });
                      }
                    }} className={`flex w-full items-start gap-2 rounded-sm px-2 py-1.5 text-left hover:bg-overlay-subtle ${effectiveTierId === t.id ? "bg-overlay-subtle" : ""}`}>
                      <div className="flex-1 min-w-0">
                        <div className="text-[11px] text-text-primary truncate">{isRoutingTierId(t.id) ? t.label : t.id}</div>
                        <div className="font-mono text-[11px] text-text-muted truncate">{t.detail}</div>
                      </div>
                    </button>
                  ))}
                  {visibleTiers.length === 0 && liveSearchStatus !== 'loading' && (
                    <div className="px-2 py-1.5 font-mono text-[11px] text-text-muted">No keyed models match</div>
                  )}
                </div>
              </div>
            </Popover>
          </div>

          <div className="relative">
            <Tooltip content="Choose send mode (Quick chat, Background task, Plan — Alt+M to cycle)" side="top">
              <button
                type="button"
                data-loquela-toolbar-item="true"
                data-keybind="Alt+M"
                aria-keyshortcuts="Alt+M"
                aria-expanded={modeOpen}
                aria-label="Choose send mode"
                onClick={() => { setModeOpen(o => !o); setPostureOpen(false); setTierOpen(false); setSkillOpen(false); }}
                className={`inline-flex h-7 items-center gap-1 rounded-full border px-2.5 transition ${SEND_MODE_TONE[executionMode]}`}
              >
                <Icon.bolt className="size-3" /><span>{SEND_MODES.find(m => m.id === executionMode)?.label}</span>
                <Icon.chevR className="size-2.5 text-text-muted rotate-90" />
              </button>
            </Tooltip>
            <Popover open={modeOpen}>
              {SEND_MODES.map(m => (
                <button key={m.id} type="button" aria-label={`Set send mode: ${m.label}`} onClick={() => { setExecutionMode(m.id); setModeOpen(false); syncSessionPosture({ execution_mode: m.id }); }} className={`flex w-full items-start gap-2 rounded-sm px-2 py-1.5 text-left hover:bg-overlay-subtle ${executionMode === m.id ? "bg-overlay-subtle" : ""}`}>
                  <div className="flex-1">
                    <div className="text-[11px] text-text-primary">{m.label}</div>
                    <div className="font-mono text-[11px] text-text-muted">{m.hint}</div>
                  </div>
                </button>
              ))}
            </Popover>
          </div>

          {executionMode === 'task' && (
            <span
              aria-label="Interaction mode"
              className="inline-flex h-7 items-center gap-1 rounded-md border border-border-subtle bg-overlay-subtle px-2 text-text-secondary"
            >
              {mode === 'verify' ? 'Verify' : 'Act'}
            </span>
          )}

          <button
            type="button"
            data-loquela-toolbar-item="true"
            onClick={() => setPostureOpen((o) => !o)}
            title="Active @kb retrieval scope (click to configure in Posture & Routing, or Alt+K to cycle)"
            className={`inline-flex h-7 items-center gap-1 rounded-full border px-2 font-mono text-[11px] transition ${KB_SCOPE_TONE[kbScope]}`}
          >
            <span>@kb:{kbScope}</span>
          </button>

          {Math.max(tokens, contextTokens) > 24_000 && (
            <button
              type="button"
              data-testid="loquela-compact-context-chip"
              onClick={() => void onSlashCommand?.('/compact', { setText })}
              title="Summarize and compact context window to save ~60–80% tokens on subsequent turns"
              className="inline-flex h-7 items-center gap-1 rounded-full border border-(--color-status-pass)/35 bg-(--color-status-pass)/10 px-2 font-mono text-[11px] text-(--color-status-pass) hover:bg-(--color-status-pass)/20"
            >
              Compact Context (~{Math.round(Math.max(tokens, contextTokens) / 1000)}k tok)
            </button>
          )}

          {estCost != null && (
            <div className="ml-auto flex items-center gap-2">
              <span className="font-mono text-[11px] text-text-muted tabular-nums" data-testid="loquela-cost-estimate">
                ~{tokens} tok · {estCost === 0 ? '$0.000 (Local · 0% markup)' : `~$${estCost.toFixed(3)} (0% markup)`}
              </span>
            </div>
          )}
        </div>
      </Glass>
    </div>
  );
}
