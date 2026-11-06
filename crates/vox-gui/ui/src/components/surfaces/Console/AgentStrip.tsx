import React from 'react';
import { viz } from '../../../lib/visualTokens';
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
      <div aria-label="agents" style={{ padding: '4px 10px', fontSize: 11, color: viz.gray400 }}>
        no agents
      </div>
    );
  }
  const visibleAgents = agents.slice(0, 5);
  const hiddenCount = Math.max(0, agents.length - visibleAgents.length);
  return (
    <div
      aria-label="agents"
      className="flex items-center gap-1.5 overflow-x-auto px-2.5 py-1.5 text-[11px] no-scrollbar"
    >
      {visibleAgents.map((a) => {
        const dotClass =
          a.state === 'running'
            ? 'bg-emerald-400'
            : a.state === 'paused'
              ? 'bg-amber-400'
              : 'bg-zinc-500';
        return (
          <Button
            key={a.id}
            onClick={() => onOpen(a.id)}
            className="inline-flex h-6 shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full px-2.5 py-0.5 font-mono text-[10px]"
            title={`${a.name} · ${a.state}`}
            aria-label={`Open agent ${a.name} (${a.state})`}
          >
            <span className={`size-1.5 rounded-full ${dotClass}`} aria-hidden="true" />
            <span>{a.name} · {a.state}</span>
          </Button>
        );
      })}
      {hiddenCount > 0 && (
        <span
          className="inline-flex h-6 shrink-0 items-center rounded-full border border-white/10 bg-white/[0.03] px-2 py-0.5 font-mono text-[10px] text-text-muted"
          title={`${hiddenCount} more agents`}
        >
          +{hiddenCount} more
        </span>
      )}
    </div>
  );
}
