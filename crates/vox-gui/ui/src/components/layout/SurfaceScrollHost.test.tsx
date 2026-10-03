// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SurfaceScrollHost } from './SurfaceScrollHost';

describe('SurfaceScrollHost', () => {
  it('makes the scrolling viewport a named, keyboard-focusable region (axe scrollable-region-focusable)', () => {
    render(
      <SurfaceScrollHost>
        <p>no focusable content here</p>
      </SurfaceScrollHost>,
    );
    const viewport = screen.getByRole('region', { name: 'Surface content' });
    expect(viewport.getAttribute('tabindex')).toBe('0');
    expect(viewport).toHaveTextContent('no focusable content here');
  });
});
