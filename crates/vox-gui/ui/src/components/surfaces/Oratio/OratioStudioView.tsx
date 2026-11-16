import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { voxTransport, type ExecuteOutput } from '../../../transport';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { Toast } from '../../../types/tauri';
import type { TranscribeResult } from '../Loquela/oratioVoiceInput';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanRotaryKnob } from '../../ui/RomanRotaryKnob';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';
import { DiegeticFxStage } from '../../fx/DiegeticFxStage';

export interface OratioStudioViewProps {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
}

export interface TranscriptUtterance {
  id: string;
  text: string;
  rawText: string;
  refinedText: string;
  seconds: number;
  capturedAt: number;
}

type ProbeKey = 'doctor' | 'status';
type ProbeState = { kind: 'ok'; out: ExecuteOutput } | { kind: 'error'; message: string };
type StudioFocus = 'all' | 'probes' | 'vad' | 'ledger';

const PREF_HISTORY = 'vox.oratio.history.v1';
const PREF_VAD = 'vox.oratio.vad_threshold.v1';
const PREF_CAPTURE_SEC = 'vox.oratio.capture_seconds.v1';
const HARMONIC_WEIGHTS = [0.28, 0.44, 0.66, 0.84, 0.94, 0.76, 0.58, 0.88, 0.96, 0.72, 0.54, 0.79, 0.62, 0.46, 0.34, 0.24];

function clampPrefInt(raw: string | null | undefined, min: number, max: number, fallback: number): number {
  const n = raw ? Number(raw) : NaN;
  return Number.isFinite(n) ? Math.max(min, Math.min(max, Math.round(n))) : fallback;
}

function parseStoredHistory(raw: string | null | undefined): TranscriptUtterance[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (x): x is TranscriptUtterance =>
        Boolean(x) && typeof x === 'object' && typeof (x as TranscriptUtterance).id === 'string' &&
        typeof (x as TranscriptUtterance).text === 'string' && typeof (x as TranscriptUtterance).seconds === 'number' &&
        typeof (x as TranscriptUtterance).capturedAt === 'number',
    ).slice(0, 25);
  } catch {
    return [];
  }
}

function parseProbeFacts(probe: ProbeState | undefined, fallback: string): Array<{ label: string; value: string }> {
  if (!probe) return [{ label: fallback, value: 'Probing...' }];
  if (probe.kind === 'error') return [{ label: 'Transport', value: probe.message }];
  const tokens = [probe.out.stdout, probe.out.stderr].filter(Boolean).join(' | ').split(/[|\n]+/).map((s) => s.trim()).filter(Boolean).slice(0, 4);
  if (tokens.length === 0) return [{ label: fallback, value: probe.out.exit_code === 0 ? 'Healthy' : `Exit ${probe.out.exit_code}` }];
  return tokens.map((tok, i) => {
    const idx = tok.indexOf(':');
    return idx > 0 && idx < tok.length - 1
      ? { label: tok.slice(0, idx).trim(), value: tok.slice(idx + 1).trim() }
      : { label: i === 0 ? fallback : `Signal ${i + 1}`, value: tok };
  });
}

function formatCapturedClock(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return 'recent';
  const sec = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (sec < 5) return 'just now';
  return sec < 60 ? `${sec}s ago` : sec < 3600 ? `${Math.floor(sec / 60)}m ago` : `${Math.floor(sec / 3600)}h ago`;
}

