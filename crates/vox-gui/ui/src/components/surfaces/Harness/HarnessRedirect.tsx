import React, { useEffect } from 'react';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { HarnessRiverPanel, type HarnessRiverPanelProps } from './HarnessRiverPanel';

export type HarnessRedirectProps = HarnessRiverPanelProps;

/**
 * Axis Harness River surface — serves both the dockable 'harness' panel in ChatSurface
 * and the full Workspace -> Harness view while preserving composer focus actions.
 */
export function HarnessRedirect(props: HarnessRedirectProps) {
  const { gamifyEnabled } = props;
  useEffect(() => {
    void recordGamifyGuiEvent('harness_redirect_viewed', undefined, { enabled: gamifyEnabled });
  }, [gamifyEnabled]);

  return <HarnessRiverPanel {...props} />;
}
