// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { ResearchReportMarkdown } from './ResearchReportMarkdown';

describe('ResearchReportMarkdown', () => {
  it('renders null when markdown is empty', () => {
    const { container } = render(<ResearchReportMarkdown markdown="" />);
    expect(container.firstChild).toBeNull();
  });

  it('renders headings of different levels', () => {
    const md = `# Heading 1\n## Heading 2\n### Heading 3`;
    render(<ResearchReportMarkdown markdown={md} />);

    expect(screen.getByRole('heading', { level: 1, name: 'Heading 1' })).toBeTruthy();
    expect(screen.getByRole('heading', { level: 2, name: 'Heading 2' })).toBeTruthy();
    expect(screen.getByRole('heading', { level: 3, name: 'Heading 3' })).toBeTruthy();
  });

  it('renders unordered and ordered lists', () => {
    const md = `- Item A\n- Item B\n\n1. Step 1\n2. Step 2`;
    render(<ResearchReportMarkdown markdown={md} />);

    expect(screen.getByText('Item A')).toBeTruthy();
    expect(screen.getByText('Item B')).toBeTruthy();
    expect(screen.getByText('Step 1')).toBeTruthy();
    expect(screen.getByText('Step 2')).toBeTruthy();
    expect(screen.getAllByRole('list').length).toBe(2);
  });

  it('renders bold text and inline code', () => {
    const md = `This has **strong text** and \`inline_code\` inside a paragraph.`;
    render(<ResearchReportMarkdown markdown={md} />);

    expect(screen.getByText('strong text').tagName).toBe('STRONG');
    expect(screen.getByText('inline_code').tagName).toBe('CODE');
  });

  it('renders fenced code blocks', () => {
    const md = "```ts\nconst answer = 42;\nconsole.log(answer);\n```";
    render(<ResearchReportMarkdown markdown={md} />);

    expect(screen.getByText(/const answer = 42;/)).toBeTruthy();
  });

  it('renders citation references as clickable buttons with aria-label', async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();
    const md = `Vox provides verifiable research [1] and robust synthesis [2].`;

    render(<ResearchReportMarkdown markdown={md} onCitationClick={onClick} />);

    const cite1 = screen.getByRole('button', { name: 'Citation 1' });
    const cite2 = screen.getByRole('button', { name: 'Citation 2' });

    expect(cite1).toBeTruthy();
    expect(cite1.textContent).toBe('[1]');
    expect(cite2).toBeTruthy();
    expect(cite2.textContent).toBe('[2]');

    await user.click(cite1);
    expect(onClick).toHaveBeenCalledWith(1);

    await user.click(cite2);
    expect(onClick).toHaveBeenCalledWith(2);
  });

  it('handles multi-citation syntax like [1, 2]', async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();
    const md = `Multiple sources corroborate this claim [1, 2].`;

    render(<ResearchReportMarkdown markdown={md} onCitationClick={onClick} />);

    const cite1 = screen.getByRole('button', { name: 'Citation 1' });
    const cite2 = screen.getByRole('button', { name: 'Citation 2' });

    expect(cite1).toBeTruthy();
    expect(cite2).toBeTruthy();

    await user.click(cite2);
    expect(onClick).toHaveBeenCalledWith(2);
  });

  it('does not confuse markdown links with citations', () => {
    const md = `Check the [documentation](https://vox.foundation) for details.`;
    render(<ResearchReportMarkdown markdown={md} />);

    const link = screen.getByRole('link', { name: 'documentation' });
    expect(link.getAttribute('href')).toBe('https://vox.foundation');
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('sanitizes unsafe link protocols like javascript: to prevent XSS', () => {
    const md = `Click here for [malicious link](javascript:alert(1)) payload.`;
    render(<ResearchReportMarkdown markdown={md} />);

    expect(screen.queryByRole('link')).toBeNull();
    expect(screen.getByText(/malicious link/)).toBeTruthy();
  });
});