export function OratioStudioView({ pushToast }: OratioStudioViewProps) {
  const [vadThreshold, setVadThreshold] = useState<number>(45);
  const [captureSeconds, setCaptureSeconds] = useState<number>(5);
  const [history, setHistory] = useState<TranscriptUtterance[]>([]);
  const [probes, setProbes] = useState<Partial<Record<ProbeKey, ProbeState>>>({});
  const [probing, setProbing] = useState<boolean>(false);
  const [capturing, setCapturing] = useState<boolean>(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const [studioFocus, setStudioFocus] = useState<StudioFocus>('all');

  const mountedRef = useRef<boolean>(true);
  const probeSeqRef = useRef<number>(0);
  const writeSeqRef = useRef<Record<string, number>>({});
  const confirmedSeqRef = useRef<Record<string, number>>({});
  const committedVadRef = useRef<number>(45);
  const confirmedVadRef = useRef<number>(45);
  const committedSecRef = useRef<number>(5);
  const confirmedSecRef = useRef<number>(5);
  const committedHistoryRef = useRef<TranscriptUtterance[]>([]);
  const confirmedHistoryRef = useRef<TranscriptUtterance[]>([]);

  useEffect(() => {
    mountedRef.current = true;
    let cancelled = false;
    if (typeof voxTransport.getGuiPreference === 'function') {
      void Promise.all([
        voxTransport.getGuiPreference(PREF_VAD).catch(() => null),
        voxTransport.getGuiPreference(PREF_CAPTURE_SEC).catch(() => null),
        voxTransport.getGuiPreference(PREF_HISTORY).catch(() => null),
      ]).then(([rawVad, rawSec, rawHist]) => {
        if (cancelled || !mountedRef.current) return;
        if (rawVad !== null && !(writeSeqRef.current[PREF_VAD] > 0)) {
          const v = clampPrefInt(rawVad, 10, 90, 45);
          committedVadRef.current = v;
          confirmedVadRef.current = v;
          setVadThreshold(v);
        }
        if (rawSec !== null && !(writeSeqRef.current[PREF_CAPTURE_SEC] > 0)) {
          const s = clampPrefInt(rawSec, 2, 15, 5);
          committedSecRef.current = s;
          confirmedSecRef.current = s;
          setCaptureSeconds(s);
        }
        if (rawHist !== null && !(writeSeqRef.current[PREF_HISTORY] > 0)) {
          const h = parseStoredHistory(rawHist);
          committedHistoryRef.current = h;
          confirmedHistoryRef.current = h;
          setHistory(h);
        }
      });
    }
    return () => {
      cancelled = true;
      mountedRef.current = false;
    };
  }, []);

  const refreshProbes = useCallback(async () => {
    const reqId = ++probeSeqRef.current;
    setProbing(true);
    const next: Partial<Record<ProbeKey, ProbeState>> = {};
    await Promise.all((['doctor', 'status'] as const).map(async (key) => {
      try {
        const out = await voxTransport.executeCommand(['oratio', key], {});
        next[key] = { kind: 'ok', out };
      } catch (err) {
        next[key] = { kind: 'error', message: sanitizeErrorForToast(err) };
      }
    }));
    if (!mountedRef.current || reqId !== probeSeqRef.current) return;
    setProbes(next);
    setProbing(false);
    const failures = Object.values(next).filter((r) => r?.kind === 'error' || (r?.kind === 'ok' && r.out.exit_code !== 0)).length;
    if (failures > 0) {
      pushToast({ tone: 'warn', title: 'Vox Oratio', body: `${failures} of 2 acoustic runtime probes reported warnings.`, cause: 'backend-error' });
    }
  }, [pushToast]);

  useEffect(() => { void refreshProbes(); }, [refreshProbes]);

  const commitNumericPref = useCallback(
    (
      key: string,
      next: number,
      min: number,
      max: number,
      ref: React.MutableRefObject<number>,
      confirmedRef: React.MutableRefObject<number>,
      setter: (v: number) => void,
      title: string,
    ) => {
      const clamped = Math.max(min, Math.min(max, Math.round(next)));
      const reqId = (writeSeqRef.current[key] ?? 0) + 1;
      writeSeqRef.current[key] = reqId;
      ref.current = clamped;
      setter(clamped);
      void voxTransport.setGuiPreference(key, String(clamped))
        .then(() => {
          if (reqId > (confirmedSeqRef.current[key] ?? 0)) {
            confirmedSeqRef.current[key] = reqId;
            confirmedRef.current = clamped;
          }
        })
        .catch((err) => {
          if (reqId !== writeSeqRef.current[key] || !mountedRef.current) return;
          ref.current = confirmedRef.current;
          setter(confirmedRef.current);
          pushToast({ tone: 'warn', title, body: sanitizeErrorForToast(err), cause: 'backend-error' });
        });
    },
    [pushToast],
  );

  const handleCapture = useCallback(async () => {
    setCapturing(true);
    setCaptureError(null);
    try {
      const res = (await voxTransport.oratioTranscribe(captureSeconds)) as TranscribeResult;
      if (!mountedRef.current) return;
      const text = (res?.text ?? '').trim();
      const rawText = (res?.raw_text ?? text).trim();
      const refinedText = (res?.refined_text ?? text).trim();
      if (!text) {
        pushToast({ tone: 'info', title: 'Vox Oratio', body: 'No speech detected inside capture window.', cause: 'validation' });
        return;
      }
      const base = committedHistoryRef.current;
      const next: TranscriptUtterance[] = [
        { id: `utt-${Date.now()}-${base.length + 1}`, text, rawText, refinedText, seconds: captureSeconds, capturedAt: Date.now() },
        ...base,
      ].slice(0, 25);
      const reqId = (writeSeqRef.current[PREF_HISTORY] ?? 0) + 1;
      writeSeqRef.current[PREF_HISTORY] = reqId;
      committedHistoryRef.current = next;
      setHistory(next);
      try {
        await voxTransport.setGuiPreference(PREF_HISTORY, JSON.stringify(next));
        if (reqId > (confirmedSeqRef.current[PREF_HISTORY] ?? 0)) {
          confirmedSeqRef.current[PREF_HISTORY] = reqId;
          confirmedHistoryRef.current = next;
        }
      } catch (persistErr) {
        if (reqId !== writeSeqRef.current[PREF_HISTORY] || !mountedRef.current) return;
        committedHistoryRef.current = confirmedHistoryRef.current;
        setHistory(confirmedHistoryRef.current);
        const msg = sanitizeErrorForToast(persistErr);
        setCaptureError(msg);
        pushToast({ tone: 'warn', title: 'Transcript persistence failed', body: msg, cause: 'backend-error' });
      }
    } catch (err) {
      if (!mountedRef.current) return;
      const msg = sanitizeErrorForToast(err);
      setCaptureError(msg);
      pushToast({ tone: 'warn', title: 'Oratio capture failed', body: msg, cause: 'backend-error' });
    } finally {
      if (mountedRef.current) setCapturing(false);
    }
  }, [captureSeconds, pushToast]);

  const handleClearHistory = useCallback(async () => {
    const reqId = (writeSeqRef.current[PREF_HISTORY] ?? 0) + 1;
    writeSeqRef.current[PREF_HISTORY] = reqId;
    committedHistoryRef.current = [];
    setHistory([]);
    try {
      await voxTransport.setGuiPreference(PREF_HISTORY, '[]');
      if (reqId > (confirmedSeqRef.current[PREF_HISTORY] ?? 0)) {
        confirmedSeqRef.current[PREF_HISTORY] = reqId;
        confirmedHistoryRef.current = [];
      }
    } catch (err) {
      if (reqId !== writeSeqRef.current[PREF_HISTORY] || !mountedRef.current) return;
      committedHistoryRef.current = confirmedHistoryRef.current;
      setHistory(confirmedHistoryRef.current);
      pushToast({ tone: 'warn', title: 'Clear history failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  const handleCopyText = useCallback(async (text: string) => {
    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) await navigator.clipboard.writeText(text);
      pushToast({ tone: 'ok', title: 'Vox Oratio', body: 'Transcript copied to clipboard.', cause: 'backend-ok' });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Clipboard copy failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  const healthyProbes = useMemo(() => Object.values(probes).filter((p) => p?.kind === 'ok' && p.out.exit_code === 0).length, [probes]);
  const failedProbes = useMemo(() => Object.values(probes).filter((p) => p?.kind === 'error' || (p?.kind === 'ok' && p.out.exit_code !== 0)).length, [probes]);
  const vadRatio = Math.max(0, Math.min(1, (vadThreshold - 10) / 80));
  const doctorFacts = useMemo(() => parseProbeFacts(probes.doctor, 'Runtime'), [probes.doctor]);
  const statusFacts = useMemo(() => parseProbeFacts(probes.status, 'Session'), [probes.status]);
  const knobSpecs = [
    {
      title: 'VAD Sensitivity', desc: 'Silero speech energy gate persisted across studio sessions.',
      ariaLabel: 'Oratio VAD threshold', label: 'VAD', min: 10, max: 90, step: 5, value: vadThreshold,
      formatValue: (v: number) => `${v}%`, onChange: setVadThreshold,
      onCommit: (v: number) => commitNumericPref(PREF_VAD, v, 10, 90, committedVadRef, confirmedVadRef, setVadThreshold, 'VAD threshold rollback'),
    },
    {
      title: 'Capture Window', desc: 'Active microphone sample duration per Push-to-Talk trigger.',
      ariaLabel: 'Oratio capture window seconds', label: 'Window', min: 2, max: 15, step: 1, value: captureSeconds,
      formatValue: (v: number) => `${v}s`, onChange: setCaptureSeconds,
      onCommit: (v: number) => commitNumericPref(PREF_CAPTURE_SEC, v, 2, 15, committedSecRef, confirmedSecRef, setCaptureSeconds, 'Capture window rollback'),
    },
  ];

  return (
    <section data-testid="oratio-studio-view" data-studio-focus={studioFocus} className="@container/surface flex min-h-0 min-w-0 flex-col gap-3.5">
      <RomanSurfaceHeader
        domainTag="COMPUTE · SPEECH"
        title="Vox Oratio"
        subtitle="Acoustic speech-to-code studio, Silero VAD tuning & live ASR telemetry"
        waxSealBadge={capturing ? 'CAPTURING' : probing ? 'PROBING' : failedProbes > 0 ? 'ERROR' : 'ASR READY'}
        icon={<Icon.mic className="size-5 text-brass" aria-hidden="true" />}
        gauges={[
          { label: 'Probes', valueText: `${healthyProbes}/2 OK`, ratio: healthyProbes / 2, tone: failedProbes > 0 ? 'crimson' : 'emerald',
            purpose: 'Oratio doctor and backend status CLI diagnostics (exit 0)', actionHint: 'Click to focus acoustic runtime diagnostics',
            active: studioFocus === 'probes', onClick: () => setStudioFocus((f) => (f === 'probes' ? 'all' : 'probes')) },
          { label: 'VAD Gate', valueText: `${vadThreshold}%`, ratio: vadRatio, tone: 'gold',
            purpose: 'Silero voice-activity threshold gating microphone frames', actionHint: 'Click to focus Silero VAD & capture controls',
            active: studioFocus === 'vad', onClick: () => setStudioFocus((f) => (f === 'vad' ? 'all' : 'vad')) },
          { label: 'Utterances', valueText: `${history.length} LOG`, ratio: Math.min(1, history.length / 10), tone: history.length > 0 ? 'emerald' : 'gold',
            purpose: 'Captured speech-to-code utterances stored in session ledger', actionHint: 'Click to focus transcript history ledger',
            active: studioFocus === 'ledger', onClick: () => setStudioFocus((f) => (f === 'ledger' ? 'all' : 'ledger')) },
        ]}
        actions={
          <button type="button" data-testid="oratio-refresh-probes-btn" disabled={probing} onClick={() => void refreshProbes()}
            className="inline-flex items-center gap-1.5 rounded-lg border border-brass/55 bg-bg-elevated px-2.5 py-1 font-display text-xs font-semibold tracking-wider text-text-primary uppercase transition-colors hover:border-brass disabled:opacity-40">
            <Icon.refresh className="size-3.5" aria-hidden="true" />
            <span>{probing ? 'Probing...' : 'Probe'}</span>
          </button>
        }
      />

      <div data-testid="oratio-vu-stage" className="relative h-32 w-full overflow-hidden rounded-xl border-2 border-border-strong bg-bg-elevated shadow-md">
        <div className="pointer-events-none absolute inset-0 bg-cover bg-center opacity-75" style={{ backgroundImage: "url('/assets/romanesque/carrara-acanthus-header-band.jpg')" }} />
        <DiegeticFxStage preset="voice-harmonics" activeBoost={capturing ? 1.55 : 0.85 + vadRatio * 0.45} intensity={1.15} testId="oratio-fx-voice-harmonics" />
        <div className="relative z-10 flex h-full flex-col justify-between p-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="rounded border border-border-strong bg-bg-surface/90 px-2.5 py-0.5 font-display text-[9.5px] font-bold tracking-[0.14em] text-text-primary uppercase">
              LIVE ACOUSTIC HARMONIC &amp; SILERO VAD STAGE
            </span>
            <RomanWaxSealBadge label={capturing ? `LISTENING · ${captureSeconds}S` : `VAD GATE ${vadThreshold}%`} tone="emerald" />
          </div>
          <div role="img" data-testid="oratio-harmonic-bars" aria-label="Acoustic harmonic RMS meter" className="flex h-12 w-full items-end gap-1.5 rounded-lg border border-border-strong/75 bg-bg-surface/90 px-3 py-1.5">
            {HARMONIC_WEIGHTS.map((weight, idx) => {
              const levelPct = Math.round(Math.min(100, Math.max(14, weight * (capturing ? 96 : 42 + (100 - vadThreshold) * 0.45))));
              return (
                <div key={idx} className="flex h-full flex-1 items-end">
                  <div style={{ height: `${levelPct}%`, backgroundColor: levelPct >= vadThreshold ? 'var(--color-primary)' : 'var(--color-border-strong)' }} className="w-full rounded-xs transition-[height,background-color] duration-200" />
                </div>
              );
            })}
          </div>
        </div>
      </div>

      <BentoGrid preset="hero">
        <BentoCard as="section" span="hero-main" data-testid="oratio-controls-card" className={studioFocus === 'vad' || studioFocus === 'probes' ? 'ring-2 ring-brass/65' : undefined}>
          <div className="mb-3 flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2.5">
            <div className="flex items-center gap-2">
              <RomanMedallion active size="sm"><Icon.mic className="size-3.5 text-text-primary" aria-hidden="true" /></RomanMedallion>
              <div>
                <h2 className="font-display text-xs font-bold tracking-wider text-text-primary uppercase">Push-to-Talk &amp; Acoustic Calibration</h2>
                <p className="text-[11px] text-text-muted">16 kHz PCM16 capture routed via deterministic Oratio STT refinement</p>
              </div>
            </div>
            <button type="button" data-testid="oratio-ptt-capture-btn" disabled={capturing} onClick={() => void handleCapture()}
              className="inline-flex items-center gap-2 rounded-lg border border-brass/55 bg-brass/15 px-3.5 py-1.5 font-display text-xs font-bold tracking-wider text-text-primary uppercase transition-colors hover:border-brass hover:bg-brass/25 disabled:cursor-not-allowed disabled:opacity-50">
              <Icon.mic className="size-3.5 text-brass" aria-hidden="true" />
              <span>{capturing ? `Capturing (${captureSeconds}s)...` : `Push-to-Talk (${captureSeconds}s)`}</span>
            </button>
          </div>

          {captureError && (
            <div role="alert" data-testid="oratio-capture-error" className="mb-3 flex items-start gap-2 rounded-lg border border-destructive/45 bg-destructive/10 px-3 py-2 text-xs text-text-primary">
              <Icon.alert className="mt-0.5 size-4 shrink-0 text-destructive" aria-hidden="true" />
              <span>{captureError}</span>
            </div>
          )}

          <div className="mb-3.5 grid grid-cols-1 gap-3 @xl/surface:grid-cols-2">
            {knobSpecs.map((k) => (
              <div key={k.ariaLabel} className="flex items-center justify-between gap-3 rounded-lg border border-border-subtle bg-bg-elevated/60 p-3">
                <div className="min-w-0">
                  <div className="font-display text-[11px] font-bold tracking-wider text-text-primary uppercase">{k.title}</div>
                  <p className="mt-0.5 text-[11px] text-text-muted">{k.desc}</p>
                </div>
                <RomanRotaryKnob ariaLabel={k.ariaLabel} label={k.label} min={k.min} max={k.max} step={k.step}
                  value={k.value} formatValue={k.formatValue} onValueChange={k.onChange} onValueCommit={k.onCommit} />
              </div>
            ))}
          </div>

          <div data-testid="oratio-diagnostics-grid" className="grid grid-cols-1 gap-2.5 @xl/surface:grid-cols-2">
            {([['doctor', 'Runtime Health', doctorFacts], ['status', 'Backend Status', statusFacts]] as const).map(([key, label, facts]) => {
              const p = probes[key];
              const ok = p?.kind === 'ok' && p.out.exit_code === 0;
              return (
                <div key={key} className="rounded-lg border border-border-subtle bg-bg-elevated/45 p-2.5">
                  <div className="mb-2 flex items-center justify-between gap-2">
                    <span className="font-display text-[10.5px] font-bold tracking-wider text-text-primary uppercase">{label}</span>
                    <RomanWaxSealBadge label={!p ? 'PENDING' : ok ? 'HEALTHY' : 'FAULT'} tone={!p ? 'gold' : ok ? 'emerald' : 'crimson'} />
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {facts.map((f) => (
                      <span key={`${f.label}-${f.value}`} className="inline-flex items-center gap-1 rounded-md border border-border-subtle bg-bg-surface px-2 py-0.5 font-mono text-[10.5px] text-text-secondary">
                        <strong className="font-display text-[9px] tracking-wider text-brass uppercase">{f.label}:</strong>
                        <span>{f.value}</span>
                      </span>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
        </BentoCard>

        <BentoCard as="section" span="hero-rail" data-testid="oratio-ledger-card" className={studioFocus === 'ledger' ? 'ring-2 ring-brass/65' : undefined}>
          <div className="mb-2.5 flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2">
            <div>
              <h2 className="font-display text-xs font-bold tracking-wider text-text-primary uppercase">Transcript Ledger</h2>
              <p className="text-[11px] text-text-muted">{history.length} captured utterance{history.length === 1 ? '' : 's'}</p>
            </div>
            <div className="flex items-center gap-1.5">
              <button type="button" data-testid="oratio-copy-latest-btn" disabled={history.length === 0}
                onClick={() => history[0] && void handleCopyText(history[0].refinedText || history[0].text)}
                className="inline-flex items-center gap-1 rounded-md border border-border-subtle bg-bg-elevated px-2 py-1 font-display text-[10px] tracking-wider text-text-secondary uppercase transition-colors hover:border-brass/50 hover:text-text-primary disabled:opacity-40">
                <Icon.file className="size-3" aria-hidden="true" /><span>Copy</span>
              </button>
              <button type="button" data-testid="oratio-clear-history-btn" disabled={history.length === 0} onClick={() => void handleClearHistory()}
                className="inline-flex items-center gap-1 rounded-md border border-border-subtle bg-bg-elevated px-2 py-1 font-display text-[10px] tracking-wider text-text-secondary uppercase transition-colors hover:border-brass/50 hover:text-text-primary disabled:opacity-40">
                <Icon.trash className="size-3" aria-hidden="true" /><span>Clear</span>
              </button>
            </div>
          </div>

          <BentoScrollRegion ariaLabel="Oratio transcript history" className="max-h-80 flex-1 pr-1">
            {history.length === 0 ? (
              <div data-testid="oratio-empty-ledger" className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border-subtle bg-bg-elevated/35 px-4 py-8 text-center">
                <RomanMedallion size="sm"><Icon.mic className="size-3.5 text-text-primary" aria-hidden="true" /></RomanMedallion>
                <div className="font-display text-xs font-bold tracking-wider text-text-primary uppercase">No Utterances Recorded</div>
                <p className="max-w-xs text-[11px] leading-relaxed text-text-muted">Trigger Push-to-Talk to sample microphone audio and log refined Vox commands here.</p>
              </div>
            ) : (
              <ul className="space-y-2">
                {history.map((item) => (
                  <li key={item.id} data-testid="oratio-utterance-row" className="rounded-lg border border-border-subtle bg-bg-elevated/55 p-2.5">
                    <div className="flex items-center justify-between gap-2 text-[10px] text-text-muted">
                      <span className="font-mono">{item.seconds}s window · {formatCapturedClock(item.capturedAt)}</span>
                      <button type="button" onClick={() => void handleCopyText(item.refinedText || item.text)} className="font-display tracking-wider text-brass uppercase transition-colors hover:text-text-primary">Copy</button>
                    </div>
                    <div className="mt-1 font-mono text-xs font-semibold text-text-primary break-words">{item.refinedText || item.text}</div>
                    {item.rawText && item.rawText !== (item.refinedText || item.text) && <div className="mt-1 font-mono text-[10.5px] text-text-muted">raw: {item.rawText}</div>}
                  </li>
                ))}
              </ul>
            )}
          </BentoScrollRegion>
        </BentoCard>
      </BentoGrid>
    </section>
  );
}
