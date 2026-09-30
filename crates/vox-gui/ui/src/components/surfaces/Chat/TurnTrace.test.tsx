// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TurnTrace } from './TurnTrace';
import type { TurnEventDto } from '../../../types/dashboard';

interface Contract {
  kinds: Array<{ kind: string; example: TurnEventDto }>;
  golden_turn?: { events: TurnEventDto[] };
}
const CONTRACT: Contract = JSON.parse(
  readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), '../../../../../../../contracts/gui/turn-event-kinds.v1.json'),
    'utf8',
  ),
);

function example(kind: string): TurnEventDto {
  const entry = CONTRACT.kinds.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return { ...entry.example };
}

const CLEAN = [
  example('routing_decision'),
  example('tool_receipt'),
  example('delegation_spawned'),
  example('research_milestone'),
];

describe('TurnTrace', () => {
  it('a clean turn is one collapsed summary row', () => {
    render(<TurnTrace events={CLEAN} verbosity="normal" latencyMs={4100} />);
    const summary = screen.getByTestId('chat-trace-summary');
    expect(summary).toHaveTextContent('acme/widget-5.5 · 1 tool · 1 receipt ✓ · 1 delegated · 3 research waves · 4.1s');
    expect(summary).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
  });

  it('the summary row expands into the steps in arrival order', () => {
    render(<TurnTrace events={CLEAN} verbosity="normal" />);
    fireEvent.click(screen.getByTestId('chat-trace-summary'));
    expect(screen.getByTestId('chat-trace-summary')).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByTestId('chat-trace-steps')).toBeVisible();
    expect(screen.getAllByTestId('chat-trace-step').map((li) => li.getAttribute('data-kind'))).toEqual([
      'routing_decision',
      'tool_receipt',
      'delegation_spawned',
      'research_milestone',
    ]);
    expect(screen.getByTestId('chat-turn-delegation-row')).toBeVisible();
    expect(screen.getByTestId('chat-turn-research-row')).toBeVisible();
  });

  it('an interrupt shows inline while the trace stays collapsed (quiet)', () => {
    render(
      <TurnTrace
        events={[example('routing_decision'), example('tool_receipt'), example('receipt_claims')]}
        verbosity="quiet"
      />,
    );
    expect(screen.getByTestId('chat-turn-claims-row')).toHaveAttribute('data-flagged', 'true');
    expect(screen.getAllByTestId('chat-turn-claims-row')).toHaveLength(1);
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('identical steps coalesce into one row with a count', () => {
    const r = example('research_milestone');
    render(<TurnTrace events={[example('routing_decision'), r, { ...r }, { ...r }]} verbosity="verbose" />);
    const step = screen
      .getAllByTestId('chat-trace-step')
      .find((li) => li.getAttribute('data-kind') === 'research_milestone');
    expect(step).toHaveTextContent('3×');
  });

  it('skill activation stays inline with its "not this one" action', () => {
    const onExcludeSkill = vi.fn();
    render(
      <TurnTrace
        events={[example('skill_activated'), example('routing_decision')]}
        verbosity="quiet"
        onExcludeSkill={onExcludeSkill}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'not this one' }));
    expect(onExcludeSkill).toHaveBeenCalledWith('ponytail');
  });

  it('a reply with no events keeps its model id as a plain, non-expandable summary', () => {
    render(<TurnTrace verbosity="normal" modelId="mens/run-7" latencyMs={1200} />);
    const summary = screen.getByTestId('chat-trace-summary');
    expect(summary).toHaveTextContent('mens/run-7 · 1.2s');
    expect(summary).not.toHaveTextContent('latest');
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('renders nothing when no event is known and there is no model id', () => {
    const { container } = render(<TurnTrace events={[{ kind: 'from_the_future' }]} verbosity="verbose" />);
    expect(container).toBeEmptyDOMElement();
  });

  it('switching to verbose opens a clean trace that normal leaves collapsed', () => {
    const { rerender } = render(<TurnTrace events={CLEAN} verbosity="normal" />);
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
    rerender(<TurnTrace events={CLEAN} verbosity="verbose" />);
    expect(screen.getByTestId('chat-trace-steps')).toBeVisible();
  });
});
