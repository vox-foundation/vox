// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const mockBudget = vi.hoisted(() => ({
  max_context_tokens: 128_000,
  reserved_tokens: 10_000,
  threshold_tokens: 102_400,
  usable_tokens: 118_000,
  strategy: 'balanced',
  used_tokens: 0,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === 'get_context_budget') return Promise.resolve(mockBudget);
    return Promise.resolve(null);
  }),
}));

import { ChatExecutionRail, sessionSpendSeriesKey } from './ChatExecutionRail';
import { LanguageProvider } from '../../../hooks/useLanguage';

const sampleKpis = {
  activeAgents: { value: 3 },
  queueDepth: { value: 7 },
  mesh: { peers: 2 },
};

describe('ChatExecutionRail', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.removeItem('vox.metric.series.v1.chat.session-spend');
    localStorage.removeItem('vox.metric.series.v1.chat.session-spend.sess-a');
    localStorage.removeItem('vox.metric.series.v1.chat.session-spend.sess-b');
  });

  it('renders task list section with aria-label Active tasks', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[{ id: 't1', title: 'Fix CI', status: 'running' }]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    expect(screen.getByRole('region', { name: /active tasks/i })).toBeInTheDocument();
    expect(screen.getByText('Fix CI')).toBeInTheDocument();
    expect(screen.getByText(/running/i)).toBeInTheDocument();
  });

  it('truncates a long task title to a single line rather than wrapping', () => {
    const longTitle = 'Describe a task with a very long and detailed multi-clause description that would otherwise wrap across several lines in the rail';
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[{ id: 't1', title: longTitle, status: 'running' }]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    const titleEl = screen.getByText(longTitle);
    expect(titleEl.className).toContain('truncate');
    expect(titleEl).toHaveAttribute('title', longTitle);
  });

  it('shows resource strip with agents, queue depth, and mesh peers from props', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    expect(screen.getByTestId('execution-rail-agents')).toHaveTextContent('3');
    expect(screen.getByTestId('execution-rail-queue')).toHaveTextContent('7');
    expect(screen.getByTestId('execution-rail-mesh')).toHaveTextContent('2 peers');
  });

  it('labels the aside landmark (axe landmark-unique)', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    expect(screen.getByRole('complementary')).toHaveAttribute('aria-label', 'Execution rail');
  });

  it('shows OpenRouter cost segment when openrouterSpendUsd is provided', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          openrouterSpendUsd={1.25}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    const segment = screen.getByTestId('execution-rail-openrouter');
    expect(segment).toHaveTextContent(/openrouter/i);
    expect(segment).toHaveTextContent('$1.25');
  });

  it('hides OpenRouter segment when openrouterSpendUsd is omitted', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    expect(screen.queryByTestId('execution-rail-openrouter')).toBeNull();
  });

  it('shows current model label when activeModel is provided', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          activeModel="claude-sonnet-4"
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    const segment = screen.getByTestId('execution-rail-model');
    expect(segment).toHaveTextContent(/model/i);
    expect(segment).toHaveTextContent('claude-sonnet-4');
  });

  it('navigates when resource segments are clicked', async () => {
    const onNavigate = vi.fn();
    const user = userEvent.setup();
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={onNavigate}
        />
      </LanguageProvider>,
    );

    await user.click(screen.getByTestId('execution-rail-agents'));
    expect(onNavigate).toHaveBeenCalledWith('agents');

    await user.click(screen.getByTestId('execution-rail-queue'));
    expect(onNavigate).toHaveBeenCalledWith('runs');

    await user.click(screen.getByTestId('execution-rail-mesh'));
    expect(onNavigate).toHaveBeenCalledWith('mesh');
  });

  it('renders intent map section with up to three intent lines and opens the Routing drawer', async () => {
    const onNavigate = vi.fn();
    const onOpenRouting = vi.fn();
    const user = userEvent.setup();
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          intents={['claude-sonnet-4 · exploit', 'Alt: gpt-4o', 'Alt: gemini-pro', 'extra']}
          onNavigate={onNavigate}
          onOpenRouting={onOpenRouting}
        />
      </LanguageProvider>,
    );

    const region = screen.getByRole('region', { name: /intent map/i });
    expect(region).toBeInTheDocument();
    expect(screen.getByText('claude-sonnet-4 · exploit')).toBeInTheDocument();
    expect(screen.getByText('Alt: gpt-4o')).toBeInTheDocument();
    expect(screen.getByText('Alt: gemini-pro')).toBeInTheDocument();
    expect(screen.queryByText('extra')).toBeNull();

    await user.click(screen.getByRole('button', { name: /claude-sonnet-4 · exploit/i }));
    expect(onOpenRouting).toHaveBeenCalledTimes(1);
    expect(onNavigate).not.toHaveBeenCalledWith('matrix');
  });

  it('omits intent map when intents prop is empty', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          intents={[]}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    expect(screen.queryByRole('region', { name: /intent map/i })).toBeNull();
  });

  it('has no leftover per-panel collapse/expand chevron UI (panel visibility is controlled entirely by the dock Panels menu now)', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    expect(screen.queryByRole('button', { name: /collapse execution rail/i })).toBeNull();
    expect(screen.queryByRole('button', { name: /expand execution rail/i })).toBeNull();
  });

  it('lists live agents and opens topology from the roster', async () => {
    const onNavigate = vi.fn();
    const onOpenAgent = vi.fn();
    const user = userEvent.setup();
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={onNavigate}
          onOpenAgent={onOpenAgent}
          agents={[
            {
              id: 'a1',
              codename: 'Falcon',
              phase: 'Executing',
              progress: 0.4,
              task: 'compile crate',
              cost: 0.1,
              budget: 2,
              eta: '1m',
            },
          ]}
        />
      </LanguageProvider>,
    );

    expect(screen.getByRole('region', { name: /agent shards/i })).toBeInTheDocument();
    expect(screen.getByText('Falcon')).toBeInTheDocument();
    expect(screen.getByText('compile crate')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /open topology/i }));
    expect(onNavigate).toHaveBeenCalledWith('flow');

    await user.click(screen.getByRole('button', { name: /falcon/i }));
    expect(onOpenAgent).toHaveBeenCalledWith('a1');
  });

  it('renders ContextWindowMeter after budget loads', async () => {
    const defaultProps = {
      tasks: [],
      kpis: { activeAgents: { value: 0 }, queueDepth: { value: 0 }, mesh: { peers: 0 } },
      onNavigate: vi.fn(),
    };
    render(<LanguageProvider><ChatExecutionRail {...defaultProps} /></LanguageProvider>);
    await waitFor(() => {
      expect(screen.getByRole('meter')).toBeInTheDocument();
    });
  });

  it('renders Agents and Queue as compact segments, not KPI cards with metric rules', () => {
    const { container } = render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} kpis={sampleKpis} onNavigate={vi.fn()} />
      </LanguageProvider>,
    );
    const rail = screen.getByRole('complementary', { name: /execution rail/i });
    expect(rail.querySelector('.vox-metric-rule')).toBeNull();
    expect(screen.getByTestId('execution-rail-agents').className).toMatch(/text-\[10px\]/);
  });

  it('shows a session-spend spark after sessionSpentUsd changes', async () => {
    const { rerender } = render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} kpis={sampleKpis} onNavigate={vi.fn()} sessionSpentUsd={0.1} />
      </LanguageProvider>,
    );
    rerender(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} kpis={sampleKpis} onNavigate={vi.fn()} sessionSpentUsd={0.4} />
      </LanguageProvider>,
    );
    expect(await screen.findByTestId('execution-rail-spend-spark')).toBeInTheDocument();
  });

  it('keys the session-spend series by session id and does not blend a switch', async () => {
    const { rerender } = render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
          sessionId="sess-a"
          sessionSpentUsd={0.1}
        />
      </LanguageProvider>,
    );
    rerender(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
          sessionId="sess-a"
          sessionSpentUsd={0.4}
        />
      </LanguageProvider>,
    );
    expect(await screen.findByTestId('execution-rail-spend-spark')).toBeInTheDocument();
    expect(screen.getByTestId('execution-rail-session')).toHaveTextContent('$0.40');
    const keyA = `vox.metric.series.v1.${sessionSpendSeriesKey('sess-a')}`;
    expect(localStorage.getItem(keyA)).toContain('0.4');

    rerender(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
          sessionId="sess-b"
          sessionSpentUsd={0.05}
        />
      </LanguageProvider>,
    );
    expect(screen.getByTestId('execution-rail-session')).toHaveTextContent('$0.05');
    expect(screen.queryByTestId('execution-rail-spend-spark')).toBeNull();
    expect(localStorage.getItem(keyA)).toContain('0.4');
  });

  it('drops a late context-budget response from the previous session', async () => {
    let resolveA!: (value: typeof mockBudget) => void;
    const delayedA = new Promise<typeof mockBudget>((resolve) => {
      resolveA = resolve;
    });
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation((cmd: string, args?: { sessionId?: string }) => {
      if (cmd === 'get_context_budget') {
        if (args?.sessionId === 'sess-a') return delayedA;
        return Promise.resolve({
          ...mockBudget,
          max_context_tokens: 1000,
          reserved_tokens: 0,
          threshold_tokens: 800,
          usable_tokens: 1000,
          used_tokens: 10,
        });
      }
      return Promise.resolve(null);
    });

    const { rerender } = render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
          sessionId="sess-a"
        />
      </LanguageProvider>,
    );
    rerender(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
          sessionId="sess-b"
        />
      </LanguageProvider>,
    );
    await waitFor(() => {
      expect(screen.getByRole('meter').getAttribute('aria-valuenow')).toBe('10');
    });
    resolveA({
      ...mockBudget,
      max_context_tokens: 1000,
      reserved_tokens: 0,
      threshold_tokens: 800,
      usable_tokens: 1000,
      used_tokens: 900,
    });
    await Promise.resolve();
    expect(screen.getByRole('meter').getAttribute('aria-valuenow')).toBe('10');
  });

  it('hides the context meter when the new session budget fetch fails', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation((cmd: string, args?: { sessionId?: string }) => {
      if (cmd === 'get_context_budget') {
        if (args?.sessionId === 'sess-a') {
          return Promise.resolve({
            ...mockBudget,
            max_context_tokens: 1000,
            reserved_tokens: 0,
            threshold_tokens: 800,
            usable_tokens: 1000,
            used_tokens: 900,
          });
        }
        return Promise.reject(new Error('daemon down'));
      }
      return Promise.resolve(null);
    });

    const { rerender } = render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
          sessionId="sess-a"
        />
      </LanguageProvider>,
    );
    await waitFor(() => {
      expect(screen.getByRole('meter').getAttribute('aria-valuenow')).toBe('900');
    });
    rerender(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
          sessionId="sess-b"
        />
      </LanguageProvider>,
    );
    await waitFor(() => {
      expect(screen.queryByRole('meter')).toBeNull();
    });
  });

  it('passes used_tokens to ContextWindowMeter so it reflects real fill percentage', async () => {
    // Override the mock to return 25% usage (250 / 1000).
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_context_budget') {
        return Promise.resolve({
          max_context_tokens: 1000,
          reserved_tokens: 0,
          threshold_tokens: 800,
          usable_tokens: 1000,
          strategy: 'balanced',
          used_tokens: 250,
        });
      }
      return Promise.resolve(null);
    });

    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          kpis={{ activeAgents: { value: 0 }, queueDepth: { value: 0 }, mesh: { peers: 0 } }}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );

    await waitFor(() => {
      const meter = screen.getByRole('meter');
      expect(meter).toBeInTheDocument();
      // aria-label should say 25% full
      expect(meter.getAttribute('aria-label')).toContain('25%');
      // aria-valuenow should be the real used_tokens value
      expect(meter.getAttribute('aria-valuenow')).toBe('250');
    });
  });
});

