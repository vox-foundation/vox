import React, { type ReactNode } from 'react';

export interface ResearchReportMarkdownProps {
  markdown: string;
  onCitationClick?: (citationNum: number) => void;
  className?: string;
}

/**
 * Parses inline markdown:
 * - Code: `code`
 * - Bold: **bold**
 * - Citations: [1], [2], [1, 2]
 * - Links: [label](url)
 */
function renderInline(
  text: string,
  onCitationClick?: (num: number) => void,
  keyPrefix = 'inline'
): ReactNode[] {
  const tokenRegex = /(`[^`]+`|\*\*[^*]+\*\*|\[\d+(?:\s*,\s*\d+)*\](?!\()|\[[^\]]+\]\([^)]+\))/g;

  const elements: ReactNode[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let count = 0;

  while ((match = tokenRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      elements.push(text.slice(lastIndex, match.index));
    }
    const token = match[0];
    const key = `${keyPrefix}-${count++}`;

    if (token.startsWith('`') && token.endsWith('`')) {
      const code = token.slice(1, -1);
      elements.push(
        <code
          key={key}
          className="rounded bg-overlay-subtle px-1 py-0.5 font-mono text-xs text-brass"
        >
          {code}
        </code>
      );
    } else if (token.startsWith('**') && token.endsWith('**')) {
      const boldText = token.slice(2, -2);
      elements.push(
        <strong key={key} className="font-semibold text-text-primary">
          {renderInline(boldText, onCitationClick, `${key}-b`)}
        </strong>
      );
    } else if (token.startsWith('[') && token.includes('](')) {
      const linkMatch = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(token);
      if (linkMatch) {
        elements.push(
          <a
            key={key}
            href={linkMatch[2]}
            target="_blank"
            rel="noreferrer"
            className="text-brass underline decoration-dotted hover:text-brass/80"
          >
            {linkMatch[1]}
          </a>
        );
      } else {
        elements.push(token);
      }
    } else if (token.startsWith('[') && token.endsWith(']')) {
      const inside = token.slice(1, -1).trim();
      const parts = inside
        .split(',')
        .map((s) => parseInt(s.trim(), 10))
        .filter((n) => !isNaN(n));
      if (parts.length > 0) {
        elements.push(
          <span key={key} className="inline-flex items-center gap-1 mx-0.5 align-baseline">
            {parts.map((num, i) => (
              <button
                key={`${key}-${num}-${i}`}
                type="button"
                onClick={() => onCitationClick?.(num)}
                className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-mono font-medium bg-brass/20 text-brass hover:bg-brass/30 cursor-pointer"
                aria-label={`Citation ${num}`}
              >
                [{num}]
              </button>
            ))}
          </span>
        );
      } else {
        elements.push(token);
      }
    } else {
      elements.push(token);
    }

    lastIndex = tokenRegex.lastIndex;
  }

  if (lastIndex < text.length) {
    elements.push(text.slice(lastIndex));
  }

  return elements;
}

/**
 * Clean markdown renderer for research reports.
 * Renders headings, lists, bold text, inline code, code blocks, and
 * transforms citation references ([1], [2], [1, 2]) into interactive buttons.
 */
export function ResearchReportMarkdown({
  markdown,
  onCitationClick,
  className = '',
}: ResearchReportMarkdownProps) {
  if (!markdown) return null;

  const lines = markdown.split(/\r?\n/);
  const blocks: ReactNode[] = [];

  let i = 0;
  let blockIndex = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Code block
    if (line.trim().startsWith('```')) {
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith('```')) {
        codeLines.push(lines[i]);
        i++;
      }
      if (i < lines.length) i++; // skip closing ```
      blocks.push(
        <pre
          key={`code-${blockIndex++}`}
          className="my-2 overflow-x-auto rounded-lg border border-border-subtle bg-black/40 p-3 font-mono text-xs text-text-secondary"
        >
          <code>{codeLines.join('\n')}</code>
        </pre>
      );
      continue;
    }

    // Headings
    const headingMatch = line.match(/^(#{1,6})\s+(.*)$/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const content = headingMatch[2];
      const inline = renderInline(content, onCitationClick, `h-${blockIndex}`);
      if (level === 1) {
        blocks.push(
          <h1
            key={`h-${blockIndex++}`}
            className="mt-4 mb-2 font-display text-lg font-semibold text-text-primary"
          >
            {inline}
          </h1>
        );
      } else if (level === 2) {
        blocks.push(
          <h2
            key={`h-${blockIndex++}`}
            className="mt-3 mb-1.5 font-display text-base font-semibold text-text-primary"
          >
            {inline}
          </h2>
        );
      } else if (level === 3) {
        blocks.push(
          <h3
            key={`h-${blockIndex++}`}
            className="mt-2.5 mb-1 font-display text-sm font-medium text-text-primary"
          >
            {inline}
          </h3>
        );
      } else {
        blocks.push(
          <h4
            key={`h-${blockIndex++}`}
            className="mt-2 mb-1 font-display text-xs font-medium text-text-primary"
          >
            {inline}
          </h4>
        );
      }
      i++;
      continue;
    }

    // Unordered lists
    if (/^\s*[-*]\s+/.test(line)) {
      const listItems: ReactNode[] = [];
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) {
        const itemContent = lines[i].replace(/^\s*[-*]\s+/, '');
        listItems.push(
          <li key={`li-${blockIndex}-${listItems.length}`} className="leading-relaxed">
            {renderInline(itemContent, onCitationClick, `li-${blockIndex}-${listItems.length}`)}
          </li>
        );
        i++;
      }
      blocks.push(
        <ul
          key={`ul-${blockIndex++}`}
          className="my-2 list-disc list-inside space-y-1 text-sm text-text-secondary"
        >
          {listItems}
        </ul>
      );
      continue;
    }

    // Ordered lists
    if (/^\s*\d+\.\s+/.test(line)) {
      const listItems: ReactNode[] = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
        const itemContent = lines[i].replace(/^\s*\d+\.\s+/, '');
        listItems.push(
          <li key={`oli-${blockIndex}-${listItems.length}`} className="leading-relaxed">
            {renderInline(itemContent, onCitationClick, `oli-${blockIndex}-${listItems.length}`)}
          </li>
        );
        i++;
      }
      blocks.push(
        <ol
          key={`ol-${blockIndex++}`}
          className="my-2 list-decimal list-inside space-y-1 text-sm text-text-secondary"
        >
          {listItems}
        </ol>
      );
      continue;
    }

    // Empty lines
    if (!line.trim()) {
      i++;
      continue;
    }

    // Paragraphs
    const pLines: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() &&
      !lines[i].trim().startsWith('```') &&
      !/^(#{1,6})\s+/.test(lines[i]) &&
      !/^\s*[-*]\s+/.test(lines[i]) &&
      !/^\s*\d+\.\s+/.test(lines[i])
    ) {
      pLines.push(lines[i]);
      i++;
    }

    blocks.push(
      <p key={`p-${blockIndex++}`} className="my-2 text-sm leading-relaxed text-text-secondary">
        {renderInline(pLines.join(' '), onCitationClick, `p-${blockIndex}`)}
      </p>
    );
  }

  return <div className={`research-report-markdown ${className}`}>{blocks}</div>;
}
