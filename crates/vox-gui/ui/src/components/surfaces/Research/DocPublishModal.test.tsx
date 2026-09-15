// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';

const mockDraft = {
  slug: 'session-42-research',
  title: 'Research Session #42 Architecture SSOT (2026)',
  filename: 'session-42-research-2026.md',
  markdown_content: `---
title: "Research Session #42 Architecture SSOT (2026)"
description: "Empirically verified architecture findings and benchmarks."
category: "Architecture SSOTs"
status: "current"
---

# Research Session #42 Architecture SSOT (2026)

## 1. Executive Summary & Codebase Reality
Findings from session 42.
`,
  is_valid: true,
  validation_errors: [],
};

const mockPublishResult = {
  file_path: '/path/to/docs/src/architecture/session-42-research-2026.md',
  relative_path: 'docs/src/architecture/session-42-research-2026.md',
  indexed: true,
};

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

import { DocPublishModal } from './DocPublishModal';

describe('DocPublishModal', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string, args?: any) => {
      if (cmd === 'generate_research_doc_draft') {
        return Promise.resolve(mockDraft);
      }
      if (cmd === 'publish_research_doc') {
        return Promise.resolve(mockPublishResult);
      }
      return Promise.resolve(null);
    });
  });

  it('renders nothing when isOpen is false', () => {
    const { container } = render(
      <DocPublishModal
        sessionId={42}
        isOpen={false}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );
    expect(container.firstChild).toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('calls generate_research_doc_draft on open and renders preview', async () => {
    render(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );

    expect(invokeMock).toHaveBeenCalledWith('generate_research_doc_draft', { sessionId: 42 });

    await waitFor(() => {
      expect(screen.getByText('Publish Architecture SSOT')).toBeTruthy();
    });

    await waitFor(() => {
      expect(screen.getByDisplayValue('session-42-research')).toBeTruthy();
      expect(screen.getByText(/Findings from session 42/i)).toBeTruthy();
    });
  });

  it('switches tabs between Preview, Raw Markdown, and Validation', async () => {
    render(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('session-42-research')).toBeTruthy();
    });

    // Click Raw Markdown tab
    const rawTab = screen.getByRole('button', { name: /Raw Markdown/i });
    fireEvent.click(rawTab);

    const textarea = screen.getByRole('textbox', { name: /Raw Markdown Content/i }) as HTMLTextAreaElement;
    expect(textarea).toBeTruthy();
    expect(textarea.value).toContain('title: "Research Session #42 Architecture SSOT (2026)"');

    // Click Validation tab
    const valTab = screen.getByRole('button', { name: /Validation/i });
    fireEvent.click(valTab);

    expect(screen.getByText(/Frontmatter Validation/i)).toBeTruthy();
    expect(screen.getByText(/YAML frontmatter fences/i)).toBeTruthy();
    expect(screen.getByText(/Title defined/i)).toBeTruthy();
    expect(screen.getByText(/Description defined/i)).toBeTruthy();
    expect(screen.getByText(/Category is "Architecture SSOTs"/i)).toBeTruthy();
    expect(screen.getByText(/Status is "current"/i)).toBeTruthy();
  });

  it('allows editing slug and textarea content with reset to original', async () => {
    render(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('session-42-research')).toBeTruthy();
    });

    // Edit slug
    const slugInput = screen.getByDisplayValue('session-42-research') as HTMLInputElement;
    fireEvent.change(slugInput, { target: { value: 'custom-slug-edited' } });
    expect(slugInput.value).toBe('custom-slug-edited');

    // Switch to Raw Markdown and edit content
    const rawTab = screen.getByRole('button', { name: /Raw Markdown/i });
    fireEvent.click(rawTab);
    const textarea = screen.getByRole('textbox', { name: /Raw Markdown Content/i }) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'Modified content without frontmatter' } });
    expect(textarea.value).toBe('Modified content without frontmatter');

    // Click Reset
    const resetButton = screen.getByRole('button', { name: /Reset/i });
    fireEvent.click(resetButton);

    // Verify reset restores original
    expect(slugInput.value).toBe('session-42-research');
    expect(textarea.value).toContain('title: "Research Session #42 Architecture SSOT (2026)"');
  });

  it('publishes doc and calls onPublished callback with relative path', async () => {
    const onPublished = vi.fn();
    const onClose = vi.fn();

    render(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={onClose}
        onPublished={onPublished}
      />
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('session-42-research')).toBeTruthy();
    });

    const publishButton = screen.getByRole('button', { name: /Approve & Publish/i });
    fireEvent.click(publishButton);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('publish_research_doc', {
        sessionId: 42,
        slug: 'session-42-research',
        content: mockDraft.markdown_content,
      });
      expect(onPublished).toHaveBeenCalledWith('docs/src/architecture/session-42-research-2026.md');
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('all buttons have type="button"', async () => {
    render(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('session-42-research')).toBeTruthy();
    });

    for (const b of screen.getAllByRole('button')) {
      expect(b.getAttribute('type')).toBe('button');
    }
  });

  it('safely toggles isOpen from false to true to false without hook violation', async () => {
    const { rerender, container } = render(
      <DocPublishModal
        sessionId={42}
        isOpen={false}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );
    expect(container.firstChild).toBeNull();

    rerender(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );
    await waitFor(() => {
      expect(screen.getByText('Publish Architecture SSOT')).toBeTruthy();
    });

    rerender(
      <DocPublishModal
        sessionId={42}
        isOpen={false}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );
    expect(screen.queryByText('Publish Architecture SSOT')).toBeNull();
  });

  it('sanitizes slug input to lowercase alphanumeric and hyphens', async () => {
    render(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('session-42-research')).toBeTruthy();
    });

    const slugInput = screen.getByLabelText(/Slug:/i) as HTMLInputElement;
    fireEvent.change(slugInput, { target: { value: 'My_New Slug & Feature!' } });

    expect(slugInput.value).toBe('my-new-slug-feature-');
    expect(screen.getByText(/my-new-slug-feature-2026\.md/)).toBeTruthy();
  });

  it('marks empty quotes in title as failing validation', async () => {
    render(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('session-42-research')).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /Raw Markdown/i }));
    const textarea = screen.getByLabelText('Raw Markdown Content') as HTMLTextAreaElement;
    fireEvent.change(textarea, {
      target: {
        value: `---
title: ""
description: "Some valid description"
category: "Architecture SSOTs"
status: "current"
---
# Content
`,
      },
    });

    fireEvent.click(screen.getByRole('button', { name: /Validation/i }));
    expect(screen.getByText('Checks Failing')).toBeTruthy();
  });
});
