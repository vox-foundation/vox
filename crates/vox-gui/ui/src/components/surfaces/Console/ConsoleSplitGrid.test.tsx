import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import React from 'react';
import {
  ConsoleSplitGrid,
  MAX_SPLIT_PANES,
  assignPaneSession,
  closePaneLeaf,
  countPaneLeaves,
  findFirstLeaf,
  findPaneLeaf,
  findPaneLeafBySession,
  splitPaneLeaf,
  type PaneNode,
} from './ConsoleSplitGrid';
import { ConsoleSessionBar } from './ConsoleSessionBar';

describe('ConsoleSplitGrid pure tree operations & react-resizable-panels v4 DOM', () => {
  beforeEach(() => cleanup());

  it('splits leaves horizontally and vertically up to MAX_SPLIT_PANES=4 and collapses cleanly on closePaneLeaf', () => {
    const root1: PaneNode = { kind: 'leaf', paneId: 'pane-1', sessionId: 's-1' };
    expect(countPaneLeaves(root1)).toBe(1);
    expect(findFirstLeaf(root1).paneId).toBe('pane-1');

    const root2 = splitPaneLeaf(root1, 'pane-1', 'horizontal', 'pane-2', 's-2');
    expect(countPaneLeaves(root2)).toBe(2);
    expect(root2.kind).toBe('split');
    if (root2.kind === 'split') {
      expect(root2.orientation).toBe('horizontal');
    }

    const root3 = splitPaneLeaf(root2, 'pane-2', 'vertical', 'pane-3', 's-3');
    expect(countPaneLeaves(root3)).toBe(3);

    const root4 = splitPaneLeaf(root3, 'pane-1', 'vertical', 'pane-4', 's-4');
    expect(countPaneLeaves(root4)).toBe(MAX_SPLIT_PANES);

    // 5th split beyond MAX_SPLIT_PANES returns unchanged reference
    const root5 = splitPaneLeaf(root4, 'pane-4', 'horizontal', 'pane-5', 's-5');
    expect(root5).toBe(root4);
    expect(countPaneLeaves(root5)).toBe(4);

    // Reassign session on a leaf
    const reassigned = assignPaneSession(root4, 'pane-3', 's-99');
    expect(findPaneLeaf(reassigned, 'pane-3')?.sessionId).toBe('s-99');
    expect(findPaneLeafBySession(reassigned, 's-99')?.paneId).toBe('pane-3');
    expect(findPaneLeafBySession(reassigned, 's-missing')).toBeNull();

    // Close pane-2 promotes pane-3 in place of split(pane-2, pane-3)
    const afterClose2 = closePaneLeaf(reassigned, 'pane-2');
    expect(countPaneLeaves(afterClose2)).toBe(3);
    expect(findPaneLeaf(afterClose2, 'pane-2')).toBeNull();
    expect(findPaneLeaf(afterClose2, 'pane-3')?.sessionId).toBe('s-99');
  });

  it('mounts ResizablePanelGroup, direct ResizablePanel and ResizableHandle children, active focus ring, and click selection', () => {
    const onSelectPane = vi.fn();
    let tree: PaneNode = { kind: 'leaf', paneId: 'pane-1', sessionId: 's-1' };
    tree = splitPaneLeaf(tree, 'pane-1', 'horizontal', 'pane-2', 's-2');
    tree = splitPaneLeaf(tree, 'pane-2', 'vertical', 'pane-3', 's-3');

    const { container } = render(
      <div style={{ width: 960, height: 600 }}>
        <ConsoleSplitGrid
          root={tree}
          activePaneId="pane-2"
          onSelectPane={onSelectPane}
          renderLeaf={(leaf) => <div data-testid={`leaf-body-${leaf.paneId}`}>{leaf.sessionId}</div>}
        />
      </div>,
    );

    const groups = container.querySelectorAll('[data-slot="resizable-panel-group"]');
    expect(groups.length).toBe(2);

    const panels = container.querySelectorAll('[data-slot="resizable-panel"]');
    expect(panels.length).toBe(4);

    const separators = screen.getAllByRole('separator');
    expect(separators.length).toBe(2);

    // Verify direct Panel/Separator children inside every Group (no intermediate wrapper div)
    groups.forEach((group) => {
      const childSlots = Array.from(group.children).map((el) => el.getAttribute('data-slot'));
      expect(childSlots).toEqual(['resizable-panel', 'resizable-handle', 'resizable-panel']);
    });

    const leaf1 = screen.getByTestId('console-split-leaf-pane-1');
    const leaf2 = screen.getByTestId('console-split-leaf-pane-2');
    expect(leaf1.getAttribute('data-active-pane')).toBe('false');
    expect(leaf2.getAttribute('data-active-pane')).toBe('true');

    fireEvent.click(leaf1);
    expect(onSelectPane).toHaveBeenCalledWith('pane-1');
  });
});

