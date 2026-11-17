import React from 'react';
import type { Agent } from '../../../types/dashboard';
import { ConciliumView } from '../Concilium/ConciliumView';

export interface GraphNode {
  id: string;
  label: string;
  phase: string;
  x: number;
  y: number;
}

export interface GraphEdge {
  from: string;
  to: string;
  flow: number;
}

export interface AgentGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface AgentFlowProps {
  agents: Agent[];
  graph?: AgentGraph;
  onSelect?: (id: string) => void;
  selectedId?: string;
}

/**
 * Mode 1 Compatibility Route (`case 'flow'`).
 * Consolidates `AgentFlow` onto `<ConciliumView initialFocus="swimlanes" />` while preserving
 * `AgentFlowProps` (`agents`, `graph`, `onSelect`, `selectedId`).
 */
export function AgentFlow({ agents, onSelect, selectedId }: AgentFlowProps) {
  return (
    <ConciliumView
      initialFocus="swimlanes"
      legacyAgents={agents}
      selectedAgentId={selectedId}
      onSelectAgent={onSelect}
    />
  );
}
