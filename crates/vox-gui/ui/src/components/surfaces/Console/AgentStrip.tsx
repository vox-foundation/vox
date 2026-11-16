import React from 'react';
import { Button } from '../../ui/Button';

export interface AgentChip {
  id: string;
  name: string;
  state: string;
}

interface Props {
  agents: AgentChip[];
  onOpen: (agentId: string) => void;
}

/**
 * Persistent strip of live agents. Data comes from the same `vox://orch-status`
 * snapshot the Dashboard uses (mapped by the parent), so the two never disagree.
 * Clicking a chip asks the parent to open that agent as a tab.
 */
export function AgentStrip({ agents, onOpen }: Props) {
  if (agents.length === 0) {
    return (
      <div aria-label="agents" className="px-2 py-1 text-[11px] font-semibold text-text-muted">
        no agents
      </div>
    );
  }
  const visibleAgents = agents.slice(0, 2);
  const hiddenCount = Math.max(0, agents.length - visibleAgents.length);
  return (
    <div
      aria-label="agents"
      className="flex flex-nowrap items-center gap-2 overflow-x-auto px-2 py-1 text-[11px] no-scrollbar"
    >
      {visibleAgents.map((a) => {
        const dotClass =
          a.state === 'running'
            ? 'bg-(--color-status-pass)'
            : a.state === 'paused'
              ? 'bg-(--color-status-warn)'
              : 'bg-text-muted';
        return (
          <Button
            key={a.id}
            onClick={() => onOpen(a.id)}
            className="inline-flex h-6 shrink-0 items-center gap-2 whitespace-nowrap rounded-full px-2 py-1 text-[11px] font-semibold"
            title={`${a.name} - ${a.state}`}
            aria-label={`${a.name} - ${a.state} (open agent ${a.name})`}
          >
            <span className={`size-2 rounded-full ${dotClass}`} aria-hidden="true" />
            <span>{a.name} - {a.state}</span>
          </Button>
        );
      })}
      {hiddenCount > 0 && (
        <span
          className="inline-flex h-6 shrink-0 items-center rounded-full border border-border-subtle bg-overlay-subtle px-2 py-1 text-[11px] font-semibold text-text-muted"
          title={`${hiddenCount} more agents`}
        >
          +{hiddenCount} more
        </span>
      )}
    </div>
  );
}
