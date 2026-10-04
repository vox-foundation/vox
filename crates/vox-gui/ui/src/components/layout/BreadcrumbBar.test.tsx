// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import React from 'react';
import { BreadcrumbBar } from './BreadcrumbBar';

describe('BreadcrumbBar', () => {
  it('renders parent and child for dashboard', () => {
    render(<BreadcrumbBar viewKey="dashboard" />);
    expect(screen.getByText('Agents')).toBeDefined();
    expect(screen.getByText('Dashboard')).toBeDefined();
  });

  it('hides on chat view', () => {
    const { container } = render(<BreadcrumbBar viewKey="chat" />);
    expect(container.firstChild).toBeNull();
  });

  it('renders Review › Runs for the self-child runs view without duplicate React keys', () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(<BreadcrumbBar viewKey="runs" onNavigate={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Navigate to Review' })).toBeDefined();
    expect(screen.getByText('Runs')).toBeDefined();
    const keyWarnings = err.mock.calls.filter((c) => String(c[0]).includes('same key'));
    err.mockRestore();
    expect(keyWarnings).toEqual([]);
  });

  it('calls onNavigate when parent segment clicked', async () => {
    const onNavigate = vi.fn();
    render(<BreadcrumbBar viewKey="console" onNavigate={onNavigate} />);
    const btn = screen.getByRole('button', { name: 'Navigate to Workspace' });
    btn.click();
    expect(onNavigate).toHaveBeenCalledWith('workspace');
  });
});
