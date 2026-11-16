import React, { useEffect, useRef } from 'react';
import { useStore } from 'zustand';
import { projectIso, getZIndex } from '../../lib/projection';
import { Icon, RoleRankEmblem } from '../ui/Icons';
import { useLudusStore, AgentState } from './store';

interface CitizenProps {
  id: string;
  name: string;
  tileWidth: number;
  tileHeight: number;
  offsetX: number;
  offsetY: number;
}

function renderMoodBadgeIcon(mood: string, rankHint: string): React.ReactNode {
  switch (mood) {
    case 'Excited':
      return <RoleRankEmblem rank={rankHint} className="size-3 shrink-0" />;
    case 'Tired':
      return <Icon.crystalGem className="size-3 shrink-0 opacity-80" aria-hidden="true" />;
    case 'Sad':
      return <Icon.minus className="size-3 shrink-0" aria-hidden="true" />;
    case 'Exhausted':
      return <Icon.circle className="size-3 shrink-0 opacity-70" aria-hidden="true" />;
    case 'Happy':
    default:
      return <Icon.spark className="size-3 shrink-0" aria-hidden="true" />;
  }
}

export const CitizenSprite: React.FC<CitizenProps> = ({
  id,
  name,
  tileWidth,
  tileHeight,
  offsetX,
  offsetY,
}) => {
  const spriteRef = useRef<HTMLDivElement | null>(null);
  const agent = useStore(useLudusStore, (state) => state.agents[id]);
  const mood = agent?.mood || 'Happy';


  useEffect(() => {
    // 1. Lazy registration ONLY if the agent doesn't exist
    const store = useLudusStore.getState();
    if (!store.agents[id]) {
      store.updateAgent(id, { x: 2, y: 2, energy: 100, mood: 'Happy' });
    }

    const updateStyle = (agentState: AgentState) => {
      const el = spriteRef.current;
      if (!el) return;

      const { px, py } = projectIso(agentState.x, agentState.y, 0, tileWidth, tileHeight, offsetX, offsetY);
      const zIndex = getZIndex(agentState.x, agentState.y);

      el.style.transform = `translate3d(${px}px, ${py - 24}px, 0) translate(-50%, -50%)`;
      el.style.zIndex = zIndex.toString();
    };

    // 2. Immediate position update to align with mount and camera panning
    const initialAgent = useLudusStore.getState().agents[id] || { x: 2, y: 2, energy: 100, mood: 'Happy' as const };
    updateStyle(initialAgent);

    let prevAgentState = initialAgent;

    // 3. Subscription with change detection to avoid redundant style writes
    const unsubscribe = useLudusStore.subscribe((state) => {
      const agent = state.agents[id];
      if (!agent || agent === prevAgentState) return;
      prevAgentState = agent;
      updateStyle(agent);
    });

    return () => unsubscribe();
  }, [id, tileWidth, tileHeight, offsetX, offsetY]);

  return (
    <div
      ref={spriteRef}
      className="absolute flex flex-col items-center pointer-events-none"
      style={{ left: 0, top: 0 }}
    >
      <div className="inline-flex items-center gap-1 text-[9px] bg-bg-elevated px-1.5 py-0.5 rounded-sm border border-[#C59B27]/50 text-text-primary font-mono scale-75 whitespace-nowrap mb-1 shadow-xs">
        <RoleRankEmblem rank={name} className="size-2.5 shrink-0 text-[#8C6825]" />
        <span>{name}</span>
      </div>
      <div className="w-5 h-5 rounded-full bg-bg-surface flex items-center justify-center border border-[#C59B27]/80 text-[#8C6825] shadow-md">
        {renderMoodBadgeIcon(mood, name)}
      </div>
    </div>
  );
};