describe('ConsoleSessionBar non-nested sibling buttons, live labels, and two-step close confirmation', () => {
  beforeEach(() => cleanup());

  it('renders non-nested sibling tab and close buttons inside role="group" and formats fallback, cwd/git_branch, and running command labels', () => {
    const onSelectSession = vi.fn();
    const onCloseSession = vi.fn();
    const onSecurityLevelChange = vi.fn();
    const onCycleControlMode = vi.fn();

    const { container } = render(
      <ConsoleSessionBar
        sessions={[
          { id: 's-1', blocks: 0 },
          { id: 's-2', blocks: 3, cwd: '/Users/dev/vox', git_branch: 'feat/warp' },
          { id: 's-3', blocks: 1, cwd: '/Users/dev/vox' },
        ]}
        activeSessionId="s-1"
        runningCommandBySession={{ 's-3': 'cargo test -p vox-gui --lib' }}
        leafCount={1}
        securityLevel="l2_safe_exec"
        controlMode="manual"
        onSelectSession={onSelectSession}
        onNewSession={vi.fn()}
        onCloseSession={onCloseSession}
        onSplitRight={vi.fn()}
        onSplitDown={vi.fn()}
        onSecurityLevelChange={onSecurityLevelChange}
        onCycleControlMode={onCycleControlMode}
      />,
    );

    expect(screen.getByTestId('term-session-bar')).toBeTruthy();

    // Zero nested <button> inside <button> anywhere in session bar
    const nestedButtons = container.querySelectorAll('button button');
    expect(nestedButtons.length).toBe(0);

    // Sibling buttons inside role="group"
    const tab1 = screen.getByTestId('term-session-tab-s-1');
    const close1 = screen.getByTestId('term-session-close-s-1');
    expect(tab1.parentElement?.getAttribute('role')).toBe('group');
    expect(tab1.parentElement).toBe(close1.parentElement);
    expect(tab1.textContent).toContain('s-1 (0)');

    // Session 2 shows cwd basename + block count + git_branch
    const tab2 = screen.getByTestId('term-session-tab-s-2');
    expect(tab2.textContent).toContain('vox (3)');
    expect(tab2.textContent).toContain('feat/warp');

    // Session 3 shows truncated running command (20 chars + ...)
    const tab3 = screen.getByTestId('term-session-tab-s-3');
    expect(tab3.textContent).toContain('cargo test -p vox-gu... (1)');

    // Idle session closes in 1 click
    fireEvent.click(close1);
    expect(onCloseSession).toHaveBeenCalledWith('s-1');

    // Active-job session requires two-step inline confirmation
    onCloseSession.mockClear();
    fireEvent.click(screen.getByTestId('term-session-close-s-3'));
    expect(onCloseSession).not.toHaveBeenCalled();
    expect(
      screen.getByText('Foreground command still running. Terminate process and close session?'),
    ).toBeTruthy();

    // Keep Session Open cancels without closing
    fireEvent.click(screen.getByRole('button', { name: 'Keep Session Open' }));
    expect(
      screen.queryByText('Foreground command still running. Terminate process and close session?'),
    ).toBeNull();
    expect(onCloseSession).not.toHaveBeenCalled();

    // Confirm Close Session uses --color-status-fail token and closes the session
    fireEvent.click(screen.getByTestId('term-session-close-s-3'));
    const confirmBtn = screen.getByRole('button', { name: 'Confirm Close Session' });
    expect(confirmBtn.className).toContain('bg-(--color-status-fail)');
    expect(confirmBtn.className).not.toContain('bg-red-500');
    fireEvent.click(confirmBtn);
    expect(onCloseSession).toHaveBeenCalledWith('s-3');

    // Escape dismisses two-step confirmation without closing
    onCloseSession.mockClear();
    fireEvent.click(screen.getByTestId('term-session-close-s-3'));
    expect(screen.getByTestId('term-close-confirm-bar')).toBeTruthy();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByTestId('term-close-confirm-bar')).toBeNull();
    expect(onCloseSession).not.toHaveBeenCalled();

    // Security VoxSelect + Control Mode Pill callbacks
    const secSelect = screen.getByTestId('term-security-level-select');
    fireEvent.click(secSelect);
    fireEvent.click(screen.getByRole('option', { name: 'L3 Workspace Write' }));
    expect(onSecurityLevelChange).toHaveBeenCalledWith('l3_workspace_write');

    fireEvent.click(screen.getByTestId('term-control-mode-pill'));
    expect(onCycleControlMode).toHaveBeenCalledTimes(1);
  });
});
