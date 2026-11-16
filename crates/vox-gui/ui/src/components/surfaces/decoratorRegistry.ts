import React from 'react';
import { ScientiaSurface } from './Scientia/ScientiaSurface';
import { ResearchView } from './Research/ResearchView';
import { PublicationsView } from './Publications/PublicationsView';
import { SubAgentsView } from './SubAgents/SubAgentsView';
import { MensTrainingView } from './Models/MensTrainingView';
import { OratioStudioView } from './Oratio/OratioStudioView';
import { PopuliMeshBentoView } from './Populi/PopuliMeshBentoView';
import type { Toast } from '../../types/tauri';

/**
 * Props every surface decorator receives. Decorators are hand-built views that
 * replace the default (generated/built-in) view for a surface key. They MUST
 * route command execution through the shared `execute_command` Tauri path so
 * every surface stays on one run seam.
 */
export interface SurfaceDecoratorProps {
  pushToast: (item: Toast) => void;
  /** When false, gamify GUI event hooks are no-ops (Settings SSOT polled in App). */
  gamifyEnabled?: boolean;
}

/**
 * Surface key → decorator. `App.tsx::renderView` consults this before its
 * built-in switch, so promoting a surface to a decorated view is a one-line
 * registration here and removing the entry reverts to the default with no other
 * change. Each command below is an arg-free, read-only CLI command.
 */
export const surfaceDecorators: Record<string, React.ComponentType<SurfaceDecoratorProps>> = {
  scientia: ScientiaSurface,
  // Bespoke, not `commandSurface`: the GPU Probe card needs the model the
  // user actually selected (`get_active_model`) to check a real fit, not a
  // static arg-free command — see `Models/MensTrainingView.tsx`.
  mens: MensTrainingView,
  populi: PopuliMeshBentoView,
  research: ResearchView,
  publications: PublicationsView,
  oratio: OratioStudioView,
  'sub-agents': SubAgentsView,
};
