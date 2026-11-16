import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import React from 'react';
import { AgentTurnContent } from './AgentTurnContent';
import type { TermBlockDto } from '../../../transport';

function makeAgentBlock(overrides: Partial<TermBlockDto> = {}): TermBlockDto {
  return {
    id: 40,
    kind: 'AgentTurn',
    status: 'Ok',
    origin: { kind: 'agent', agent_id: 'terminal-agent' },
    input: '? check listening ports',
    output: [],
    exit_code: 0,
    started_ms: 100,
    ended_ms: 200,
    ...overrides,
  };
}

describe('AgentTurnContent (TERM-AGENT-01, TERM-AGENT-02)', () => {
  beforeEach(() => {
    cleanup();
    window.localStorage.clear();
  });

  it('shows streaming status pulse and Stop Stream when Running with empty output chunks', () => {
    const onCancel = vi.fn();
    render(
      <AgentTurnContent
        block={makeAgentBlock({ status: 'Running', exit_code: null, ended_ms: null, output: [] })}
        onCancelAgentTurn={onCancel}
      />,
    );

    const root = screen.getByTestId('term-agent-turn-content');
    expect(root.getAttribute('aria-busy')).toBe('true');
    expect(root.className).toContain('max-h-80');
    expect(root.className).toContain('overflow-y-auto');
    expect(screen.getByTestId('term-agent-streaming-status').textContent).toContain(
      'Routing prompt through model registry...',
    );
    const stopBtn = screen.getByRole('button', { name: /Stop Stream/i });
    fireEvent.click(stopBtn);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('shows Streaming response from {model} while streaming non-empty chunks and renders semantic ul/ol lists', () => {
    const { container } = render(
      <AgentTurnContent
        block={makeAgentBlock({
          status: 'Running',
          exit_code: null,
          ended_ms: null,
          output: [
            {
              stream: 'Agent',
              text: '[agent-model: openrouter - claude-sonnet]\n- Check open sockets\n- Inspect PID\n1. Run lsof\n',
              ts_ms: 110,
            },
          ],
        })}
      />,
    );

    expect(screen.getByTestId('term-agent-streaming-status').textContent).toContain(
      'Streaming response from openrouter/claude-sonnet...',
    );
    expect(container.querySelectorAll('ul li')).toHaveLength(2);
    expect(container.querySelectorAll('ol li')).toHaveLength(1);
  });

  it('parses [agent-model: ...] pill, prose, fenced commands, and VOX_PROPOSED_COMMAND for general prompts', () => {
    const onRun = vi.fn();
    const onEdit = vi.fn();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    const { container } = render(
      <AgentTurnContent
        block={makeAgentBlock({
          output: [
            {
              stream: 'Agent',
              text: '[agent-model: openrouter - claude-sonnet]\nUse `lsof` to inspect ports:\n```sh\nlsof -iTCP -sTCP:LISTEN\n```\nVOX_PROPOSED_COMMAND: ss -tulpn\n',
              ts_ms: 110,
            },
          ],
        })}
        onRunProposedCommand={onRun}
        onEditInPrompt={onEdit}
      />,
    );

    expect(screen.getByTestId('term-agent-model-pill').textContent).toContain(
      'openrouter/claude-sonnet',
    );
    const cards = screen.getAllByTestId('term-proposed-command-card');
    expect(cards).toHaveLength(2);

    const runBtns = screen.getAllByTestId('term-proposed-cmd-run');
    const editBtns = screen.getAllByTestId('term-proposed-cmd-edit');
    const copyBtns = screen.getAllByTestId('term-proposed-cmd-copy');

    fireEvent.click(runBtns[0]!);
    expect(onRun).toHaveBeenCalledWith('lsof -iTCP -sTCP:LISTEN');

    fireEvent.click(editBtns[1]!);
    expect(onEdit).toHaveBeenCalledWith('ss -tulpn');

    fireEvent.click(copyBtns[0]!);
    expect(writeText).toHaveBeenCalledWith('lsof -iTCP -sTCP:LISTEN');

    expect(container.innerHTML).not.toMatch(/\b(?:px|py|p|gap|size|mr|ml|mt|mb|pt|pb|space-y)-\d+\.5\b/);
    expect(container.innerHTML).not.toContain('font-bold');
    expect(container.textContent).not.toMatch(/[^\x20-\x7e\n]/);
  });

  it('renders Run Fix and Edit in Prompt on ? fix / ? explain repair turns', () => {
    const onRun = vi.fn();
    const onEdit = vi.fn();
    render(
      <AgentTurnContent
        block={makeAgentBlock({
          input: '? fix block #12',
          output: [
            {
              stream: 'Agent',
              text: 'Add the missing crate feature:\n```bash\ncargo check -p vox-cli --all-features\n```\n',
              ts_ms: 120,
            },
          ],
        })}
        onRunProposedCommand={onRun}
        onEditInPrompt={onEdit}
      />,
    );

    fireEvent.click(screen.getByTestId('term-proposed-fix-run'));
    expect(onRun).toHaveBeenCalledWith('cargo check -p vox-cli --all-features');

    fireEvent.click(screen.getByTestId('term-proposed-fix-edit'));
    expect(onEdit).toHaveBeenCalledWith('cargo check -p vox-cli --all-features');
  });

  it('renders Clavis unconfigured diagnostic card with Open Keys & Secrets and Run Secrets Doctor', () => {
    const onOpenSecrets = vi.fn();
    const onRunDoctor = vi.fn();
    const navEvents: CustomEvent[] = [];
    const onNav = (e: Event) => navEvents.push(e as CustomEvent);
    window.addEventListener('vox://navigate-surface', onNav);

    try {
      render(
        <AgentTurnContent
          block={makeAgentBlock({
            status: 'Failed',
            exit_code: 1,
            output: [
              {
                stream: 'Agent',
                text: '[clavis-unconfigured] No LLM provider or local MENS endpoint configured.\nRun `vox secrets doctor` or configure a key.',
                ts_ms: 130,
              },
            ],
          })}
          onOpenSecrets={onOpenSecrets}
          onRunSecretsDoctor={onRunDoctor}
        />,
      );

      const diag = screen.getByTestId('term-agent-clavis-diagnostic');
      expect(diag.textContent).toContain('No LLM Provider Configured');

      fireEvent.click(screen.getByTestId('term-clavis-open-secrets'));
      expect(onOpenSecrets).toHaveBeenCalledTimes(1);
      expect(window.localStorage.getItem('vox_settings_seed')).toBe(
        JSON.stringify({ section: 'secrets' }),
      );
      expect(navEvents.some((e) => e.detail?.view === 'settings')).toBe(true);

      fireEvent.click(screen.getByTestId('term-clavis-run-doctor'));
      expect(onRunDoctor).toHaveBeenCalledTimes(1);
    } finally {
      window.removeEventListener('vox://navigate-surface', onNav);
    }
  });

  it('renders stream interruption banner with Retry Agent Turn on [stream-error]', () => {
    const onRetry = vi.fn();
    const blk = makeAgentBlock({
      status: 'Failed',
      exit_code: 1,
      output: [{ stream: 'Agent', text: '[stream-error] upstream timed out', ts_ms: 140 }],
    });
    render(<AgentTurnContent block={blk} onRetryAgentTurn={onRetry} />);

    expect(screen.getByTestId('term-agent-stream-error').textContent).toContain('upstream timed out');
    fireEvent.click(screen.getByTestId('term-agent-retry'));
    expect(onRetry).toHaveBeenCalledWith(blk);
  });
});
