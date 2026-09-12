import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CommandCardsView, SurfaceCard } from '../CommandCardsView';
import { MensServePanel } from './MensServePanel';
import type { Toast } from '../../../types/tauri';

interface MensTrainingViewProps {
  pushToast: (item: Toast) => void;
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
      <CommandCardsView
        title="Vox Mens"
        subtitle="ML training & local models"
        cards={cards}
        pushToast={pushToast}
      />
    </div>
  );
}
