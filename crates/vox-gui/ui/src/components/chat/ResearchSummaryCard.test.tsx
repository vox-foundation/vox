// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import React from 'react';
import { ResearchSummaryCard } from './ResearchSummaryCard';
import { ChatMessage } from './ChatMessage';
import type { ResearchSummary } from '../../lib/types';

describe('ResearchSummaryCard', () => {
  it('renders research status badge for complete status', () => {
    const summary: ResearchSummary = {
      sessionId: 'sess-123',
      query: 'Rust memory safety and concurrency',
      status: 'complete',
      claims: {
        supported: 10,
        contested: 2,
        refuted: 1,
      },
    };

    render(<ResearchSummaryCard summary={summary} />);

    expect(screen.getByTestId('research-status-badge')).toHaveTextContent(/Research Complete/i);
    expect(screen.getByTestId('research-summary-topic')).toHaveTextContent(
      'Rust memory safety and concurrency'
    );
  });

  it('renders research status badge for in_progress status', () => {
    render(<ResearchSummaryCard status="in_progress" query="Investigating async runtime" />);

    expect(screen.getByTestId('research-status-badge')).toHaveTextContent(/Research In Progress/i);
    expect(screen.getByTestId('research-summary-topic')).toHaveTextContent(
      'Investigating async runtime'
    );
  });

  it('renders research status badge for refuted status', () => {
    render(<ResearchSummaryCard status="refuted" query="Perpetual motion claim" />);

    expect(screen.getByTestId('research-status-badge')).toHaveTextContent(/Refuted by Evidence/i);
  });

  it('renders correct pill counts for supported, contested, and refuted claims', () => {
    render(
      <ResearchSummaryCard
        claims={{
          supported: 8,
          contested: 3,
          refuted: 2,
        }}
      />
    );

    expect(screen.getByTestId('pill-supported')).toHaveTextContent('8 Supported');
    expect(screen.getByTestId('pill-contested')).toHaveTextContent('3 Contested');
    expect(screen.getByTestId('pill-refuted')).toHaveTextContent('2 Refuted');
  });

  it('calls onOpenResearch with sessionId when clicking Open in Research Studio button', () => {
    const onOpenResearch = vi.fn();
    const sessionId = 'session-xyz-789';

    render(<ResearchSummaryCard sessionId={sessionId} onOpenResearch={onOpenResearch} />);

    const openButton = screen.getByTestId('open-research-studio-btn');
    expect(openButton).toHaveTextContent(/Open in Research Studio/i);

    fireEvent.click(openButton);
    expect(onOpenResearch).toHaveBeenCalledTimes(1);
    expect(onOpenResearch).toHaveBeenCalledWith(sessionId);
  });
});

describe('ChatMessage', () => {
  it('renders ResearchSummaryCard when message has research metadata', () => {
    render(
      <ChatMessage
        message={{
          id: 'msg-1',
          role: 'assistant',
          text: 'Here are the research results',
          research: {
            sessionId: 'sess-abc',
            query: 'Quantum entanglement',
            status: 'complete',
            claims: { supported: 5, contested: 1, refuted: 0 },
          },
        }}
      />
    );

    expect(screen.getByTestId('research-summary-card')).toBeInTheDocument();
    expect(screen.getByTestId('research-status-badge')).toHaveTextContent(/Research Complete/i);
    expect(screen.getByTestId('pill-supported')).toHaveTextContent('5 Supported');
  });

  it('renders ResearchSummaryCard when message has researchSessionId', () => {
    render(
      <ChatMessage
        message={{
          id: 'msg-2',
          role: 'assistant',
          text: 'Research started in background',
          researchSessionId: 'sess-bg-999',
        }}
      />
    );

    expect(screen.getByTestId('research-summary-card')).toBeInTheDocument();
    expect(screen.getByTestId('research-session-id')).toHaveTextContent('#sess-bg-999');
  });

  it('does not render ResearchSummaryCard when message has no research info', () => {
    render(
      <ChatMessage
        message={{
          id: 'msg-3',
          role: 'user',
          text: 'Hello world',
        }}
      />
    );

    expect(screen.queryByTestId('research-summary-card')).not.toBeInTheDocument();
  });
});
