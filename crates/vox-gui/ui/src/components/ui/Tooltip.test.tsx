// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import React from 'react';
import { Tooltip } from './Tooltip';

describe('Tooltip', () => {
  it('renders trigger children cleanly', () => {
    render(
      <Tooltip content="Helper text">
        <button type="button">Hover me</button>
      </Tooltip>
    );
    expect(screen.getByRole('button', { name: 'Hover me' })).toBeInTheDocument();
  });

  it('renders just children when content is empty', () => {
    render(
      <Tooltip content="">
        <button type="button">No tooltip</button>
      </Tooltip>
    );
    expect(screen.getByRole('button', { name: 'No tooltip' })).toBeInTheDocument();
  });

  it('reveals tooltip content on focus/hover with default delay 0 in test mode', async () => {
    render(
      <Tooltip content="Helper text" delayDuration={0}>
        <button type="button">Hover me</button>
      </Tooltip>
    );
    const trigger = screen.getByRole('button', { name: 'Hover me' });
    trigger.focus();
    expect(await screen.findByText('Helper text')).toBeInTheDocument();
  });
});
