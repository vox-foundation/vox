import React, { type ReactNode } from 'react';

export type DocFormat =
  | 'auto'
  | 'markdown'
  | 'mdx'
  | 'asciidoc'
  | 'json'
  | 'yaml'
  | 'toml'
  | 'csv'
  | 'code'
  | 'text';

export interface DocHeadingItem {
  id: string;
  level: number;
  text: string;
}

export interface ResearchReportMarkdownProps {
  markdown: string;
  onCitationClick?: (citationNum: number) => void;
  className?: string;
  path?: string;
  format?: DocFormat;
  showFrontmatterCard?: boolean;
}

const KNOWN_FORMATS: ReadonlyArray<Exclude<DocFormat, 'auto'>> = [
  'markdown',
  'mdx',
  'asciidoc',
  'json',
  'yaml',
  'toml',
  'csv',
  'code',
  'text',
];

export function detectDocFormat(
  path?: string,
  content?: string,
): Exclude<DocFormat, 'auto'> {
  const lower = (path ?? '').trim().toLowerCase();
  const ext = lower.includes('.') ? lower.split('.').pop() ?? '' : '';

  if (ext === 'mdx') return 'mdx';
  if (ext === 'md' || ext === 'markdown') return 'markdown';
  if (ext === 'adoc' || ext === 'asciidoc') return 'asciidoc';
  if (ext === 'json' || ext === 'jsonl') return 'json';
  if (ext === 'yaml' || ext === 'yml') return 'yaml';
  if (ext === 'toml') return 'toml';
  if (ext === 'csv' || ext === 'tsv') return 'csv';
  if (
    ext === 'vox' ||
    ext === 'rs' ||
    ext === 'ts' ||
    ext === 'tsx' ||
    ext === 'js' ||
    ext === 'jsx' ||
    ext === 'py' ||
    ext === 'sh' ||
    ext === 'sql'
  ) {
    return 'code';
  }
  if (ext === 'txt' || ext === 'log') return 'text';

  const trimmed = (content ?? '').trim();
  if (
    (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
    (trimmed.startsWith('[') && trimmed.endsWith(']'))
  ) {
    return 'json';
  }
  if (/^=\s+\S+/m.test(trimmed) && /\n==\s+\S+/m.test(trimmed)) {
    return 'asciidoc';
  }
  return 'markdown';
}

function slugifyHeading(raw: string): string {
  return (
    raw
      .toLowerCase()
      .replace(/[^a-z0-9\s-]/g, '')
      .trim()
      .replace(/\s+/g, '-') || 'section'
  );
}

interface ParsedFrontmatter {
  entries: Array<{ key: string; value: string }>;
  body: string;
}

function extractFrontmatter(raw: string): ParsedFrontmatter {
  const normalized = raw.replace(/\r\n/g, '\n');
  let delimiter: '---' | '+++' | null = null;
  if (normalized.startsWith('---\n')) delimiter = '---';
  else if (normalized.startsWith('+++\n')) delimiter = '+++';

  if (!delimiter) {
    return { entries: [], body: raw };
  }

  const closeIdx = normalized.indexOf(`\n${delimiter}`, delimiter.length);
  if (closeIdx === -1) {
    return { entries: [], body: raw };
  }

  const block = normalized.slice(delimiter.length + 1, closeIdx);
  const body = normalized.slice(closeIdx + delimiter.length + 1).replace(/^\n+/, '');
  const entries: Array<{ key: string; value: string }> = [];

  for (const line of block.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const sep = delimiter === '+++' ? '=' : ':';
    const idx = trimmed.indexOf(sep);
    if (idx <= 0) continue;
    const key = trimmed.slice(0, idx).trim();
    const val = trimmed
      .slice(idx + 1)
      .trim()
      .replace(/^["']|["']$/g, '');
    if (key) entries.push({ key, value: val });
  }

  return { entries, body };
}

export function extractDocHeadings(
  content: string,
  format: DocFormat = 'auto',
  path?: string,
): DocHeadingItem[] {
  if (!content.trim()) return [];
  const effectiveFormat =
    format === 'auto' ? detectDocFormat(path, content) : format;
  if (
    effectiveFormat !== 'markdown' &&
    effectiveFormat !== 'mdx' &&
    effectiveFormat !== 'asciidoc'
  ) {
    return [];
  }

  const { body } = extractFrontmatter(content);
  const lines = body.split(/\r?\n/);
  const headings: DocHeadingItem[] = [];
  let inCodeFence = false;
  const seen = new Map<string, number>();

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith('```') || trimmed === '----') {
      inCodeFence = !inCodeFence;
      continue;
    }
    if (inCodeFence) continue;

    let match: RegExpMatchArray | null = null;
    if (effectiveFormat === 'asciidoc') {
      match = line.match(/^(={1,6})\s+(.*)$/);
    }
    if (!match) {
      match = line.match(/^(#{1,6})\s+(.*)$/);
    }
    if (match) {
      const level = match[1]!.length;
      const text = match[2]!.trim();
      const baseSlug = slugifyHeading(text);
      const count = seen.get(baseSlug) ?? 0;
      seen.set(baseSlug, count + 1);
      const id = count === 0 ? `doc-sec-${baseSlug}` : `doc-sec-${baseSlug}-${count}`;
      headings.push({ id, level, text });
    }
  }

  return headings;
}

export function listDocFormatCycle(): ReadonlyArray<DocFormat> {
  return ['auto', ...KNOWN_FORMATS];
}

/**
 * Parses inline markdown:
 * - Code: `code`
 * - Bold: **bold**
 * - Strikethrough: ~~strike~~
 * - Italic: *italic*
 * - Citations: [1], [2], [1, 2]
 * - Links: [label](url)
 */
function renderInline(
  text: string,
  onCitationClick?: (num: number) => void,
  keyPrefix = 'inline',
): ReactNode[] {
  const tokenRegex =
    /(`[^`]+`|\*\*[^*]+\*\*|~~[^~]+~~|\*[^*\n]+\*|\[\d{1,3}(?:\s*,\s*\d{1,3})*\](?!\()|\[[^\]]+\]\([^)\s]+\))/g;

  const elements: ReactNode[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let count = 0;

  while ((match = tokenRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      elements.push(text.slice(lastIndex, match.index));
    }
    const token = match[0]!;
    const key = `${keyPrefix}-${count++}`;

    if (token.startsWith('`') && token.endsWith('`')) {
      const code = token.slice(1, -1);
      elements.push(
        <code
          key={key}
          className="rounded bg-overlay-subtle px-1 py-0.5 font-mono text-xs text-brass"
        >
          {code}
        </code>,
      );
    } else if (token.startsWith('**') && token.endsWith('**')) {
      const boldText = token.slice(2, -2);
      elements.push(
        <strong key={key} className="font-semibold text-text-primary">
          {renderInline(boldText, onCitationClick, `${key}-b`)}
        </strong>,
      );
    } else if (token.startsWith('~~') && token.endsWith('~~')) {
      const struck = token.slice(2, -2);
      elements.push(
        <del key={key} className="text-text-muted">
          {renderInline(struck, onCitationClick, `${key}-del`)}
        </del>,
      );
    } else if (token.startsWith('*') && token.endsWith('*') && token.length > 2) {
      const emText = token.slice(1, -1);
      elements.push(
        <em key={key} className="italic text-text-primary">
          {renderInline(emText, onCitationClick, `${key}-em`)}
        </em>,
      );
    } else if (token.startsWith('[') && token.includes('](')) {
      const linkMatch = /^\[([^\]]+)\]\((.*)\)$/.exec(token);
      if (linkMatch) {
        const rawUrl = linkMatch[2]!.trim();
        const isSafeProtocol = /^(https?:\/\/|mailto:)/i.test(rawUrl);
        if (isSafeProtocol) {
          elements.push(
            <a
              key={key}
              href={rawUrl}
              target="_blank"
              rel="noreferrer noopener"
              className="text-brass underline decoration-dotted hover:text-brass/80"
            >
              {linkMatch[1]}
            </a>,
          );
        } else {
          elements.push(linkMatch[1]);
        }
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
            {parts.map((num, idx) => (
              <button
                key={`${key}-${num}-${idx}`}
                type="button"
                onClick={() => onCitationClick?.(num)}
                className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-mono font-medium bg-brass/20 text-brass hover:bg-brass/30 cursor-pointer"
                aria-label={`Citation ${num}`}
              >
                [{num}]
              </button>
            ))}
          </span>,
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

function splitPipeRow(line: string): string[] {
  const trimmed = line.trim().replace(/^\|/, '').replace(/\|$/, '');
  return trimmed.split('|').map((cell) => cell.trim());
}

function isTableDelimiterLine(line: string): boolean {
  const cells = splitPipeRow(line);
  return cells.length > 0 && cells.every((c) => /^:?-{2,}:?$/.test(c));
}

function renderStructuredJson(content: string): ReactNode {
  const trimmed = content.trim();
  if (!trimmed) return null;

  try {
    const parsed: unknown = JSON.parse(trimmed);
    const pretty = JSON.stringify(parsed, null, 2);
    const entries =
      parsed && typeof parsed === 'object' && !Array.isArray(parsed)
        ? Object.entries(parsed as Record<string, unknown>).slice(0, 24)
        : null;

    return (
      <div data-testid="doc-json-view" className="space-y-3">
        {entries && entries.length > 0 && (
          <div className="overflow-x-auto rounded-lg border border-border-subtle">
            <table className="w-full border-collapse text-left text-xs">
              <thead>
                <tr className="border-b border-border-strong bg-bg-elevated font-display uppercase tracking-wider text-[11px] text-text-secondary">
                  <th className="px-3 py-1.5">Key</th>
                  <th className="px-3 py-1.5">Value</th>
                </tr>
              </thead>
              <tbody>
                {entries.map(([k, v]) => (
                  <tr key={k} className="border-b border-border-subtle/60 last:border-b-0">
                    <td className="px-3 py-1.5 font-mono text-brass">{k}</td>
                    <td className="px-3 py-1.5 font-mono text-text-primary break-all">
                      {typeof v === 'object' ? JSON.stringify(v) : String(v)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <pre className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-elevated p-3 font-mono text-xs text-text-primary">
          <code>{pretty}</code>
        </pre>
      </div>
    );
  } catch {
    // Handle JSONL (newline-delimited JSON records) cleanly
    const jsonlRows = trimmed
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean);
    const parsedRows = jsonlRows
      .slice(0, 50)
      .map((line) => {
        try {
          return JSON.parse(line) as unknown;
        } catch {
          return null;
        }
      })
      .filter((r) => r !== null);

    if (parsedRows.length > 0) {
      return (
        <div data-testid="doc-json-view" className="space-y-2">
          <div className="font-mono text-[11px] text-text-secondary">
            JSONL Stream ({jsonlRows.length} records)
          </div>
          <pre className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-elevated p-3 font-mono text-xs text-text-primary">
            <code>{parsedRows.map((r) => JSON.stringify(r)).join('\n')}</code>
          </pre>
        </div>
      );
    }

    return (
      <pre data-testid="doc-json-view" className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-elevated p-3 font-mono text-xs text-text-primary">
        <code>{content}</code>
      </pre>
    );
  }
}

function renderCsvGrid(content: string, isTsv: boolean): ReactNode {
  const delimiter = isTsv ? '\t' : ',';
  const rows = content
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean)
    .map((line) => line.split(delimiter).map((c) => c.trim().replace(/^"|"$/g, '')));

  if (rows.length === 0) return null;
  const header = rows[0]!;
  const body = rows.slice(1);

  return (
    <div data-testid="doc-csv-table" className="overflow-x-auto rounded-lg border border-border-subtle">
      <table className="w-full border-collapse text-left text-xs">
        <thead>
          <tr className="border-b border-border-strong bg-bg-elevated font-display uppercase tracking-wider text-[11px] text-text-secondary">
            {header.map((cell, idx) => (
              <th key={idx} className="px-3 py-1.5">
                {cell}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {body.map((row, rIdx) => (
            <tr key={rIdx} className="border-b border-border-subtle/60 last:border-b-0">
              {header.map((_, cIdx) => (
                <td key={cIdx} className="px-3 py-1.5 font-mono text-text-primary">
                  {row[cIdx] ?? ''}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function renderKeyValueConfig(content: string, mode: 'yaml' | 'toml'): ReactNode {
  const sep = mode === 'toml' ? '=' : ':';
  const pairs: Array<{ key: string; value: string }> = [];
  for (const rawLine of content.split(/\r?\n/)) {
    const trimmed = rawLine.trim();
    if (!trimmed || trimmed.startsWith('#') || trimmed.startsWith('[')) continue;
    const idx = trimmed.indexOf(sep);
    if (idx > 0) {
      pairs.push({
        key: trimmed.slice(0, idx).trim(),
        value: trimmed.slice(idx + 1).trim(),
      });
    }
  }

  return (
    <div data-testid="doc-config-view" className="space-y-2.5">
      {pairs.length > 0 && (
        <div className="overflow-x-auto rounded-lg border border-border-subtle">
          <table className="w-full border-collapse text-left text-xs">
            <thead>
              <tr className="border-b border-border-strong bg-bg-elevated font-display uppercase tracking-wider text-[11px] text-text-secondary">
                <th className="px-3 py-1.5">{mode.toUpperCase()} Key</th>
                <th className="px-3 py-1.5">Value</th>
              </tr>
            </thead>
            <tbody>
              {pairs.map((p, idx) => (
                <tr key={`${p.key}-${idx}`} className="border-b border-border-subtle/60 last:border-b-0">
                  <td className="px-3 py-1.5 font-mono text-brass">{p.key}</td>
                  <td className="px-3 py-1.5 font-mono text-text-primary break-all">{p.value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <pre className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-elevated p-3 font-mono text-xs text-text-secondary">
        <code>{content}</code>
      </pre>
    </div>
  );
}

function renderLineNumberedCode(content: string): ReactNode {
  const lines = content.split(/\r?\n/);
  return (
    <div
      data-testid="doc-code-view"
      className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-elevated p-3 font-mono text-xs text-text-primary"
    >
      <table className="w-full border-collapse">
        <tbody>
          {lines.map((line, idx) => (
            <tr key={idx}>
              <td className="select-none pr-3 text-right text-[11px] text-text-muted align-top w-8">
                {idx + 1}
              </td>
              <td className="whitespace-pre text-text-primary">{line || ' '}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function calloutStyleForKind(kind: string): { label: string; borderClass: string } {
  const upper = kind.toUpperCase();
  if (upper === 'WARNING' || upper === 'CAUTION') {
    return {
      label: upper,
      borderClass: 'border-(--color-status-warn)/60 bg-(--color-status-warn)/10',
    };
  }
  if (upper === 'IMPORTANT' || upper === 'TIP') {
    return {
      label: upper,
      borderClass: 'border-(--color-status-pass)/50 bg-bg-elevated',
    };
  }
  return {
    label: upper || 'NOTE',
    borderClass: 'border-brass/55 bg-bg-elevated',
  };
}

/**
 * Full documentation and research parser supporting GFM (headings, tables, task checklists,
 * callout alerts, fenced code, inline styles, safe links, [N] citation buttons), YAML/TOML
 * frontmatter cards, `<!-- kb:summary -->`, MDX `<Aside>`, AsciiDoc, JSON/JSONL, YAML, TOML,
 * CSV/TSV, and line-numbered source files.
 */
export function ResearchReportMarkdown({
  markdown,
  onCitationClick,
  className = '',
  path,
  format = 'auto',
  showFrontmatterCard = false,
}: ResearchReportMarkdownProps) {
  if (!markdown) return null;

  const effectiveFormat =
    format === 'auto' ? detectDocFormat(path, markdown) : format;

  if (effectiveFormat === 'json') {
    return <div className={`research-report-markdown ${className}`}>{renderStructuredJson(markdown)}</div>;
  }
  if (effectiveFormat === 'csv') {
    const isTsv = (path ?? '').toLowerCase().endsWith('.tsv');
    return <div className={`research-report-markdown ${className}`}>{renderCsvGrid(markdown, isTsv)}</div>;
  }
  if (effectiveFormat === 'yaml' || effectiveFormat === 'toml') {
    return (
      <div className={`research-report-markdown ${className}`}>
        {renderKeyValueConfig(markdown, effectiveFormat)}
      </div>
    );
  }
  if (effectiveFormat === 'code') {
    return <div className={`research-report-markdown ${className}`}>{renderLineNumberedCode(markdown)}</div>;
  }
  if (effectiveFormat === 'text') {
    return (
      <pre className={`research-report-markdown whitespace-pre-wrap font-mono text-xs leading-relaxed text-text-primary ${className}`}>
        {markdown}
      </pre>
    );
  }

  const { entries: frontmatterEntries, body: withoutFrontmatter } = extractFrontmatter(markdown);

  let kbSummaryText: string | null = null;
  const cleanedBody = withoutFrontmatter.replace(
    /<!--\s*kb:summary\s*-->([\s\S]*?)<!--\s*\/kb:summary\s*-->/i,
    (_m, inner: string) => {
      kbSummaryText = inner.trim();
      return '';
    },
  );

  const lines = cleanedBody.split(/\r?\n/);
  const blocks: ReactNode[] = [];
  const headingSlugCounts = new Map<string, number>();

  let i = 0;
  let blockIndex = 0;

  while (i < lines.length) {
    const line = lines[i]!;
    const trimmed = line.trim();

    if (!trimmed) {
      i++;
      continue;
    }

    // MDX export/import lines
    if (effectiveFormat === 'mdx' && /^(import|export)\s+/.test(trimmed)) {
      i++;
      continue;
    }

    // MDX <Aside type="..."> ... </Aside>
    const asideOpen = trimmed.match(/^<Aside(?:\s+[^>]*type=["']([^"']+)["'])?[^>]*>$/i);
    if (asideOpen) {
      const kind = asideOpen[1] ?? 'NOTE';
      const innerLines: string[] = [];
      i++;
      while (i < lines.length && !/^<\/Aside>/i.test(lines[i]!.trim())) {
        innerLines.push(lines[i]!);
        i++;
      }
      if (i < lines.length) i++;
      const style = calloutStyleForKind(kind);
      blocks.push(
        <aside
          key={`callout-${blockIndex++}`}
          data-testid="doc-callout-block"
          className={`my-2.5 rounded-lg border px-3.5 py-2.5 text-xs ${style.borderClass}`}
        >
          <div className="mb-1 font-display text-[11px] font-bold uppercase tracking-wider text-brass">
            {style.label}
          </div>
          <p className="font-serif text-sm leading-relaxed text-text-primary">
            {renderInline(innerLines.join(' ').trim(), onCitationClick, `aside-${blockIndex}`)}
          </p>
        </aside>,
      );
      continue;
    }

    // Horizontal rule
    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      blocks.push(<hr key={`hr-${blockIndex++}`} className="my-3 border-border-subtle" />);
      i++;
      continue;
    }

    // Code block (Markdown ```lang or AsciiDoc ----)
    if (trimmed.startsWith('```') || (effectiveFormat === 'asciidoc' && trimmed === '----')) {
      const isAsciiFence = trimmed === '----';
      const lang = isAsciiFence ? '' : trimmed.slice(3).trim();
      const codeLines: string[] = [];
      i++;
      while (
        i < lines.length &&
        (isAsciiFence ? lines[i]!.trim() !== '----' : !lines[i]!.trim().startsWith('```'))
      ) {
        codeLines.push(lines[i]!);
        i++;
      }
      if (i < lines.length) i++;
      blocks.push(
        <div
          key={`code-${blockIndex++}`}
          className="my-2 overflow-hidden rounded-lg border border-border-subtle bg-bg-elevated"
        >
          {lang && (
            <div className="border-b border-border-subtle px-3 py-1 font-mono text-[11px] uppercase tracking-wider text-text-muted">
              {lang}
            </div>
          )}
          <pre className="overflow-x-auto p-3 font-mono text-xs text-text-secondary">
            <code>{codeLines.join('\n')}</code>
          </pre>
        </div>,
      );
      continue;
    }

    // GFM Pipe Table
    if (
      line.includes('|') &&
      i + 1 < lines.length &&
      lines[i + 1]!.includes('|') &&
      isTableDelimiterLine(lines[i + 1]!)
    ) {
      const headers = splitPipeRow(line);
      i += 2;
      const bodyRows: string[][] = [];
      while (i < lines.length && lines[i]!.includes('|') && lines[i]!.trim().length > 0) {
        bodyRows.push(splitPipeRow(lines[i]!));
        i++;
      }
      const tblKey = `tbl-${blockIndex++}`;
      blocks.push(
        <div
          key={tblKey}
          className="my-2.5 overflow-x-auto rounded-lg border border-border-subtle"
        >
          <table
            data-testid="doc-markdown-table"
            className="w-full border-collapse text-left text-xs"
          >
            <thead>
              <tr className="border-b border-border-strong bg-bg-elevated font-display uppercase tracking-wider text-[11px] text-text-secondary">
                {headers.map((hCell, cIdx) => (
                  <th key={`${tblKey}-h-${cIdx}`} className="px-3 py-1.5 font-semibold">
                    {renderInline(hCell, onCitationClick, `${tblKey}-hc-${cIdx}`)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {bodyRows.map((row, rIdx) => (
                <tr
                  key={`${tblKey}-r-${rIdx}`}
                  className="border-b border-border-subtle/60 last:border-b-0"
                >
                  {headers.map((_, cIdx) => (
                    <td key={`${tblKey}-c-${rIdx}-${cIdx}`} className="px-3 py-1.5 text-text-primary">
                      {renderInline(row[cIdx] ?? '', onCitationClick, `${tblKey}-td-${rIdx}-${cIdx}`)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>,
      );
      continue;
    }

    // Blockquotes & GitHub/Obsidian callouts (> [!NOTE] ...)
    if (trimmed.startsWith('>')) {
      const quoteLines: string[] = [];
      while (i < lines.length && lines[i]!.trim().startsWith('>')) {
        quoteLines.push(lines[i]!.trim().replace(/^>\s?/, ''));
        i++;
      }
      const alertHeader = quoteLines[0]?.match(/^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*(.*)$/i);
      if (alertHeader) {
        const style = calloutStyleForKind(alertHeader[1]!);
        const bodyParts = [alertHeader[2] ?? '', ...quoteLines.slice(1)]
          .map((s) => s.trim())
          .filter(Boolean);
        blocks.push(
          <aside
            key={`callout-${blockIndex++}`}
            data-testid="doc-callout-block"
            className={`my-2.5 rounded-lg border px-3.5 py-2.5 text-xs ${style.borderClass}`}
          >
            <div className="mb-1 font-display text-[11px] font-bold uppercase tracking-wider text-brass">
              {style.label}
            </div>
            <p className="font-serif text-sm leading-relaxed text-text-primary">
              {renderInline(bodyParts.join(' '), onCitationClick, `callout-${blockIndex}`)}
            </p>
          </aside>,
        );
      } else {
        blocks.push(
          <blockquote
            key={`bq-${blockIndex++}`}
            className="my-2 border-l-2 border-brass/70 pl-3 font-serif text-sm italic text-text-secondary"
          >
            {renderInline(quoteLines.join(' '), onCitationClick, `bq-${blockIndex}`)}
          </blockquote>,
        );
      }
      continue;
    }

    // AsciiDoc single-line admonition (NOTE: ...)
    const adocNoteMatch =
      effectiveFormat === 'asciidoc'
        ? trimmed.match(/^(NOTE|TIP|IMPORTANT|WARNING|CAUTION):\s+(.*)$/)
        : null;
    if (adocNoteMatch) {
      const style = calloutStyleForKind(adocNoteMatch[1]!);
      blocks.push(
        <aside
          key={`callout-${blockIndex++}`}
          data-testid="doc-callout-block"
          className={`my-2.5 rounded-lg border px-3.5 py-2.5 text-xs ${style.borderClass}`}
        >
          <div className="mb-1 font-display text-[11px] font-bold uppercase tracking-wider text-brass">
            {style.label}
          </div>
          <p className="font-serif text-sm leading-relaxed text-text-primary">
            {renderInline(adocNoteMatch[2]!, onCitationClick, `adoc-${blockIndex}`)}
          </p>
        </aside>,
      );
      i++;
      continue;
    }

    // Headings (# .. ###### or AsciiDoc = .. ======)
    const headingMatch =
      (effectiveFormat === 'asciidoc' ? line.match(/^(={1,6})\s+(.*)$/) : null) ??
      line.match(/^(#{1,6})\s+(.*)$/);
    if (headingMatch) {
      const level = headingMatch[1]!.length;
      const content = headingMatch[2]!.trim();
      const baseSlug = slugifyHeading(content);
      const count = headingSlugCounts.get(baseSlug) ?? 0;
      headingSlugCounts.set(baseSlug, count + 1);
      const headingId = count === 0 ? `doc-sec-${baseSlug}` : `doc-sec-${baseSlug}-${count}`;
      const inline = renderInline(content, onCitationClick, `h-${blockIndex}`);
      if (level === 1) {
        blocks.push(
          <h1
            id={headingId}
            key={`h-${blockIndex++}`}
            className="mt-4 mb-2 font-display text-lg font-semibold text-text-primary"
          >
            {inline}
          </h1>,
        );
      } else if (level === 2) {
        blocks.push(
          <h2
            id={headingId}
            key={`h-${blockIndex++}`}
            className="mt-3 mb-1.5 font-display text-base font-semibold text-text-primary"
          >
            {inline}
          </h2>,
        );
      } else if (level === 3) {
        blocks.push(
          <h3
            id={headingId}
            key={`h-${blockIndex++}`}
            className="mt-2.5 mb-1 font-display text-sm font-medium text-text-primary"
          >
            {inline}
          </h3>,
        );
      } else {
        blocks.push(
          <h4
            id={headingId}
            key={`h-${blockIndex++}`}
            className="mt-2 mb-1 font-display text-xs font-medium text-text-primary"
          >
            {inline}
          </h4>,
        );
      }
      i++;
      continue;
    }

    // Task checklists (- [x] / - [ ])
    if (/^\s*[-*]\s+\[(x|X| )\]\s+/.test(line)) {
      const items: Array<{ done: boolean; text: string }> = [];
      while (i < lines.length && /^\s*[-*]\s+\[(x|X| )\]\s+/.test(lines[i]!)) {
        const m = lines[i]!.match(/^\s*[-*]\s+\[(x|X| )\]\s+(.*)$/)!;
        items.push({ done: m[1]!.toLowerCase() === 'x', text: m[2]! });
        i++;
      }
      blocks.push(
        <ul
          key={`tasks-${blockIndex++}`}
          data-testid="doc-task-list"
          className="my-2 space-y-1 text-sm text-text-secondary"
        >
          {items.map((item, idx) => (
            <li key={idx} className="flex items-start gap-2 leading-relaxed">
              <span
                aria-hidden="true"
                className={`mt-1 inline-flex size-4 shrink-0 items-center justify-center rounded-xs border text-xs ${
                  item.done
                    ? 'border-(--color-status-pass) bg-(--color-status-pass)/20 text-(--color-status-pass)'
                    : 'border-border-strong bg-bg-surface text-text-muted'
                }`}
              >
                {item.done ? (
                  <svg viewBox="0 0 12 12" className="size-3 stroke-current fill-none" strokeWidth={2}>
                    <path d="M2.5 6.2 5 8.7 9.5 3.5" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                ) : null}
              </span>
              <span className={item.done ? 'text-text-primary' : 'text-text-secondary'}>
                {renderInline(item.text, onCitationClick, `task-${blockIndex}-${idx}`)}
              </span>
            </li>
          ))}
        </ul>,
      );
      continue;
    }

    // Unordered lists
    if (/^\s*[-*]\s+/.test(line)) {
      const listItems: ReactNode[] = [];
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i]!)) {
        const itemContent = lines[i]!.replace(/^\s*[-*]\s+/, '');
        listItems.push(
          <li key={`li-${blockIndex}-${listItems.length}`} className="leading-relaxed">
            {renderInline(itemContent, onCitationClick, `li-${blockIndex}-${listItems.length}`)}
          </li>,
        );
        i++;
      }
      blocks.push(
        <ul
          key={`ul-${blockIndex++}`}
          className="my-2 list-disc list-inside space-y-1 text-sm text-text-secondary"
        >
          {listItems}
        </ul>,
      );
      continue;
    }

    // Ordered lists
    if (/^\s*\d+\.\s+/.test(line)) {
      const listItems: ReactNode[] = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i]!)) {
        const itemContent = lines[i]!.replace(/^\s*\d+\.\s+/, '');
        listItems.push(
          <li key={`oli-${blockIndex}-${listItems.length}`} className="leading-relaxed">
            {renderInline(itemContent, onCitationClick, `oli-${blockIndex}-${listItems.length}`)}
          </li>,
        );
        i++;
      }
      blocks.push(
        <ol
          key={`ol-${blockIndex++}`}
          className="my-2 list-decimal list-inside space-y-1 text-sm text-text-secondary"
        >
          {listItems}
        </ol>,
      );
      continue;
    }

    // Paragraphs
    const pLines: string[] = [];
    while (
      i < lines.length &&
      lines[i]!.trim() &&
      !lines[i]!.trim().startsWith('```') &&
      !lines[i]!.trim().startsWith('>') &&
      !/^(-{3,}|\*{3,}|_{3,})$/.test(lines[i]!.trim()) &&
      !/^(#{1,6})\s+/.test(lines[i]!) &&
      !/^\s*[-*]\s+/.test(lines[i]!) &&
      !/^\s*\d+\.\s+/.test(lines[i]!)
    ) {
      pLines.push(lines[i]!);
      i++;
    }

    blocks.push(
      <p key={`p-${blockIndex++}`} className="my-2 font-serif text-sm leading-relaxed text-text-primary">
        {renderInline(pLines.join(' '), onCitationClick, `p-${blockIndex}`)}
      </p>,
    );
  }

  return (
    <div className={`research-report-markdown ${className}`}>
      {showFrontmatterCard && frontmatterEntries.length > 0 && (
        <div
          data-testid="doc-frontmatter-card"
          className="mb-3 flex flex-wrap items-center gap-2 rounded-lg border border-border-subtle bg-bg-elevated px-3 py-2 text-xs"
        >
          {frontmatterEntries.map((entry) => (
            <span
              key={entry.key}
              className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-surface px-2 py-0.5 font-mono text-[11px]"
            >
              <span className="text-text-muted">{entry.key}:</span>
              <span className="text-text-primary">{entry.value}</span>
            </span>
          ))}
        </div>
      )}
      {kbSummaryText && (
        <div
          data-testid="doc-kb-summary-callout"
          className="mb-3 rounded-lg border border-brass/45 bg-bg-elevated px-3.5 py-2.5 text-xs"
        >
          <div className="mb-1 font-display text-[11px] font-bold uppercase tracking-wider text-brass">
            Knowledge Base Summary
          </div>
          <p className="font-serif text-xs leading-relaxed text-text-primary">
            {renderInline(kbSummaryText, onCitationClick, 'kb-summary')}
          </p>
        </div>
      )}
      {blocks}
    </div>
  );
}
