/**
 * Typed daemon-backed RPC wrappers for Mode 1 (Vox Concilium) and Mode 2 (Vox Populi Placement).
 * Routes 100% through `invokeCmd` (`transport.ts`) -> `commands::mission_control` (`daemon`-class)
 * -> `/api/v2/concilium/*` on `vox-orchestrator-mcp`. Zero `VoxDb` handles or CLI spawns in `vox-gui`.
 */

import { invokeCmd } from '../transport';
import type {
  ConciliumPilotActionInput,
  ConciliumPilotActionResult,
  ConciliumSnapshotDto,
  PopuliPlacementDto,
  SequenceBoardEntry,
} from '../types/concilium';

export interface ConciliumBoardQuery {
  sinceSeq?: number | null;
  seqs?: number[] | null;
  channel?: string | null;
  limit?: number | null;
}

export function conciliumSnapshot(): Promise<ConciliumSnapshotDto> {
  return invokeCmd<ConciliumSnapshotDto>('concilium_snapshot');
}

export function conciliumBoard(query?: ConciliumBoardQuery): Promise<SequenceBoardEntry[]> {
  return invokeCmd<SequenceBoardEntry[]>('concilium_board', {
    sinceSeq: query?.sinceSeq ?? null,
    seqs: query?.seqs ?? null,
    channel: query?.channel ?? null,
    limit: query?.limit ?? null,
  });
}

export function conciliumPlacement(): Promise<PopuliPlacementDto> {
  return invokeCmd<PopuliPlacementDto>('concilium_placement');
}

export function conciliumPilotAction(
  input: ConciliumPilotActionInput,
): Promise<ConciliumPilotActionResult> {
  return invokeCmd<ConciliumPilotActionResult>('concilium_pilot_action', { input });
}
