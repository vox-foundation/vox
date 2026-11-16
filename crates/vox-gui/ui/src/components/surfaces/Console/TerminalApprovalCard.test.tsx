import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import {
  TerminalApprovalCard,
  TerminalPaneApprovalBanner,
} from './TerminalApprovalCard';
import { deriveSessionAllowPattern } from '../../../hooks/useTerminalPendingApprovals';
import type { PendingApprovalRow } from '../../../lib/mcpToolResult';

const sampleApproval: PendingApprovalRow = {
  approval_id: 'AP-445-1',
  tool: 'vox_terminal_exec',
  summary: 'Co-Pilot review - workspace command',
  requested_at_ms: 1700000000,
  session_id: 's-1',
  command: 'cargo test -p vox-gui',
  control_mode: 'co_pilot',
};

describe('TerminalApprovalCard & TerminalPaneApprovalBanner (TERM-AGENT-03)', () => {
  beforeEach(() => {
    cleanup();
  });

  it('derives token-safe session allow patterns and rejects unsafe metacharacters', () => {
    expect(deriveSessionAllowPattern('cargo test -p vox-gui')).toBe('cargo test');
    expect(deriveSessionAllowPattern('git status --short')).toBe('git status');
    expect(deriveSessionAllowPattern('ls -la /tmp')).toBe('ls');
    expect(deriveSessionAllowPattern('echo hi && rm -rf /')).toBe('echo');
    expect(deriveSessionAllowPattern('/usr/bin/env bash')).toBeNull();
    expect(deriveSessionAllowPattern('   ')).toBeNull();
  });

  it('renders semantic wax-seal signet pill without .5 spacing, font-bold, or non-ASCII glyphs and keeps inputs null when idle', () => {
    const { container } = render(
      <TerminalApprovalCard
        approval={sampleApproval}
        onResolve={vi.fn()}
      />,
    );

    const card = screen.getByTestId('term-approval-card');
    expect(card.getAttribute('role')).toBe('region');
    const seal = card.querySelector('[data-roman-wax-seal="true"]');
    expect(seal).toBeTruthy();
    expect(seal?.textContent).toBe('CO-PILOT GATE');
    expect(seal?.querySelector('svg')).toBeTruthy();

    expect(screen.getByTestId('term-approval-command').textContent).toBe('$ cargo test -p vox-gui');
    expect(screen.getByTestId('term-approval-pattern-pill').textContent).toContain('cargo test');

    expect(screen.queryByTestId('term-approval-edit-input')).toBeNull();
    expect(screen.queryByTestId('term-approval-steer-input')).toBeNull();
    expect(screen.queryAllByRole('textbox')).toHaveLength(0);

    const html = container.innerHTML;
    expect(html).not.toMatch(/\b(?:px|py|p|gap|size|mr|ml|mt|mb|pt|pb|space-y)-\d+\.5\b/);
    expect(html).not.toContain('font-bold');
    expect(card.textContent).not.toMatch(/[^\x20-\x7e]/);
  });

  it('renders SAFETY FLOOR GATE and CREDENTIAL GATE and disables Allow Pattern in Session', () => {
    const floorRow: PendingApprovalRow = {
      ...sampleApproval,
      approval_id: 'AP-floor',
      floor_category: 'history_rewrite',
      command: 'git push --force origin main',
    };
    const { rerender } = render(<TerminalApprovalCard approval={floorRow} onResolve={vi.fn()} />);
    expect(screen.getByTestId('term-approval-card').querySelector('[data-roman-wax-seal="true"]')?.textContent).toBe('SAFETY FLOOR GATE');
    expect(screen.getByTestId('term-approval-allow-pattern')).toBeDisabled();
    expect(screen.getByTestId('term-approval-pattern-pill').textContent).toContain('Single-command approval only');

    const credRow: PendingApprovalRow = {
      ...sampleApproval,
      approval_id: 'AP-cred',
      credential_prompt: true,
      command: 'sudo -S apt update',
    };
    rerender(<TerminalApprovalCard approval={credRow} onResolve={vi.fn()} />);
    expect(screen.getByTestId('term-approval-card').querySelector('[data-roman-wax-seal="true"]')?.textContent).toBe('CREDENTIAL GATE');
    expect(screen.getByTestId('term-approval-allow-pattern')).toBeDisabled();
  });

  it('handles Approve, lazy Edit & Run with onRecordCorrection, Allow Pattern in Session, and Reject + Steer', async () => {
    const onResolve = vi.fn().mockResolvedValue({ ok: true });
    const onRecordCorrection = vi.fn();

    render(
      <TerminalApprovalCard
        approval={sampleApproval}
        onResolve={onResolve}
        onRecordCorrection={onRecordCorrection}
      />,
    );

    fireEvent.click(screen.getByTestId('term-approval-approve'));
    await waitFor(() => expect(onResolve).toHaveBeenCalledWith(sampleApproval, { outcome: 'approved' }));

    fireEvent.click(screen.getByTestId('term-approval-allow-pattern'));
    await waitFor(() =>
      expect(onResolve).toHaveBeenCalledWith(sampleApproval, {
        outcome: 'approved',
        allowPattern: 'cargo test',
      }),
    );

    fireEvent.click(screen.getByTestId('term-approval-edit-toggle'));
    const editInput = screen.getByTestId('term-approval-edit-input') as HTMLTextAreaElement;
    expect(document.activeElement).toBe(editInput);
    const editPanel = screen.getByTestId('term-approval-edit-panel');
    const approveBtn = screen.getByTestId('term-approval-approve');
    expect(approveBtn.compareDocumentPosition(editPanel) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    fireEvent.keyDown(editInput, { key: 'Escape' });
    expect(screen.queryByTestId('term-approval-edit-input')).toBeNull();

    fireEvent.click(screen.getByTestId('term-approval-edit-toggle'));
    const editInputReopened = screen.getByTestId('term-approval-edit-input') as HTMLTextAreaElement;
    expect(editInputReopened.value).toBe('cargo test -p vox-gui');
    fireEvent.change(editInputReopened, { target: { value: 'cargo test -p vox-terminal-core' } });
    fireEvent.click(screen.getByTestId('term-approval-edit-submit'));
    await waitFor(() =>
      expect(onResolve).toHaveBeenCalledWith(sampleApproval, {
        outcome: 'modified',
        modifiedCommand: 'cargo test -p vox-terminal-core',
      }),
    );
    expect(onRecordCorrection).toHaveBeenCalledWith(
      'cargo test -p vox-gui',
      'cargo test -p vox-terminal-core',
    );
    expect(screen.queryByTestId('term-approval-edit-input')).toBeNull();

    fireEvent.click(screen.getByTestId('term-approval-reject-toggle'));
    const steerInput = screen.getByTestId('term-approval-steer-input') as HTMLInputElement;
    fireEvent.change(steerInput, { target: { value: 'run clippy instead' } });
    fireEvent.click(screen.getByTestId('term-approval-reject-submit'));
    await waitFor(() =>
      expect(onResolve).toHaveBeenCalledWith(sampleApproval, {
        outcome: 'rejected',
        steeringNote: 'run clippy instead',
      }),
    );
    expect(screen.queryByTestId('term-approval-steer-input')).toBeNull();
  });

  it('keeps Edit & Run open and shows inline AST error when resolve fails', async () => {
    const onResolve = vi.fn().mockResolvedValue({
      ok: false,
      error: 'Edited command blocked by ExecPolicy (deny-subshell): subshell forbidden.',
    });
    render(<TerminalApprovalCard approval={sampleApproval} onResolve={onResolve} />);

    fireEvent.click(screen.getByTestId('term-approval-edit-toggle'));
    const editInput = screen.getByTestId('term-approval-edit-input');
    fireEvent.change(editInput, { target: { value: 'cargo test && rm -rf /' } });
    fireEvent.click(screen.getByTestId('term-approval-edit-submit'));

    expect(await screen.findByTestId('term-approval-edit-error')).toHaveTextContent('ExecPolicy');
    expect(screen.getByTestId('term-approval-edit-input')).toBeTruthy();

    fireEvent.click(screen.getByTestId('term-approval-edit-cancel'));
    expect(screen.queryByTestId('term-approval-edit-input')).toBeNull();
  });

  it('renders Autopilot preemption banner and queue count in TerminalPaneApprovalBanner', async () => {
    const onResume = vi.fn();
    const onDismiss = vi.fn();
    render(
      <TerminalPaneApprovalBanner
        approvals={[
          sampleApproval,
          { ...sampleApproval, approval_id: 'AP-445-2', command: 'cargo clippy' },
        ]}
        preemptedBanner
        onResumeCoPilot={onResume}
        onDismissPreemptedBanner={onDismiss}
        onResolveApproval={vi.fn().mockResolvedValue({ ok: true })}
      />,
    );

    const banner = screen.getByTestId('term-autopilot-preempted-banner');
    expect(banner.getAttribute('role')).toBe('status');
    expect(banner.textContent).toContain(
      'Autopilot preempted to Manual on operator input - queued agent commands paused.',
    );
    expect(screen.getByTestId('term-approval-tray')).toBeTruthy();
    expect(screen.getByTestId('term-approval-queue-count').textContent).toBe('1 of 2 pending');

    fireEvent.click(screen.getByTestId('term-preempt-resume-copilot'));
    expect(onResume).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId('term-preempt-dismiss'));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
