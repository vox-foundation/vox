// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import React from 'react';
import { EmptyState } from './EmptyState';

describe('EmptyState Primitive', () => {
  it('renders default text based on variant type', () => {
    render(<EmptyState variant="no-permission" title="Denied" />);
    expect(screen.getByText('Denied')).toBeInTheDocument();
  });

  it('triggers primary and secondary callbacks on click', () => {
    const onPrimary = vi.fn();
    const onSecondary = vi.fn();
    render(
      <EmptyState 
        title="Empty"
        primaryAction={{ label: 'Save', onClick: onPrimary }}
        secondaryAction={{ label: 'Cancel', onClick: onSecondary }}
      />
    );
    fireEvent.click(screen.getByText('Save'));
    fireEvent.click(screen.getByText('Cancel'));
    expect(onPrimary).toHaveBeenCalledTimes(1);
    expect(onSecondary).toHaveBeenCalledTimes(1);
  });

  it('renders the title as h3 by default and as h2 when headingLevel is 2', () => {
    const { unmount } = render(<EmptyState title="Default level" />);
    expect(screen.getByRole('heading', { level: 3, name: 'Default level' })).toBeInTheDocument();
    unmount();
    render(<EmptyState title="Raised" headingLevel={2} />);
    expect(screen.getByRole('heading', { level: 2, name: 'Raised' })).toBeInTheDocument();
  });
});
