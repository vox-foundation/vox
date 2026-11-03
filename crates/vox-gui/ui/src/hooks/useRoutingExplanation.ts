import { useEffect, useState } from 'react';
import { voxTransport } from '../transport';
import type { RouteExplanation, RoutingHealth } from '../types/tauri';

export interface RoutingExplanationState {
  explanation: RouteExplanation | null;
  health: RoutingHealth | null;
  loading: boolean;
  error: string | null;
}

/** How routing would choose now for a mode, task and complexity, plus routing health. */
export function useRoutingExplanation(mode: string, task: string, complexity: number): RoutingExplanationState {
  const [state, setState] = useState<RoutingExplanationState>({ explanation: null, health: null, loading: true, error: null });
  useEffect(() => {
    let live = true;
    setState(s => ({ ...s, loading: true }));
    Promise.allSettled([
      voxTransport.explainRouting(mode, task, complexity),
      voxTransport.getRoutingHealth(),
    ]).then(([exp, health]) => {
      if (!live) return;
      const explanation = exp.status === 'fulfilled' ? exp.value ?? null : null;
      setState({
        explanation,
        health: health.status === 'fulfilled' ? health.value ?? null : null,
        loading: false,
        error: explanation ? null : 'Routing explanation unavailable',
      });
    });
    return () => { live = false; };
  }, [mode, task, complexity]);
  return state;
}
