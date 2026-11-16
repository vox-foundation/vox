import React from 'react';
import { Icon } from './Icons';

export type PlanStepVisualState =
  | 'pending'
  | 'queued'
  | 'running'
  | 'in_progress'
  | 'completed'
  | 'done'
  | 'failed'
  | 'error'
  | 'blocked'
  | 'paused'
  | 'skipped';

export function PlanStepGlyph({
  state,
  className = 'size-3.5 shrink-0',
}: {
  state: PlanStepVisualState | string;
  className?: string;
}) {
  const s = state.toLowerCase();
  if (s === 'completed' || s === 'done' || s === 'ok') {
    return <Icon.check className={className} aria-hidden="true" data-icon="plan-step-done" />;
  }
  if (s === 'failed' || s === 'error') {
    return <Icon.close className={className} aria-hidden="true" data-icon="plan-step-failed" />;
  }
  if (s === 'paused' || s === 'blocked') {
    return <Icon.pause className={className} aria-hidden="true" data-icon="plan-step-paused" />;
  }
  if (s === 'skipped') {
    return <Icon.chevR className={className} aria-hidden="true" data-icon="plan-step-skipped" />;
  }
  if (s === 'running' || s === 'in_progress') {
    return (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={className} aria-hidden="true" data-icon="plan-step-running">
        <circle cx="12" cy="12" r="8.5" strokeOpacity="0.28" />
        <path d="M12 3.5a8.5 8.5 0 0 1 8.5 8.5" strokeLinecap="round" />
        <circle cx="12" cy="12" r="2.5" fill="currentColor" />
      </svg>
    );
  }
  if (s === 'queued') {
    return (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className={className} aria-hidden="true" data-icon="plan-step-queued">
        <circle cx="12" cy="12" r="8.5" strokeDasharray="3 3" />
      </svg>
    );
  }
  return <Icon.circle className={className} aria-hidden="true" data-icon="plan-step-pending" />;
}

export function RoleRankEmblem({
  rank,
  className = 'size-3.5 shrink-0',
}: {
  rank: 'legatus' | 'praetor' | 'centurion' | 'faber' | 'scribe' | string;
  className?: string;
}) {
  const r = rank.toLowerCase();
  if (r === 'legatus' || r === 'imperator' || r === 'architect') {
    return <Icon.laurelWreath className={className} aria-hidden="true" data-icon="rank-legatus" />;
  }
  if (r === 'praetor' || r === 'senior' || r === 'reviewer') {
    return <Icon.crystalGem className={className} aria-hidden="true" data-icon="rank-praetor" />;
  }
  if (r === 'centurion' || r === 'executor') {
    return <Icon.gladiusShield className={className} aria-hidden="true" data-icon="rank-centurion" />;
  }
  if (r === 'faber' || r === 'worker') {
    return <Icon.anvilForge className={className} aria-hidden="true" data-icon="rank-faber" />;
  }
  return <Icon.scrollTabula className={className} aria-hidden="true" data-icon="rank-scribe" />;
}

export function KeycapIcon({
  keyName,
  className = 'size-3 shrink-0',
}: {
  keyName: 'Enter' | 'Shift' | 'Mod' | 'Tab' | 'Up' | 'Down';
  className?: string;
}) {
  switch (keyName) {
    case 'Enter':
      return <Icon.cornerDownLeft className={className} aria-hidden="true" data-icon="keycap-enter" />;
    case 'Mod':
      return <Icon.command className={className} aria-hidden="true" data-icon="keycap-mod" />;
    case 'Shift':
      return <Icon.arrowUp className={className} aria-hidden="true" data-icon="keycap-shift" />;
    case 'Tab':
      return <Icon.arrowRight className={className} aria-hidden="true" data-icon="keycap-tab" />;
    case 'Up':
      return <Icon.arrowUp className={className} aria-hidden="true" data-icon="keycap-up" />;
    case 'Down':
      return <Icon.arrowDown className={className} aria-hidden="true" data-icon="keycap-down" />;
  }
}
