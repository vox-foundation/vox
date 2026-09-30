// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ChatTurnEventRow } from './ChatTurnEventRow';
import { isKnownTurnEvent, MODE_NAMES, modeLabel, TURN_EVENT_KINDS } from '../../../lib/turnEvents';
import type { TurnEventDto } from '../../../types/dashboard';

interface ContractKind {
  kind: string;
  status?: string;
  required: string[];
  example: TurnEventDto;
}

const CONTRACT: { kinds: ContractKind[] } = JSON.parse(
  readFileSync(
    join(
      dirname(fileURLToPath(import.meta.url)),
      '../../../../../../../contracts/gui/turn-event-kinds.v1.json',
    ),
    'utf8',
  ),
);

const shipped = CONTRACT.kinds.filter((k) => k.status !== 'planned');

function example(kind: string): TurnEventDto {
  const entry = CONTRACT.kinds.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return { ...entry.example };
}

function without(event: TurnEventDto, field: string): TurnEventDto {
  const copy: TurnEventDto = { ...event };
  delete copy[field];
  return copy;
}

describe('turn-event contract (GUI side)', () => {
  it('the GUI knows exactly the contract kinds', () => {
    expect(CONTRACT.kinds.map((k) => k.kind).sort()).toEqual([...TURN_EVENT_KINDS].sort());
  });

  it.each(shipped.map((k) => [k.kind, k] as const))('%s: its example renders', (_kind, entry) => {
    const { container } = render(<ChatTurnEventRow event={entry.example} />);
    expect(container).not.toBeEmptyDOMElement();
  });

  it.each(shipped.flatMap((k) => k.required.map((field) => [k.kind, field, k] as const)))(
    '%s without %s renders nothing',
    (_kind, field, entry) => {
      const { container } = render(<ChatTurnEventRow event={without(entry.example, field)} />);
      expect(container).toBeEmptyDOMElement();
    },
  );

  it('research_milestone never renders its model-supplied query', () => {
    render(<ChatTurnEventRow event={{ ...example('research_milestone'), query: 'SENTINEL-QUERY-TEXT' }} />);
    const row = screen.getByTestId('chat-turn-research-row');
    expect(row).toHaveTextContent('3 waves');
    expect(row).not.toHaveTextContent('SENTINEL-QUERY-TEXT');
  });

  it('routing_decision without a mode still renders, with no mode text', () => {
    const noMode = without(without(example('routing_decision'), 'mode'), 'objective');
    render(<ChatTurnEventRow event={noMode} />);
    const row = screen.getByTestId('chat-turn-routing-row');
    expect(row).toHaveTextContent('acme/widget-5.5');
    expect(row).not.toHaveTextContent('Efficient');
    expect(row).not.toHaveTextContent('undefined');
  });

  it('mode names are the canonical vocabulary keyed by wire value', () => {
    expect(MODE_NAMES).toEqual({ free: 'Free', efficiency: 'Efficient', balanced: 'Balanced', genius: 'Genius' });
    expect(modeLabel('efficiency')).toBe('Efficient');
    expect(modeLabel('constructor')).toBe('constructor');
  });

  it('a kind that only exists on Object.prototype is not a known event', () => {
    expect(isKnownTurnEvent({ kind: 'constructor' })).toBe(false);
    expect(isKnownTurnEvent({ kind: 'toString' })).toBe(false);
    expect(isKnownTurnEvent(undefined)).toBe(false);
  });
});
