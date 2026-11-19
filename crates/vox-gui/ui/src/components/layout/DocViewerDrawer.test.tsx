import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { DocViewerDrawer } from './DocViewerDrawer';

vi.mock('../surfaces/DocReader/DocReader', () => ({
  DocReader: ({ path }: { path: string }) => <div data-testid="doc-reader-stub">{path}</div>,
}));

describe('DocViewerDrawer', () => {
  it('renders nothing when doc is null', () => {
    const { container } = render(
      <DocViewerDrawer doc={null} onClose={vi.fn()} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders a non-modal docked workspace with title, width/minimize controls, and close button', () => {
    render(
      <DocViewerDrawer doc={{ path: 'docs/foo.md', title: 'Foo Guide' }} onClose={vi.fn()} />,
    );
    const dock = screen.getByRole('dialog', { name: 'Foo Guide' });
    expect(dock).toBeInTheDocument();
    expect(dock.getAttribute('aria-modal')).toBe('false');
    expect(document.querySelector('[data-slot="drawer-backdrop"]')).toBeNull();
    expect(screen.getByTestId('doc-reader-stub')).toHaveTextContent('docs/foo.md');
    expect(screen.getByTestId('doc-dock-width-btn')).toHaveTextContent('Width: Split');
    expect(screen.getByTestId('doc-dock-minimize-btn')).toHaveTextContent('Minimize');
  });

  it('cycles docked width modes and supports minimizing without losing the doc state', () => {
    render(
      <DocViewerDrawer doc={{ path: 'docs/foo.md', title: 'Foo Guide' }} onClose={vi.fn()} />,
    );
    const widthBtn = screen.getByTestId('doc-dock-width-btn');
    fireEvent.click(widthBtn);
    expect(widthBtn).toHaveTextContent('Width: Wide');
    fireEvent.click(widthBtn);
    expect(widthBtn).toHaveTextContent('Width: Full');

    const minBtn = screen.getByTestId('doc-dock-minimize-btn');
    fireEvent.click(minBtn);
    expect(screen.queryByTestId('doc-reader-stub')).toBeNull();
    expect(minBtn).toHaveTextContent('Expand');

    fireEvent.click(minBtn);
    expect(screen.getByTestId('doc-reader-stub')).toHaveTextContent('docs/foo.md');
  });

  it('calls onClose when the close button or Escape key is pressed', () => {
    const onClose = vi.fn();
    render(
      <DocViewerDrawer doc={{ path: 'docs/foo.md', title: 'Foo Guide' }} onClose={onClose} />,
    );
    fireEvent.keyDown(window, { key: 'Escape' });
    fireEvent.click(screen.getByRole('button', { name: 'Close doc' }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it('moves focus into the dock on open and restores focus to the trigger element after close', () => {
    function Harness() {
      const [open, setOpen] = React.useState(false);
      return (
        <div>
          <button type="button" onClick={() => setOpen(true)}>
            Open doc
          </button>
          <DocViewerDrawer
            doc={open ? { path: 'docs/foo.md', title: 'Foo Guide' } : null}
            onClose={() => setOpen(false)}
          />
        </div>
      );
    }

    render(<Harness />);
    const trigger = screen.getByRole('button', { name: 'Open doc' });
    trigger.focus();
    fireEvent.click(trigger);

    expect(screen.getByRole('button', { name: 'Close doc' })).toHaveFocus();

    fireEvent.click(screen.getByRole('button', { name: 'Close doc' }));

    expect(trigger).toHaveFocus();
  });
});
