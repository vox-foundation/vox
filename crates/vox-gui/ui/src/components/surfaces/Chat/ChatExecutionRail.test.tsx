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
import { fireEvent, within } from '@testing-library/react';
import { countCompletedTurns } from './ChatExecutionRail';

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

  it('renders execution-rail-lock-chip with holding state and resource id', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[
            {
              id: 't1',
              title: 'Migrate orders table',
              status: 'running',
              lock: { resourceId: 'db://orders/42', state: 'holding' },
            },
          ]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    const chip = screen.getByTestId('execution-rail-lock-chip');
    expect(chip).toBeInTheDocument();
    expect(chip).toHaveAttribute('data-lock-state', 'holding');
    expect(chip).toHaveAttribute('title', 'db://orders/42');
    expect(chip).toHaveTextContent('holding db://orders/42');
  });

  it('renders execution-rail-lock-chip with waiting state', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[
            {
              id: 't1',
              title: 'Migrate orders table',
              status: 'pending',
              lock: { resourceId: 'db://orders/42', state: 'waiting' },
            },
          ]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    const chip = screen.getByTestId('execution-rail-lock-chip');
    expect(chip).toBeInTheDocument();
    expect(chip).toHaveAttribute('data-lock-state', 'waiting');
    expect(chip).toHaveAttribute('title', 'db://orders/42');
    expect(chip).toHaveTextContent('waiting on db://orders/42');
  });

  it('renders no lock chip for a task without lock', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[{ id: 't1', title: 'Task without lock', status: 'running' }]}
          kpis={sampleKpis}
          onNavigate={vi.fn()}
        />
      </LanguageProvider>,
    );
    expect(screen.queryByTestId('execution-rail-lock-chip')).toBeNull();
  });
});

describe('ChatExecutionRail — this session only (plan 3a)', () => {
  const routing = {
    model: 'deepseek/deepseek-flash (offline)',
    reason: 'lowest cost that fits the mode',
    state: 'confirmed',
    alternatives: ['anthropic/claude-haiku', 'google/gemini-flash'],
  };
  const old = { kpis: sampleKpis, onNavigate: vi.fn() };

  it('shows a Routing section for the next turn and never the word Intents', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={routing} {...old} />
      </LanguageProvider>,
    );
    const region = screen.getByRole('region', { name: 'Routing' });
    expect(within(region).getByTestId('execution-rail-routing-scope')).toHaveTextContent('next turn');
    expect(within(region).getByTestId('execution-rail-routing').textContent).toBe(
      'Routes to deepseek/deepseek-flash (offline) — lowest cost that fits the mode',
    );
    expect(screen.queryByText(/intents/i)).toBeNull();
  });

  it('truncates the routing line and keeps the full text in its title', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={routing} {...old} />
      </LanguageProvider>,
    );
    const line = screen.getByTestId('execution-rail-routing');
    expect(line.className).toContain('truncate');
    expect(line).toHaveAttribute('title', line.textContent);
  });

  it('omits the dash when the server sent no reason', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={{ ...routing, reason: null }} {...old} />
      </LanguageProvider>,
    );
    expect(screen.getByTestId('execution-rail-routing').textContent).toBe('Routes to deepseek/deepseek-flash (offline)');
  });

  it('keeps alternatives and the model state behind a closed disclosure, with an explaining tooltip', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={routing} {...old} />
      </LanguageProvider>,
    );
    const details = screen.getByText('Why this model').closest('details')!;
    expect(details).not.toHaveAttribute('open');
    expect(details).toHaveTextContent('Alternatives: anthropic/claude-haiku, google/gemini-flash');
    const state = screen.getByTestId('execution-rail-routing-state');
    expect(details.contains(state)).toBe(true);
    expect(state).toHaveAttribute('title', expect.stringContaining('eligible for routing'));
  });

  it('clicking the routing line opens the Routing panel', () => {
    const onOpenRouting = vi.fn();
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={routing} onOpenRouting={onOpenRouting} {...old} />
      </LanguageProvider>,
    );
    fireEvent.click(screen.getByTestId('execution-rail-routing'));
    expect(onOpenRouting).toHaveBeenCalledTimes(1);
  });

  it('renders no Agents roster and no global Resources block even when handed their old inputs', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          agents={[{ id: 'a1', codename: 'Aquila', task: 'Refactor', phase: 'Idle' }]}
          activeModel="deepseek/deepseek-flash"
          openrouterSpendUsd={3.2}
          {...old}
        />
      </LanguageProvider>,
    );
    expect(screen.queryByRole('region', { name: /agent shards/i })).toBeNull();
    expect(screen.queryByLabelText('Resource strip')).toBeNull();
    for (const id of ['agents', 'queue', 'mesh', 'model', 'openrouter']) {
      expect(screen.queryByTestId(`execution-rail-${id}`)).toBeNull();
    }
  });

  it('keeps the one session-scoped number: Session spend', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} sessionId="sess-a" sessionSpentUsd={0.25} {...old} />
      </LanguageProvider>,
    );
    expect(screen.getByRole('region', { name: 'Session spend' })).toHaveTextContent('$0.25');
  });

  it('refreshes the context meter once per completed turn, not once per session', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation((cmd: string) =>
      cmd === 'get_context_budget' ? Promise.resolve(mockBudget) : Promise.resolve(null),
    );
    vi.mocked(invoke).mockClear();
    const budgetCalls = () => vi.mocked(invoke).mock.calls.filter(([c]) => c === 'get_context_budget').length;
    const ui = (turns: number) => (
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} sessionId="sess-a" turnsCompleted={turns} {...old} />
      </LanguageProvider>
    );
    const { rerender } = render(ui(0));
    await waitFor(() => expect(budgetCalls()).toBe(1));
    rerender(ui(1));
    await waitFor(() => expect(budgetCalls()).toBe(2));
    rerender(ui(1));
    rerender(ui(2));
    await waitFor(() => expect(budgetCalls()).toBe(3));
    expect(screen.getByRole('meter')).toBeInTheDocument();
  });

  it('Phase 5 lock chips still render under their task beside the Routing section (regression)', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[
            {
              id: 't1',
              title: 'Migrate orders table',
              status: 'running',
              lock: { resourceId: 'db://orders/42', state: 'holding' },
            },
          ]}
          routing={routing}
          {...old}
        />
      </LanguageProvider>,
    );
    const tasksRegion = screen.getByRole('region', { name: /active tasks/i });
    expect(within(tasksRegion).getByTestId('execution-rail-lock-chip')).toHaveTextContent('holding db://orders/42');
    expect(screen.getByRole('region', { name: 'Routing' })).toBeInTheDocument();
  });
});

describe('countCompletedTurns', () => {
  const m = (role: 'user' | 'assistant' | 'system', status: 'pending' | 'streaming' | 'done' | 'failed', i: number) => ({
    id: `m${i}`,
    role,
    text: '',
    status,
    runId: 'r',
  });

  it('counts finished assistant replies only (done or failed)', () => {
    expect(
      countCompletedTurns([
        m('user', 'done', 1),
        m('assistant', 'done', 2),
        m('assistant', 'failed', 3),
        m('assistant', 'streaming', 4),
        m('assistant', 'pending', 5),
        m('system', 'done', 6),
      ]),
    ).toBe(2);
    expect(countCompletedTurns([])).toBe(0);
  });
});


