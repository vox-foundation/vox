// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/react';
import React from 'react';
import { NotificationCenter } from './NotificationCenter';
import type { Notice } from '../../lib/noticeStore';

const n = (id: string, severity: Notice['severity'], title: string, extra: Partial<Notice> = {}): Notice => ({
  id, groupKey: id, severity, scope: 'engine', source: 'engine', title,
  count: 1, lastAtMs: 0, read: false, ...extra,
});

const notices = [
  n('1', 'error', 'FAILED · task 7', { count: 2, body: 'error: boom' }),
  n('2', 'success', 'Saved', { scope: 'action', source: 'backend-ok', read: true }),
];

const bell = (name: string | RegExp = /^Notifications/) => screen.getByRole('button', { name });

describe('NotificationCenter', () => {
  it('the bell names the number of unread problems', () => {
    render(<NotificationCenter notices={notices} onMarkAllRead={vi.fn()} />);
    expect(bell('Notifications, 1 need attention').getAttribute('aria-expanded')).toBe('false');
    expect(screen.getByTestId('notification-unread').textContent).toBe('1');
  });

  it('shows no count when nothing needs attention, keeping the count slot so the bell does not shift', () => {
    render(<NotificationCenter notices={[notices[1]]} onMarkAllRead={vi.fn()} />);
    expect(screen.queryByTestId('notification-unread')).toBeNull();
    expect(bell('Notifications').querySelector('[data-slot="count"]')).not.toBeNull();
  });

  it('opens a focused drawer that lists notices with counts and filters to problems', () => {
    render(<NotificationCenter notices={notices} onMarkAllRead={vi.fn()} />);
    fireEvent.click(bell());
    expect(bell().getAttribute('aria-expanded')).toBe('true');
    const dialog = screen.getByRole('dialog', { name: 'Notifications' });
    expect(document.activeElement).toBe(dialog);
    const items = within(dialog).getAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(items[0].getAttribute('data-severity')).toBe('error');
    expect(within(items[0]).getByText('×2')).toBeTruthy();
    fireEvent.click(within(dialog).getByRole('radio', { name: 'Problems' }));
    expect(within(dialog).getAllByRole('listitem')).toHaveLength(1);
  });

  it('marks all read; Escape closes and returns focus to the bell', () => {
    const onMarkAllRead = vi.fn();
    render(<NotificationCenter notices={notices} onMarkAllRead={onMarkAllRead} />);
    fireEvent.click(bell());
    fireEvent.click(screen.getByRole('button', { name: 'Mark all read' }));
    expect(onMarkAllRead).toHaveBeenCalled();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(bell());
  });

  it('closes on a click outside', () => {
    render(<div><span data-testid="outside" /><NotificationCenter notices={notices} onMarkAllRead={vi.fn()} /></div>);
    fireEvent.click(bell());
    fireEvent.mouseDown(screen.getByTestId('outside'));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('says so when there is nothing to report', () => {
    render(<NotificationCenter notices={[]} onMarkAllRead={vi.fn()} />);
    fireEvent.click(bell());
    expect(screen.getByText('Nothing to report')).toBeTruthy();
  });
});
