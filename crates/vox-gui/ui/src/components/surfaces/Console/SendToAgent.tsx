import React, { useEffect, useState } from 'react';
import type { AgentChip } from './AgentStrip';
import { Button } from '../../ui/Button';
import { VoxSelect } from '../../ui/Select';
import { TERMINAL_MONO_STYLE } from './terminalTheme';

interface Props {
  /** Prefilled message body (e.g. the last command line). */
  initialBody?: string;
  agents: AgentChip[];
  onSend: (agentId: string, body: string) => void;
  onClose: () => void;
}

/** Small composer to send a free-form note to an agent's A2A inbox. */
export function SendToAgent({ initialBody = '', agents, onSend, onClose }: Props) {
  const [target, setTarget] = useState(agents[0]?.id ?? '');
  const [body, setBody] = useState(initialBody);
  // Keep the selected target valid as the agent list arrives/changes.
  useEffect(() => {
    if (agents.length === 0) {
      setTarget('');
    } else if (!agents.some((a) => a.id === target)) {
      setTarget(agents[0].id);
    }
  }, [agents, target]);
  const canSend = target !== '' && body.trim() !== '';
  return (
    <div
      role="dialog"
      aria-label="send to agent"
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation();
          onClose();
        }
      }}
      className="rounded-lg border border-border-strong bg-bg-elevated p-3 text-xs text-text-primary"
    >
      <VoxSelect
        aria-label="agent"
        value={target}
        onValueChange={setTarget}
        options={agents.map((a) => ({
          value: a.id,
          label: `${a.name} - ${a.state}`,
        }))}
        className="mb-2 w-full"
      />
      <textarea
        aria-label="message"
        data-terminal-mono="true"
        value={body}
        onChange={(e) => setBody(e.target.value)}
        placeholder="message to send to the agent's inbox"
        rows={3}
        style={TERMINAL_MONO_STYLE}
        className="mb-2 w-full rounded border border-border-subtle bg-bg-base p-2 text-xs leading-[1.45] text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-1 focus:ring-ring"
      />
      <div className="flex items-center gap-2">
        <Button size="xs" variant="primary" disabled={!canSend} onClick={() => onSend(target, body)} className="h-6 px-2 py-1 text-[11px] font-semibold">
          Send
        </Button>
        <Button size="xs" variant="ghost" onClick={onClose} className="h-6 px-2 py-1 text-[11px] font-semibold">Cancel</Button>
      </div>
    </div>
  );
}
