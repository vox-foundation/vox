import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import React from 'react';

vi.mock('./ScientiaDashboard', () => ({
  ScientiaDashboard: () => <div data-testid="scientia-dashboard" />,
}));
vi.mock('./ClaimsView', () => ({
  ClaimsView: () => <div data-testid="scientia-claims" />,
}));
vi.mock('./DiscoveryInbox', () => ({
  DiscoveryInbox: () => <div data-testid="scientia-inbox" />,
}));
vi.mock('./DiscoveryReview', () => ({
  DiscoveryReview: () => <div data-testid="scientia-studio" />,
}));

import { ScientiaSurface } from './ScientiaSurface';

describe('ScientiaSurface (Findings)', () => {
  it('defaults to the dashboard and exposes Inbox, Studio & Review, and Claims tabs', () => {
    render(<ScientiaSurface pushToast={() => {}} />);
    expect(screen.getByTestId('scientia-dashboard')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: 'Inbox' }));
    expect(screen.getByTestId('scientia-inbox')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: 'Studio & Review' }));
    expect(screen.getByTestId('scientia-studio')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: 'Claims' }));
    expect(screen.getByTestId('scientia-claims')).toBeInTheDocument();
  });
});
