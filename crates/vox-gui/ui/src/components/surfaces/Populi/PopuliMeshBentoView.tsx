import React from 'react';
import type { Toast } from '../../../types/tauri';
import { PopuliTopologyView } from './PopuliTopologyView';

export interface PopuliMeshBentoViewProps {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
}

export function PopuliMeshBentoView({ pushToast, gamifyEnabled }: PopuliMeshBentoViewProps) {
  return (
    <PopuliTopologyView
      entrySurface="populi"
      pushToast={pushToast}
      gamifyEnabled={gamifyEnabled}
    />
  );
}
