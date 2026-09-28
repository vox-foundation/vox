// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ChatTurnEventRow } from './ChatTurnEventRow';
import type { TurnEventDto } from '../../../types/dashboard';

describe('ChatTurnEventRow', () => {
  it('renders a chip naming the activated skill', () => {
    render(<ChatTurnEventRow event={{ kind: 'skill_activated', skill_id: 'ponytail' }} />);
    expect(screen.getByTestId('chat-turn-event-row')).toHaveTextContent('ponytail');
  });

  it('calls onExcludeSkill with the skill id when "not this one" is clicked', () => {
    const onExcludeSkill = vi.fn();
    render(
      <ChatTurnEventRow
        event={{ kind: 'skill_activated', skill_id: 'ponytail' }}
        onExcludeSkill={onExcludeSkill}
      />,
    );
    fireEvent.click(screen.getByText('not this one'));
    expect(onExcludeSkill).toHaveBeenCalledWith('ponytail');
  });

  it('does not offer exclusion for an unresolved ("unknown") skill id', () => {
    const onExcludeSkill = vi.fn();
    render(
      <ChatTurnEventRow
        event={{ kind: 'skill_activated', skill_id: 'unknown' }}
        onExcludeSkill={onExcludeSkill}
      />,
    );
    expect(screen.queryByText('not this one')).not.toBeInTheDocument();
  });

  it('renders without throwing on an unrecognized event kind', () => {
    const unknownEvent = { kind: 'some_future_kind_the_ui_has_never_seen' } as TurnEventDto;
    expect(() => render(<ChatTurnEventRow event={unknownEvent} />)).not.toThrow();
    expect(screen.queryByTestId('chat-turn-event-row')).not.toBeInTheDocument();
  });

  it('renders a verified receipt chip for a verified tool_receipt event', () => {
    render(
      <ChatTurnEventRow
        event={{
          kind: 'tool_receipt',
          tool: 'vox_git_status',
          receipt_id: '01920000-aaaa-7bbb-8ccc-000000000001',
          verified: true,
          fulfilled: true,
        }}
      />,
    );
    const row = screen.getByTestId('chat-turn-receipt-row');
    expect(row).toHaveTextContent('vox_git_status');
    expect(row).toHaveTextContent('verified');
    expect(row).toHaveAttribute('data-verified', 'true');
    expect(row).toHaveAttribute('title', 'receipt 01920000-aaaa-7bbb-8ccc-000000000001');
    expect(row).toHaveTextContent('01920000');
  });

  it('renders an unverified receipt chip when verified is false', () => {
    render(
      <ChatTurnEventRow
        event={{
          kind: 'tool_receipt',
          tool: 'vox_skill_list',
          receipt_id: '01920000-aaaa-7bbb-8ccc-000000000002',
          verified: false,
          fulfilled: true,
        }}
      />,
    );
    const row = screen.getByTestId('chat-turn-receipt-row');
    expect(row).toHaveTextContent('vox_skill_list');
    expect(row).toHaveTextContent('unverified');
    expect(row).toHaveAttribute('data-verified', 'false');
  });

  it('renders nothing when tool or receipt_id is missing or non-string', () => {
    const { container: c1 } = render(
      <ChatTurnEventRow
        event={{
          kind: 'tool_receipt',
          tool: 123 as unknown as string,
          receipt_id: '01920000-aaaa-7bbb-8ccc-000000000001',
        }}
      />,
    );
    expect(c1).toBeEmptyDOMElement();

    const { container: c2 } = render(
      <ChatTurnEventRow
        event={{
          kind: 'tool_receipt',
          receipt_id: '01920000-aaaa-7bbb-8ccc-000000000001',
        }}
      />,
    );
    expect(c2).toBeEmptyDOMElement();

    const { container: c3 } = render(
      <ChatTurnEventRow
        event={{
          kind: 'tool_receipt',
          tool: 'vox_git_status',
        }}
      />,
    );
    expect(c3).toBeEmptyDOMElement();
  });

  it('renders a claims chip with counts and flags when fabricated or unverified > 0', () => {
    render(
      <ChatTurnEventRow
        event={{
          kind: 'receipt_claims',
          valid: 2,
          fabricated: 1,
          unverified: 0,
        }}
      />,
    );
    const row = screen.getByTestId('chat-turn-claims-row');
    expect(row).toHaveTextContent('claims · 2 valid · 1 fabricated · 0 unverified');
    expect(row).toHaveAttribute('data-flagged', 'true');
  });

  it('renders a claims chip with data-flagged false when fabricated and unverified are 0', () => {
    render(
      <ChatTurnEventRow
        event={{
          kind: 'receipt_claims',
          valid: 3,
          fabricated: 0,
          unverified: 0,
        }}
      />,
    );
    const row = screen.getByTestId('chat-turn-claims-row');
    expect(row).toHaveTextContent('claims · 3 valid · 0 fabricated · 0 unverified');
    expect(row).toHaveAttribute('data-flagged', 'false');
  });

  it('renders nothing when claims counts are non-numeric or missing', () => {
    const { container: c1 } = render(
      <ChatTurnEventRow
        event={{
          kind: 'receipt_claims',
          valid: '2' as unknown as number,
          fabricated: 0,
          unverified: 0,
        }}
      />,
    );
    expect(c1).toBeEmptyDOMElement();

    const { container: c2 } = render(
      <ChatTurnEventRow
        event={{
          kind: 'receipt_claims',
          valid: 2,
          fabricated: undefined as unknown as number,
          unverified: 0,
        }}
      />,
    );
    expect(c2).toBeEmptyDOMElement();

    const { container: c3 } = render(
      <ChatTurnEventRow
        event={{
          kind: 'receipt_claims',
          valid: 2,
          fabricated: 0,
          unverified: NaN,
        }}
      />,
    );
    expect(c3).toBeEmptyDOMElement();
  });
});


