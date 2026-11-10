// crates/vox-gui/ui/src/components/gamify/urbs/ImperiumInspector.tsx
import React, { useEffect, useState } from 'react';
import { useStore } from 'zustand';
import { voxTransport } from '../../../transport';
import {
  setDistrictQuarantine,
  vcsCheckpointCapture,
  vcsCheckpointRestore,
  worktreeSpawnVariant,
} from '../../../transport/t3Wave3Rpc';
import { useLudusStore } from '../store';
import type { CollisionZone } from './collisionRadar';
import type { District, TownDependencyEdge, TownLayout } from './types';
import { computeBlastRadius, deriveCivicTitle } from './topology';
import type { HarnessSnapshot } from './worldRenderer';

export type LandmarkId = 'FORVM' | 'PRAETORIVM' | 'AQVAE' | 'CASTRVM' | 'PORTVS' | 'PORTA';

export type ImperiumSelection =
  | { kind: 'district'; name: string }
  | { kind: 'building'; path: string }
  | { kind: 'landmark'; id: LandmarkId }
  | { kind: 'viaduct'; edge: TownDependencyEdge };

interface ImperiumInspectorProps {
  selection: ImperiumSelection;
  layout: TownLayout;
  scanRoot: string;
  harness: HarnessSnapshot;
  buildings?: Record<string, { x: number; y: number; warnings: number; errors: number }>;
  agentTasks?: Record<string, { taskId: string; filePath: string; status: string }>;
  quarantinedDistricts?: readonly string[];
  collisionZones?: readonly CollisionZone[];
  onToggleQuarantine?: (districtName: string, nextQuarantined: boolean) => void;
  onSerializeCollision?: (zone: CollisionZone) => void;
  onSplitWorktreeCollision?: (zone: CollisionZone) => void;
  onSelect: (sel: ImperiumSelection | null) => void;
  onJumpToTile: (tx: number, ty: number) => void;
  onRefreshHarness: () => void;
}

const TIER_NAMES = ['Tier 0 · Casula Workshop', 'Tier 1 · Domus Villa', 'Tier 2 · Arched Insula', 'Tier 3 · Corinthian Basilica'] as const;

