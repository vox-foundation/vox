import React from 'react';
import type { Toast } from '../../../types/tauri';
import { PopuliTopologyView } from '../Populi/PopuliTopologyView';

export interface MeshViewProps {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
}

/** One node row as summarized by the `vox_mesh_nodes` MCP tool. */
export interface MeshNode {
  id: string;
  status: string;
  vram_gb?: number | null;
  host_triple?: string | null;
  gpu_summary?: string | null;
  trust_tier?: string | null;
  advertised_models?: string[];
  last_seen_unix_ms?: number;
}

export function MeshView({ pushToast, gamifyEnabled }: MeshViewProps) {
  return (
    <PopuliTopologyView
      entrySurface="mesh"
      pushToast={pushToast}
      gamifyEnabled={gamifyEnabled}
    />
  );
}
