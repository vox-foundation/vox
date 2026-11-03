import React from 'react';
import { EmptyState } from '../ui/EmptyState';
import { surfaceAvailability, useBackendStatus } from '../../hooks/useBackendStatus';
import type { SurfaceUnavailableReason } from '../../types/tauri';

export const UNAVAILABLE_COPY: Record<SurfaceUnavailableReason, { title: string; description: string }> = {
  not_in_profile: {
    title: 'Not available on this server',
    description: "This server's profile does not include this surface.",
  },
  needs_local_machine: {
    title: 'Not available on this server',
    description: 'This surface works on files, processes, or settings on this machine. Disconnect from the server to use it.',
  },
  not_remote_capable: {
    title: 'Not available on this server',
    description: 'This surface cannot run against a remote server yet.',
  },
};

export const UNAVAILABLE_REASON_LABEL: Record<SurfaceUnavailableReason, string> = {
  not_in_profile: 'not in server profile',
  needs_local_machine: 'needs this machine',
  not_remote_capable: 'not remote-capable yet',
};

export function SurfaceUnavailable({ viewKey, reason }: { viewKey: string; reason: SurfaceUnavailableReason }) {
  const copy = UNAVAILABLE_COPY[reason];
  return (
    <div data-testid="surface-unavailable" data-reason={reason} data-view={viewKey} className="p-6">
      <EmptyState variant="no-connection" title={copy.title} description={copy.description} headingLevel={2} />
    </div>
  );
}

export function SurfaceGate({ viewKey, children }: { viewKey: string; children: React.ReactNode }) {
  const { status } = useBackendStatus();
  const availability = surfaceAvailability(status, viewKey);
  if (availability.available || !availability.reason) return <>{children}</>;
  return <SurfaceUnavailable viewKey={viewKey} reason={availability.reason} />;
}
