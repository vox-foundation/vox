// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import React from 'react';
import { BreadcrumbBar } from './BreadcrumbBar';

describe('BreadcrumbBar', () => {
  it('renders parent and child for dashboard', () => {
    render(<BreadcrumbBar viewKey="dashboard" />);
    expect(screen.getByText('Agents')).toBeDefined();
    expect(screen.getAllByText('Dashboard').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByRole('tab', { name: 'Flow' })).toBeDefined();
  });

  it('hides on chat view', () => {
    const { container } = render(<BreadcrumbBar viewKey="chat" />);
    expect(container.firstChild).toBeNull();
  });

  it('calls onNavigate when parent segment clicked', async () => {
    const onNavigate = vi.fn();
    render(<BreadcrumbBar viewKey="console" onNavigate={onNavigate} />);
    const btn = screen.getByRole('button', { name: 'Navigate to Workspace' });
    btn.click();
    expect(onNavigate).toHaveBeenCalledWith('workspace');
  });

  it('calls onNavigate when subsection tab clicked', async () => {
    const onNavigate = vi.fn();
    render(<BreadcrumbBar viewKey="memory" onNavigate={onNavigate} />);
    const researchTab = screen.getByRole('tab', { name: 'Research' });
    researchTab.click();
    expect(onNavigate).toHaveBeenCalledWith('research');
  });
});