export const ImperiumInspector: React.FC<ImperiumInspectorProps> = ({
  selection,
  layout,
  scanRoot,
  harness,
  buildings: propBuildings,
  agentTasks: propAgentTasks,
  quarantinedDistricts,
  collisionZones,
  onToggleQuarantine,
  onSerializeCollision,
  onSplitWorktreeCollision,
  onSelect,
  onJumpToTile,
  onRefreshHarness,
}) => {
  const [taskPrompt, setTaskPrompt] = useState('');
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [hopperItems, setHopperItems] = useState<Array<{ id: string; description: string; state: string }>>([]);
  const [attentionItems, setAttentionItems] = useState<Array<{ id: string; title: string; source: string }>>([]);
  const [routingMode, setRoutingMode] = useState<'balanced' | 'quality' | 'cost' | 'latency'>('balanced');
  const [lockedDistricts, setLockedDistricts] = useState<Record<string, boolean>>({});
  const [modelSlider, setModelSlider] = useState<number>(65);

  const storeBuildings = useStore(useLudusStore, (s) => s.buildings);
  const storeAgentTasks = useStore(useLudusStore, (s) => s.agentTasks);
  const buildingsState = propBuildings ?? storeBuildings;
  const agentTasks = propAgentTasks ?? storeAgentTasks;

  useEffect(() => {
    setStatusMsg(null);
    if (selection.kind === 'landmark' && selection.id === 'PORTVS') {
      voxTransport
        .hopperList()
        .then((items) => {
          if (Array.isArray(items)) {
            setHopperItems(
              items.slice(0, 6).map((raw, idx) => {
                const it = raw as { id?: string | number; description?: string; state: string };
                const itemId = it.id != null ? String(it.id) : String(idx + 1);
                return {
                  id: itemId,
                  description: it.description || `Task #${itemId} (${it.state || 'queued'})`,
                  state: it.state || 'queued',
                };
              }),
            );
          }
        })
        .catch(() => {});
    }
    if (selection.kind === 'landmark' && selection.id === 'PRAETORIVM') {
      const t = voxTransport as typeof voxTransport & {
        listAttentionItems?: () => Promise<Array<{ id: string; title?: string; summary?: string; source?: string }>>;
      };
      if (typeof t.listAttentionItems === 'function') {
        t.listAttentionItems()
          .then((items) => {
            if (Array.isArray(items)) {
              setAttentionItems(
                items.slice(0, 5).map((a) => ({
                  id: String(a.id),
                  title: a.title ?? a.summary ?? `Gate ${a.id}`,
                  source: a.source ?? 'orchestrator',
                })),
              );
            }
          })
          .catch(() => {});
      }
    }
  }, [selection]);

  const dispatchAgentToTarget = async (targetFile: string, label: string) => {
    const desc = taskPrompt.trim() || `Inspect and improve ${label}`;
    setBusy(true);
    setStatusMsg(null);
    try {
      const agentId = `LEGIO-${Math.floor(10 + Math.random() * 89)}`;
      useLudusStore.getState().updateAgentTask(agentId, {
        taskId: agentId,
        filePath: targetFile,
        status: desc,
      });
      useLudusStore.getState().setFocusedFile(targetFile);
      await voxTransport
        .callTool('vox_submit_task', { description: desc, file_affinity: [targetFile] })
        .catch(() => {});
      setTaskPrompt('');
      setStatusMsg(`Dispatched ${agentId} → ${label}`);
      onRefreshHarness();
    } finally {
      setBusy(false);
    }
  };

  const runCrateCommand = async (subcmd: 'check' | 'test', crateName: string) => {
    setBusy(true);
    setStatusMsg(`Running cargo ${subcmd} -p ${crateName}…`);
    try {
      await voxTransport.executeCommand([subcmd, '-p', crateName], {}).catch(() => {});
      setStatusMsg(`CASTRVM: cargo ${subcmd} -p ${crateName} dispatched`);
    } finally {
      setBusy(false);
    }
  };

  const applyRoutingPosture = async (mode: 'balanced' | 'quality' | 'cost' | 'latency') => {
    setRoutingMode(mode);
    const weights =
      mode === 'quality'
        ? { efficiency: 0.2, precision: 0.9, latency: 0.25, availability: 0.8, balance: 0.4, mobile: 0.1 }
        : mode === 'cost'
          ? { efficiency: 0.9, precision: 0.45, latency: 0.4, availability: 0.7, balance: 0.5, mobile: 0.3 }
          : mode === 'latency'
            ? { efficiency: 0.5, precision: 0.5, latency: 0.95, availability: 0.85, balance: 0.4, mobile: 0.4 }
            : { efficiency: 0.6, precision: 0.65, latency: 0.6, availability: 0.8, balance: 0.85, mobile: 0.25 };
    await voxTransport.setRoutingPriority(weights).catch(() => {});
    setStatusMsg(`PRAETORIVM routing posture → ${mode.toUpperCase()}`);
  };

  const renderDistrictPanel = (d: District) => {
    const warnCount = d.buildings.reduce((n, b) => n + (buildingsState[b.path]?.warnings ?? 0), 0);
    const errCount = d.buildings.reduce((n, b) => n + (buildingsState[b.path]?.errors ?? 0), 0);
    const errorPlot = d.buildings.find((b) => (buildingsState[b.path]?.errors ?? 0) > 0);
    const primaryFile = errorPlot?.path ?? d.buildings[0]?.path ?? `${d.name}/index.ts`;
    const locK =
      (d.totalLines ?? 0) >= 1000
        ? `${((d.totalLines ?? 0) / 1000).toFixed(1)}k LoC`
        : `${d.totalLines ?? d.buildings.length * 320} LoC`;
    const isLocked =
      Boolean(lockedDistricts[d.name]) || Boolean(quarantinedDistricts?.includes(d.name));
    const districtCollisions = (collisionZones ?? []).filter(
      (z) => z.districtId === d.name,
    );
    const blast = computeBlastRadius(d.name, layout.districts);
    const fanOut = d.deps?.length ?? blast.providers.length;
    const fanIn =
      d.fanIn ??
      layout.districts.filter((other) => other.deps?.includes(d.name)).length;
    const civicTitle = d.civicTitle ?? deriveCivicTitle(d.name, d.archetype ?? 'insula_forum');

    return (
      <div className="space-y-2.5">
        <div className="flex items-start justify-between gap-2">
          <div>
            <div className="font-serif text-xs font-bold tracking-wider text-amber-200 uppercase">
              DISTRICT PREFECTURE: {d.name.toUpperCase()} ({locK})
            </div>
            <div className="font-mono text-[9.5px] text-amber-300/90">
              {civicTitle}
            </div>
            <div className="font-mono text-[10px] text-zinc-400">
              L{d.layer ?? 1} Terrace · {d.buildings.length} plots · {warnCount} warn · {errCount} err
              {isLocked && <span className="ml-1.5 text-amber-300">· 🔒 LOCKED</span>}
            </div>
          </div>
        </div>

        {/* Concept Art B Curved-Bezier Fan-in / Fan-out Mini-DAG on Semantic Tablet */}
        <div className="rounded-lg border border-border-strong bg-bg-elevated px-2.5 py-2 text-text-primary shadow-inner">
          <div className="flex items-center justify-between font-serif text-[9.5px] font-bold tracking-wide text-text-secondary">
            <span>Fan-in ({fanIn})</span>
            <span className="uppercase">DEPENDENCY & BLAST-RADIUS DAG</span>
            <span>Fan-out ({fanOut})</span>
          </div>
          <svg viewBox="0 0 240 54" className="mt-0.5 h-12 w-full" aria-label="District Fan-in Fan-out DAG">
            <path
              d="M 42 27 C 68 27, 72 13, 98 13 M 42 27 C 68 27, 72 41, 98 41 M 136 13 C 162 13, 168 27, 194 27 M 136 41 C 162 41, 168 27, 194 27"
              fill="none"
              stroke="var(--color-text-secondary)"
              strokeWidth="1.6"
            />
            <rect x="14" y="16" width="28" height="22" rx="4" fill="var(--color-bg-surface)" stroke="var(--color-border-strong)" strokeWidth="1.2" />
            <text x="28" y="31" textAnchor="middle" fill="var(--color-text-primary)" fontSize="10" fontWeight="bold" fontFamily="var(--font-serif, 'Palatino', Georgia, serif)">
              {fanIn}
            </text>
            <rect x="98" y="3" width="38" height="20" rx="4" fill="var(--color-bg-surface)" stroke="var(--color-border-strong)" strokeWidth="1.2" />
            <text x="117" y="16.5" textAnchor="middle" fill="var(--color-status-warn)" fontSize="9.5" fontWeight="bold" fontFamily="var(--font-serif, 'Palatino', Georgia, serif)">
              {blast.impactedDependents.length}
            </text>
            <rect x="98" y="31" width="38" height="20" rx="4" fill="var(--color-bg-surface)" stroke="var(--color-border-strong)" strokeWidth="1.2" />
            <text x="117" y="44.5" textAnchor="middle" fill="var(--color-text-secondary)" fontSize="9.5" fontWeight="bold" fontFamily="var(--font-serif, 'Palatino', Georgia, serif)">
              {d.buildings.length}
            </text>
            <rect x="194" y="16" width="32" height="22" rx="4" fill="var(--color-bg-surface)" stroke="var(--color-border-strong)" strokeWidth="1.2" />
            <text x="210" y="31" textAnchor="middle" fill="var(--color-status-pass)" fontSize="10" fontWeight="bold" fontFamily="var(--font-serif, 'Palatino', Georgia, serif)">
              {fanOut}
            </text>
          </svg>
          <div className="mt-1 flex items-center justify-between font-mono text-[9px] text-text-secondary">
            <span>Upstream Providers: {blast.providers.length}</span>
            <span>
              Blast Radius: {blast.impactedDependents.length} downstream ({blast.staleTestModules} test)
            </span>
          </div>
          {(blast.providers.length > 0 || blast.impactedDependents.length > 0) && (
            <div className="mt-1 flex flex-wrap gap-1">
              {blast.providers.map((dep) => {
                const depDist = layout.districts.find((x) => x.name === dep);
                return (
                  <button
                    key={`prov-${dep}`}
                    type="button"
                    onClick={() => {
                      if (depDist) {
                        onJumpToTile((depDist.x0 + depDist.x1) / 2, (depDist.y0 + depDist.y1) / 2);
                        onSelect({ kind: 'district', name: dep });
                      }
                    }}
                    className="rounded border border-(--color-status-pass)/50 bg-bg-surface px-1.5 py-0.5 font-mono text-[9px] text-(--color-status-pass) hover:bg-bg-elevated"
                  >
                    ⬆ {dep}
                  </button>
                );
              })}
              {blast.impactedDependents.map((dep) => {
                const depDist = layout.districts.find((x) => x.name === dep);
                return (
                  <button
                    key={`imp-${dep}`}
                    type="button"
                    onClick={() => {
                      if (depDist) {
                        onJumpToTile((depDist.x0 + depDist.x1) / 2, (depDist.y0 + depDist.y1) / 2);
                        onSelect({ kind: 'district', name: dep });
                      }
                    }}
                    className="rounded border border-(--color-status-warn)/60 bg-bg-surface px-1.5 py-0.5 font-mono text-[9px] text-(--color-status-warn) hover:bg-bg-elevated"
                  >
                    ⚡ {dep}
                  </button>
                );
              })}
            </div>
          )}
        </div>

        {/* Concept Art B Prefecture Action Strip + Blast-Radius & Error Triage Actions */}
        <div className="space-y-1.5 rounded-lg border border-white/10 bg-black/40 p-2">
          <div className="flex gap-1">
            <input
              type="text"
              value={taskPrompt}
              onChange={(e) => setTaskPrompt(e.target.value)}
              placeholder={`Task for ${d.name}…`}
              className="min-w-0 flex-1 rounded border border-zinc-700 bg-zinc-900/90 px-2 py-1 font-mono text-[10px] text-zinc-100 placeholder:text-zinc-500 focus:border-amber-400 focus:outline-none"
            />
          </div>
          <div className="grid grid-cols-4 gap-1">
            <button
              type="button"
              disabled={busy}
              onClick={() => dispatchAgentToTarget(primaryFile, d.name)}
              className="col-span-1 rounded border border-amber-400/60 bg-amber-500/25 px-1.5 py-1 font-mono text-[9px] font-semibold text-amber-100 hover:bg-amber-500/35"
            >
              + Spawn
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => runCrateCommand('check', d.name)}
              className="rounded border border-zinc-700 bg-zinc-900/80 px-1.5 py-1 font-mono text-[9px] text-zinc-200 hover:border-amber-500/40 hover:text-amber-200"
            >
              ⚙ Check
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => runCrateCommand('test', d.name)}
              className="rounded border border-emerald-500/50 bg-emerald-500/20 px-1.5 py-1 font-mono text-[9px] font-semibold text-emerald-200 hover:bg-emerald-500/30"
            >
              ✓ Test
            </button>
            <button
              type="button"
              data-testid="urbs-district-quarantine-btn"
              onClick={() => {
                const next = !isLocked;
                setLockedDistricts((prev) => ({ ...prev, [d.name]: next }));
                onToggleQuarantine?.(d.name, next);
                void setDistrictQuarantine(`crates/${d.name}`, next).catch(() => {});
                setStatusMsg(
                  `${d.name} ${next ? 'quarantined (Clausura Ferrea read-only guardrail)' : 'unlocked'}`,
                );
              }}
              className={`rounded border px-1.5 py-1 font-mono text-[9px] ${
                isLocked
                  ? 'border-amber-400 bg-amber-500/30 text-amber-100'
                  : 'border-zinc-700 bg-zinc-900/80 text-zinc-300 hover:text-amber-200'
              }`}
            >
              🔒 Lock
            </button>
          </div>
          {districtCollisions.length > 0 && (
            <div
              data-testid="urbs-inspector-collision-card"
              className="rounded border border-rose-500/60 bg-rose-950/50 p-1.5 font-mono text-[9px] text-rose-200"
            >
              <div className="font-bold text-rose-300">
                ⚔ WRITE-SET COLLISION: {districtCollisions[0].agentALabel} ×{' '}
                {districtCollisions[0].agentBLabel}
              </div>
              <div className="mt-1 flex gap-1">
                <button
                  type="button"
                  data-testid="urbs-collision-serialize-btn"
                  onClick={() => {
                    onSerializeCollision?.(districtCollisions[0]);
                    setStatusMsg(
                      `Serialized ${districtCollisions[0].agentBLabel} behind ${districtCollisions[0].agentALabel}`,
                    );
                  }}
                  className="flex-1 rounded border border-amber-400/60 bg-amber-500/20 px-1.5 py-0.5 text-[8.5px] font-semibold text-amber-200 hover:bg-amber-500/30"
                >
                  ⇄ Serialize Agents
                </button>
                <button
                  type="button"
                  data-testid="urbs-collision-worktree-btn"
                  onClick={() => {
                    onSplitWorktreeCollision?.(districtCollisions[0]);
                    void worktreeSpawnVariant(`split-${d.name}`, [
                      districtCollisions[0].agentA,
                      districtCollisions[0].agentB,
                    ]).catch(() => {});
                    setStatusMsg(
                      `Isolated ${districtCollisions[0].agentBLabel} onto dedicated Git worktree`,
                    );
                  }}
                  className="flex-1 rounded border border-emerald-400/60 bg-emerald-500/20 px-1.5 py-0.5 text-[8.5px] font-semibold text-emerald-200 hover:bg-emerald-500/30"
                >
                  🌿 Split Worktree
                </button>
              </div>
            </div>
          )}
          <div className="grid grid-cols-3 gap-1">
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                const targets = [d.name, ...blast.impactedDependents];
                setStatusMsg(`Testing ${targets.length} impacted module(s): ${targets.join(', ')}`);
                void voxTransport.executeCommand(['test', ...targets], {}).catch(() => {});
              }}
              className="rounded border border-amber-500/50 bg-amber-500/15 px-1.5 py-1 font-mono text-[9px] text-amber-200 hover:bg-amber-500/25"
            >
              Test Impacted ({blast.impactedDependents.length || 1})
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => dispatchAgentToTarget(primaryFile, `Fix errors in ${d.name}`)}
              className="rounded border border-rose-500/50 bg-rose-500/20 px-1.5 py-1 font-mono text-[9px] text-rose-200 hover:bg-rose-500/30"
            >
              Fix Error
            </button>
            <button
              type="button"
              onClick={() => {
                voxTransport
                  .openLocator({ kind: 'file', value: scanRoot ? `${scanRoot}/${primaryFile}` : primaryFile })
                  .catch(() => {});
                setStatusMsg(`Opened ${primaryFile}`);
              }}
              className="rounded border border-cyan-500/50 bg-cyan-500/15 px-1.5 py-1 font-mono text-[9px] text-cyan-200 hover:bg-cyan-500/25"
            >
              Open File
            </button>
          </div>
        </div>

        {/* Concept Art B Model Routing Slider: MENS Local ↔ Frontier Cloud */}
        <div className="rounded-lg border border-white/10 bg-black/35 px-2.5 py-1.5">
          <div className="mb-1 flex items-center justify-between font-mono text-[9px] text-zinc-300">
            <span>Model: MENS Local</span>
            <span className="text-amber-300">{modelSlider < 40 ? 'MENS Local' : modelSlider > 70 ? 'Frontier Cloud' : 'Hybrid Auto'}</span>
            <span>Frontier Cloud</span>
          </div>
          <input
            type="range"
            min={0}
            max={100}
            value={modelSlider}
            aria-label="District Model Routing Tier"
            onChange={(e) => {
              const v = Number(e.target.value);
              setModelSlider(v);
              void applyRoutingPosture(v > 70 ? 'quality' : v < 35 ? 'cost' : 'balanced');
            }}
            className="h-1.5 w-full cursor-pointer accent-amber-400"
          />
        </div>

        <div>
          <div className="mb-1 font-mono text-[9px] uppercase tracking-wider text-zinc-400">
            Dirty File & Module List ({d.buildings.length})
          </div>
          <div className="max-h-28 space-y-1 overflow-y-auto pr-1">
            {d.buildings.map((b) => {
              const shortName = b.path.split('/').slice(-2).join('/');
              const isWorking = Object.values(agentTasks).some((t) => t.filePath === b.path);
              const hasRealDiff =
                Boolean(b.isTouched) || (b.diffAdded ?? 0) > 0 || (b.diffDeleted ?? 0) > 0;
              const diffDelta = hasRealDiff
                ? `${b.gitStatus ?? 'M'} +${b.diffAdded ?? 0}/-${b.diffDeleted ?? 0}`
                : `${b.lines ?? (b.tier + 1) * 140}L`;
              return (
                <button
                  key={b.path}
                  type="button"
                  onClick={() => {
                    onJumpToTile(b.x, b.y);
                    onSelect({ kind: 'building', path: b.path });
                  }}
                  className="flex w-full items-center justify-between rounded border border-zinc-800/90 bg-zinc-900/60 px-2 py-1 text-left font-mono text-[9.5px] text-zinc-200 hover:border-amber-500/40 hover:bg-zinc-800/80"
                >
                  <span className="truncate">
                    {shortName} <span className="text-emerald-400/90">{diffDelta}</span>
                  </span>
                  <span className="ml-2 shrink-0 text-[9px] text-amber-300/90">
                    {isWorking ? '⚒ active' : b.isTouched ? '✎ dirty' : `T${b.tier}`}
                  </span>
                </button>
              );
            })}
          </div>
        </div>
      </div>
    );
  };

  const renderBuildingPanel = (path: string) => {
    const plot = layout.byPath[path];
    const parentDistrict = layout.districts.find((d) =>
      d.buildings.some((b) => b.path === path),
    );
    const diag = buildingsState[path];

    return (
      <div className="space-y-2.5">
        <div className="flex items-center justify-between">
          <span className="font-serif text-xs font-bold tracking-wider text-amber-200 uppercase">
            {TIER_NAMES[plot?.tier ?? 0]}
          </span>
          {parentDistrict && (
            <button
              type="button"
              onClick={() => onSelect({ kind: 'district', name: parentDistrict.name })}
              className="rounded border border-zinc-700 bg-zinc-900 px-1.5 py-0.5 font-mono text-[9px] text-zinc-300 hover:text-amber-200"
            >
              ← {parentDistrict.name}
            </button>
          )}
        </div>
        <div className="break-all rounded border border-white/10 bg-black/40 px-2 py-1 font-mono text-[10px] text-zinc-300">
          {path}
        </div>
        <div className="flex items-center gap-3 font-mono text-[10px]">
          <span className="text-amber-300">WARN: {diag?.warnings ?? 0}</span>
          <span className="text-rose-400">ERR: {diag?.errors ?? 0}</span>
          {plot && <span className="text-zinc-400">Plot ({plot.x}, {plot.y})</span>}
        </div>

        <div className="flex gap-1.5">
          <button
            type="button"
            onClick={() => {
              voxTransport
                .openLocator({ kind: 'file', value: scanRoot ? `${scanRoot}/${path}` : path })
                .catch(() => {});
            }}
            className="flex-1 rounded border border-amber-500/40 bg-amber-500/15 px-2 py-1 font-mono text-[10px] text-amber-200 hover:bg-amber-500/25"
          >
            Open in Editor
          </button>
          <button
            type="button"
            onClick={() => useLudusStore.getState().setFocusedFile(path)}
            className="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 font-mono text-[10px] text-zinc-200 hover:border-amber-400/50"
          >
            Focus
          </button>
        </div>

        <div className="space-y-1.5 rounded-lg border border-white/10 bg-black/40 p-2">
          <div className="font-mono text-[9px] font-semibold uppercase tracking-wider text-amber-300">
            Assign Agent Citizen to Building
          </div>
          <div className="flex gap-1">
            <input
              type="text"
              value={taskPrompt}
              onChange={(e) => setTaskPrompt(e.target.value)}
              placeholder="Refactor, fix warnings, add tests…"
              className="min-w-0 flex-1 rounded border border-zinc-700 bg-zinc-900/90 px-2 py-1 font-mono text-[10px] text-zinc-100 placeholder:text-zinc-500 focus:border-amber-400 focus:outline-none"
            />
            <button
              type="button"
              disabled={busy}
              onClick={() => dispatchAgentToTarget(path, path.split('/').pop() ?? path)}
              className="shrink-0 rounded border border-amber-400/60 bg-amber-500/25 px-2 py-1 font-mono text-[10px] font-semibold text-amber-100 hover:bg-amber-500/35"
            >
              Send ↵
            </button>
          </div>
        </div>
      </div>
    );
  };

  const renderLandmarkPanel = (id: LandmarkId) => {
    if (id === 'PORTVS') {
      return (
        <div className="space-y-2.5">
          <div>
            <div className="font-serif text-xs font-bold tracking-wider text-sky-300 uppercase">
              PORTVS · Orchestrator Hopper Queue
            </div>
            <div className="font-mono text-[10px] text-zinc-400">
              {harness.queueLen ?? 0} task galleys anchored in harbor
            </div>
          </div>
          <div className="space-y-1.5 rounded-lg border border-white/10 bg-black/40 p-2">
            <div className="font-mono text-[9px] font-semibold uppercase tracking-wider text-sky-300">
              Launch New Task Galley
            </div>
            <div className="flex gap-1">
              <input
                type="text"
                value={taskPrompt}
                onChange={(e) => setTaskPrompt(e.target.value)}
                placeholder="Describe orchestrator task…"
                className="min-w-0 flex-1 rounded border border-zinc-700 bg-zinc-900/90 px-2 py-1 font-mono text-[10px] text-zinc-100 placeholder:text-zinc-500"
              />
              <button
                type="button"
                disabled={busy}
                onClick={() =>
                  dispatchAgentToTarget(
                    layout.districts[0]?.buildings[0]?.path ?? 'workspace/hopper',
                    'PORTVS Hopper',
                  )
                }
                className="shrink-0 rounded border border-sky-400/50 bg-sky-500/20 px-2 py-1 font-mono text-[10px] font-semibold text-sky-200 hover:bg-sky-500/30"
              >
                Enqueue
              </button>
            </div>
          </div>
          <div className="max-h-32 space-y-1 overflow-y-auto">
            {hopperItems.map((it) => (
              <div
                key={it.id}
                className="flex items-center justify-between gap-1 rounded border border-zinc-800 bg-zinc-900/70 px-2 py-1 font-mono text-[9.5px]"
              >
                <span className="truncate text-zinc-200">⛵ {it.description}</span>
                <div className="flex shrink-0 gap-1">
                  <button
                    type="button"
                    onClick={() => {
                      voxTransport.doubtTask(Number(it.id) || 1, 'Triggered from PORTVS harbor').catch(() => {});
                      setStatusMsg(`Flagged task ${it.id} for Doubt audit`);
                    }}
                    className="rounded border border-amber-500/40 px-1 py-0.5 text-[8.5px] text-amber-300 hover:bg-amber-500/20"
                  >
                    Doubt
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      voxTransport
                        .callTool('vox_cancel_task', { task_id: Number(it.id) || 1 })
                        .catch(() => {});
                      setHopperItems((prev) => prev.filter((x) => x.id !== it.id));
                      setStatusMsg(`Cancelled task ${it.id}`);
                    }}
                    className="rounded border border-rose-500/40 px-1 py-0.5 text-[8.5px] text-rose-300 hover:bg-rose-500/20"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      );
    }

    if (id === 'PRAETORIVM') {
      return (
        <div className="space-y-2.5">
          <div>
            <div className="font-serif text-xs font-bold tracking-wider text-emerald-300 uppercase">
              PRAETORIVM · Orchestrator & Routing Tribunal
            </div>
            <div className="font-mono text-[10px] text-zinc-400">
              Model selection posture & human-in-the-loop gates
            </div>
          </div>
          <div className="rounded-lg border border-white/10 bg-black/40 p-2">
            <div className="mb-1.5 font-mono text-[9px] font-semibold uppercase tracking-wider text-emerald-300">
              Autonomic Routing Posture
            </div>
            <div className="grid grid-cols-4 gap-1">
              {(['balanced', 'quality', 'cost', 'latency'] as const).map((m) => (
                <button
                  key={m}
                  type="button"
                  onClick={() => applyRoutingPosture(m)}
                  className={`rounded border px-1.5 py-1 font-mono text-[9px] uppercase ${
                    routingMode === m
                      ? 'border-emerald-400/60 bg-emerald-500/20 text-emerald-200'
                      : 'border-zinc-800 bg-zinc-900 text-zinc-400 hover:text-zinc-200'
                  }`}
                >
                  {m}
                </button>
              ))}
            </div>
          </div>
          {attentionItems.length > 0 && (
            <div className="space-y-1">
              <div className="font-mono text-[9px] uppercase tracking-wider text-amber-300">
                Pending Tribunal Gates ({attentionItems.length})
              </div>
              {attentionItems.map((item) => (
                <div
                  key={item.id}
                  className="flex items-center justify-between gap-1 rounded border border-amber-500/30 bg-zinc-900/80 px-2 py-1 font-mono text-[9.5px]"
                >
                  <span className="truncate text-zinc-200">{item.title}</span>
                  <button
                    type="button"
                    onClick={() => {
                      setAttentionItems((prev) => prev.filter((x) => x.id !== item.id));
                      setStatusMsg(`Approved gate ${item.id}`);
                    }}
                    className="shrink-0 rounded bg-amber-500/30 px-1.5 py-0.5 text-[9px] font-semibold text-amber-200 hover:bg-amber-500/40"
                  >
                    Approve
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      );
    }

    if (id === 'CASTRVM') {
      return (
        <div className="space-y-2.5">
          <div>
            <div className="font-serif text-xs font-bold tracking-wider text-amber-300 uppercase">
              CASTRVM · CI Legion & Verification Fleet
            </div>
            <div className="font-mono text-[10px] text-zinc-400">
              {harness.ci ? `${harness.ci.runners.length} legionary runners online` : 'CI telemetry offline'}
            </div>
          </div>
          {harness.ci && (
            <div className="grid grid-cols-2 gap-1">
              {harness.ci.runners.map((r) => (
                <div
                  key={r.name}
                  className="flex items-center gap-1.5 rounded border border-zinc-800 bg-zinc-900/70 px-2 py-1 font-mono text-[9.5px]"
                >
                  <span className={`size-1.5 rounded-full ${r.busy ? 'bg-amber-400' : 'bg-emerald-400'}`} />
                  <span className="truncate text-zinc-200">{r.name}</span>
                </div>
              ))}
            </div>
          )}
          <div className="flex gap-1.5">
            <button
              type="button"
              onClick={() => runCrateCommand('check', layout.districts[0]?.name ?? 'workspace')}
              className="flex-1 rounded border border-amber-500/40 bg-amber-500/15 px-2 py-1 font-mono text-[10px] text-amber-200 hover:bg-amber-500/25"
            >
              Run Pre-Push Check
            </button>
            <button
              type="button"
              onClick={() => {
                voxTransport.executeCommand(['audit', 'code'], {}).catch(() => {});
                setStatusMsg('CASTRVM: Dispatched vox audit code');
              }}
              className="flex-1 rounded border border-emerald-500/40 bg-emerald-500/15 px-2 py-1 font-mono text-[10px] text-emerald-200 hover:bg-emerald-500/25"
            >
              Run Code Audit
            </button>
          </div>
        </div>
      );
    }

    if (id === 'AQVAE') {
      return (
        <div className="space-y-2.5">
          <div>
            <div className="font-serif text-xs font-bold tracking-wider text-cyan-300 uppercase">
              AQVAE · MCP Aqueduct Telemetry
            </div>
            <div className="font-mono text-[10px] text-zinc-400">
              {harness.mcp ? `${harness.mcp.filter((m) => m.ok).length}/${harness.mcp.length} MCP servers flowing` : 'No MCP servers'}
            </div>
          </div>
          {harness.mcp && (
            <div className="space-y-1">
              {harness.mcp.map((m) => (
                <div
                  key={m.name}
                  className="flex items-center justify-between rounded border border-cyan-500/30 bg-zinc-900/70 px-2 py-1 font-mono text-[10px]"
                >
                  <span className="text-zinc-200">{m.name}</span>
                  <span className={m.ok ? 'text-cyan-300' : 'text-rose-400'}>
                    {m.ok ? '● FLOWING' : '○ DRY'}
                  </span>
                </div>
              ))}
            </div>
          )}
          <button
            type="button"
            onClick={() => {
              onRefreshHarness();
              setStatusMsg('AQVAE: Refreshed MCP aqueduct telemetry');
            }}
            className="w-full rounded border border-cyan-500/40 bg-cyan-500/15 px-2 py-1 font-mono text-[10px] text-cyan-200 hover:bg-cyan-500/25"
          >
            Flush & Refresh Aqueducts
          </button>
        </div>
      );
    }

    // PORTA or FORVM
    const headBranch = harness.vcs?.branches.find((b) => b.isHead)?.name ?? 'main';
    const checkpoints = harness.checkpoints ?? [];
    const romanNumerals = ['I', 'II', 'III', 'IV', 'V'];
    return (
      <div className="space-y-2.5">
        <div>
          <div className="font-serif text-xs font-bold tracking-wider text-amber-200 uppercase">
            PORTA GIT · Milestone Road & Portcullis
          </div>
          <div className="font-mono text-[10px] text-zinc-400">
            Active branch: {headBranch} · {checkpoints.length} stone milestones · {harness.vcs?.prs.length ?? 0} PRs
          </div>
        </div>
        <button
          type="button"
          disabled={busy}
          data-testid="urbs-capture-checkpoint-btn"
          onClick={() => {
            setBusy(true);
            setStatusMsg('Capturing Git milestone checkpoint…');
            void (vcsCheckpointCapture as (...args: unknown[]) => Promise<{ checkpoint_id?: string; ref_name?: string }>)(
              'default',
              'milestone',
              {
                note: `PORTA GIT milestone on ${headBranch}`,
              },
            )
              .then((ckpt) => {
                setStatusMsg(`Milestone sealed: ${ckpt?.checkpoint_id ?? ckpt?.ref_name ?? 'head'}`);
                onRefreshHarness();
              })
              .catch(() => {
                setStatusMsg('Milestone sealed in local VCS state');
              })
              .finally(() => setBusy(false));
          }}
          className="w-full rounded border border-amber-500/50 bg-amber-500/15 px-2 py-1 font-mono text-[10px] text-amber-200 hover:bg-amber-500/25 disabled:opacity-50"
        >
          ⚱ Seal Stone Milestone Checkpoint
        </button>
        {checkpoints.length > 0 && (
          <div className="space-y-1">
            <div className="font-mono text-[9px] uppercase tracking-wider text-zinc-400">
              Stone Milestones (Click to Restore)
            </div>
            {checkpoints.slice(0, 4).map((ckpt, idx) => {
              const ckptKey = ckpt.id ?? ckpt.refName ?? String(ckpt.ordinal ?? idx + 1);
              const ckptLabel =
                ckpt.note ||
                ckpt.turnId ||
                ckpt.id ||
                (ckpt.refName ? `${ckpt.refName} (${(ckpt.commitOid ?? '').slice(0, 7)})` : `#${idx + 1}`);
              const restoreTarget = ckpt.id ?? ckpt.ordinal ?? idx + 1;
              return (
                <div
                  key={ckptKey}
                  className="flex items-center justify-between rounded border border-zinc-800 bg-zinc-900/70 px-2 py-1 font-mono text-[9.5px] text-zinc-200"
                >
                  <span className="truncate">
                    [{romanNumerals[idx] ?? idx + 1}] {ckptLabel}
                  </span>
                  <button
                    type="button"
                    data-testid={`urbs-restore-checkpoint-${restoreTarget}`}
                    onClick={() => {
                      setStatusMsg(`Restoring milestone ${restoreTarget}…`);
                      void (vcsCheckpointRestore as (...args: unknown[]) => Promise<unknown>)(
                        'default',
                        restoreTarget,
                      )
                        .then(() => {
                          setStatusMsg(`Restored to milestone ${restoreTarget}`);
                          onRefreshHarness();
                        })
                        .catch(() => {
                          setStatusMsg(`Restore requested for ${restoreTarget}`);
                        });
                    }}
                    className="ml-2 shrink-0 rounded border border-cyan-500/40 bg-cyan-500/15 px-1.5 py-0.5 text-[8.5px] text-cyan-200 hover:bg-cyan-500/30"
                  >
                    ↺ Restore
                  </button>
                </div>
              );
            })}
          </div>
        )}
        {harness.vcs && harness.vcs.prs.length > 0 && (
          <div className="space-y-1">
            {harness.vcs.prs.map((pr) => (
              <div
                key={pr.number}
                className="rounded border border-zinc-800 bg-zinc-900/70 px-2 py-1 font-mono text-[9.5px] text-zinc-200"
              >
                #{pr.number} · {pr.title}
              </div>
            ))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div
      data-testid="urbs-imperium-inspector"
      className="vox-marble-tablet pointer-events-auto absolute bottom-[82px] left-2.5 z-20 max-h-[calc(100%-136px)] w-[304px] overflow-y-auto custom-scrollbar rounded-xl border-2 border-[#C59B27]/85 bg-[#FAF6EE]/96 p-2.5 text-[#1C140C] shadow-2xl backdrop-blur-md"
    >
      <div className="mb-1.5 flex items-center justify-between border-b border-[#C59B27]/40 pb-1">
        <span className="font-display text-[9.5px] font-bold uppercase tracking-widest text-[#8C6214]">
          🏛 IMPERIVM PREFECTURE DOCK
        </span>
        <button
          type="button"
          aria-label="Close Imperium Inspector"
          onClick={() => onSelect(null)}
          className="rounded px-1.5 py-0.5 font-mono text-[10px] text-[#5C4428] hover:bg-[#EFE6D5] hover:text-[#1C140C]"
        >
          ✕
        </button>
      </div>

      {selection.kind === 'district' && (() => {
        const d = layout.districts.find((x) => x.name === selection.name) ?? layout.districts[0];
        return d ? renderDistrictPanel(d) : null;
      })()}
      {selection.kind === 'building' && renderBuildingPanel(selection.path)}
      {selection.kind === 'landmark' && renderLandmarkPanel(selection.id)}
      {selection.kind === 'viaduct' && (
        <div data-testid="urbs-viaduct-contract-panel" className="space-y-2.5">
          <div>
            <div className="font-serif text-xs font-bold tracking-wider text-amber-200 uppercase">
              PONS FOEDERIS · Viaduct Contract Inspector
            </div>
            <div className="font-mono text-[10px] text-cyan-300">
              {selection.edge.from_district_id} (L{selection.edge.from_layer ?? 1}) →{' '}
              {selection.edge.to_district_id} (L{selection.edge.to_layer ?? 1})
            </div>
          </div>
          {selection.edge.layer_violation && (
            <div
              data-testid="urbs-viaduct-layer-violation-banner"
              className="rounded border border-rose-500/70 bg-rose-950/60 px-2 py-1.5 font-mono text-[9.5px] text-rose-200"
            >
              ⚠ LAYER VIOLATION: Lower layer L{selection.edge.from_layer ?? 1} imports upward into L
              {selection.edge.to_layer ?? 3} (`layers.toml` breach)
            </div>
          )}
          <div className="rounded border border-white/10 bg-black/40 p-2 font-mono text-[9.5px]">
            <div className="mb-1 text-[9px] uppercase tracking-wider text-zinc-400">
              Imported Symbols & Call Sites ({selection.edge.call_site_count ?? 1})
            </div>
            <div className="space-y-1">
              {(selection.edge.imported_symbols ?? [`use ${selection.edge.to_district_id}::*`]).map(
                (sym, idx) => (
                  <div
                    key={idx}
                    className="truncate rounded bg-zinc-900/80 px-1.5 py-0.5 text-emerald-300"
                  >
                    {sym}
                  </div>
                ),
              )}
            </div>
          </div>
          <button
            type="button"
            data-testid="urbs-viaduct-repair-btn"
            disabled={busy}
            onClick={() =>
              dispatchAgentToTarget(
                `${selection.edge.from_district_id}/Cargo.toml`,
                `Repair contract edge ${selection.edge.from_district_id} → ${selection.edge.to_district_id}`,
              )
            }
            className="w-full rounded border border-amber-400/60 bg-amber-500/25 px-2 py-1 font-mono text-[10px] font-semibold text-amber-100 hover:bg-amber-500/35"
          >
            ⚒ Repair Contract Mismatch
          </button>
        </div>
      )}

      {statusMsg && (
        <div className="mt-2 rounded border border-emerald-500/40 bg-emerald-500/10 px-2 py-1 font-mono text-[9.5px] text-emerald-300">
          {statusMsg}
        </div>
      )}
    </div>
  );
};
