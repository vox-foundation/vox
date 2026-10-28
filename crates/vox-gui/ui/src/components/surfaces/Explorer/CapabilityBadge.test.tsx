// @vitest-environment jsdom
import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { CapabilityBadge, CapabilityTag, type CapabilityStatus, type CapabilityTagKind } from './CapabilityBadge';

describe('CapabilityBadge', () => {
  const statuses: { status: CapabilityStatus; label: string; glyph: string }[] = [
    { status: 'dedicated', label: 'Dedicated surface', glyph: '✓' },
    { status: 'generic_form', label: 'Generic form', glyph: '≡' },
    { status: 'cli_only', label: 'CLI-only', glyph: '>_' },
    { status: 'mcp_only', label: 'MCP-only / orphan', glyph: '◇' },
  ];

  for (const { status, label, glyph } of statuses) {
    it(`renders label and glyph for status '${status}'`, () => {
      const { container } = render(<CapabilityBadge status={status} />);
      const text = container.textContent ?? '';
      expect(text).toContain(label);
      expect(text).toContain(glyph);
      const className = container.firstElementChild?.className ?? '';
      expect(className).not.toMatch(/\b(sky|zinc|cyan|emerald)-/);
    });
  }

  const tags: { tag: CapabilityTagKind; label: string; glyph?: string }[] = [
    { tag: 'exempt', label: 'Exempt' },
    { tag: 'overdue', label: 'Review overdue', glyph: '!' },
  ];

  for (const { tag, label, glyph } of tags) {
    it(`renders label and optional glyph for tag '${tag}'`, () => {
      const { container } = render(<CapabilityTag tag={tag} />);
      const text = container.textContent ?? '';
      expect(text).toContain(label);
      if (glyph) {
        expect(text).toContain(glyph);
      }
      const className = container.firstElementChild?.className ?? '';
      expect(className).not.toMatch(/\b(sky|zinc|cyan|emerald)-/);
    });
  }
});
