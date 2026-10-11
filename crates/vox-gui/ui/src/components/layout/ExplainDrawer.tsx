import React from 'react';
import type { CapabilityIndexEntry } from '../../generated/capabilityIndex.generated';
import type { Toast } from '../../types/tauri';

export interface ExplainDrawerProps {
  row: CapabilityIndexEntry | null;
  rows: readonly CapabilityIndexEntry[];
  domains: readonly { id: string; label: string }[];
  catalogPaths: ReadonlySet<string>;
  coverageAsOf?: { sha: string; date: string } | null;
  onClose: () => void;
  onOpenForm: (path: string[]) => void;
  onOpenDoc: (path: string) => void;
  pushToast: (t: Toast) => void;
}

export function ExplainDrawer({ row }: ExplainDrawerProps) {
  if (!row) return null;
  return <div />;
}
