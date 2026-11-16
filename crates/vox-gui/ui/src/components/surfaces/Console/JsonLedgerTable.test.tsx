import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { JsonLedgerTable, tryParseStructuredJson } from './JsonLedgerTable';

describe('JsonLedgerTable', () => {
  const writeTextMock = vi.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    cleanup();
    writeTextMock.mockClear();
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: writeTextMock },
      configurable: true,
    });
  });

  it('tryParseStructuredJson strips ANSI, enforces 1 MiB ceiling, and parses empty & populated arrays/objects safely (IN-01)', () => {
    expect(tryParseStructuredJson('\x1b[32m[{"name":"alpha"}]\x1b[0m')).toEqual([{ name: 'alpha' }]);
    expect(tryParseStructuredJson('{"host":"local","port":8080}')).toEqual({ host: 'local', port: 8080 });
    expect(tryParseStructuredJson('plain stdout text')).toBeNull();
    expect(tryParseStructuredJson('[]')).toEqual([]);
    expect(tryParseStructuredJson('{}')).toEqual({});
    expect(tryParseStructuredJson('[' + 'a'.repeat(1_048_600) + ']')).toBeNull();
  });

  it('renders object arrays with TanStack Table v9 sorting, searchbox filtering, nested cell expansion, and CSV/JSON copy', async () => {
    const rows = [
      { name: 'alpha', size: 120, meta: { ok: true, note: 'comma, "quoted"' } },
      { name: 'beta', size: 40, meta: { ok: false } },
    ];

    render(<JsonLedgerTable data={rows} />);

    expect(screen.getByTestId('term-ledger-table')).toBeTruthy();
    expect(screen.getByText('2 of 2 rows')).toBeTruthy();

    const filterInput = screen.getByTestId('term-ledger-filter') as HTMLInputElement;
    expect(filterInput.type).toBe('search');
    expect(screen.getByRole('searchbox', { name: /Filter table rows/i })).toBe(filterInput);

    // Column sorting (size: ascending 40 -> 120, descending 120 -> 40)
    const sizeSortBtn = screen.getByRole('button', { name: /^size/i });
    const sizeHeader = sizeSortBtn.closest('th')!;
    expect(sizeHeader.getAttribute('aria-sort')).toBe('none');

    fireEvent.click(sizeSortBtn);
    expect(sizeHeader.getAttribute('aria-sort')).toBe('ascending');
    const ascBodyRows = screen.getAllByRole('row').slice(1);
    expect(ascBodyRows[0]?.textContent).toContain('beta');
    expect(ascBodyRows[1]?.textContent).toContain('alpha');

    fireEvent.click(sizeSortBtn);
    expect(sizeHeader.getAttribute('aria-sort')).toBe('descending');
    const descBodyRows = screen.getAllByRole('row').slice(1);
    expect(descBodyRows[0]?.textContent).toContain('alpha');
    expect(descBodyRows[1]?.textContent).toContain('beta');

    // Nested object cell expansion
    const nestedToggles = screen.getAllByText('{...}');
    expect(nestedToggles.length).toBe(2);
    fireEvent.click(nestedToggles[0]!);
    expect(screen.getByText(/"ok": true/)).toBeTruthy();

    // Copy CSV (RFC-4180 quote escaping) & Copy JSON
    fireEvent.click(screen.getByRole('button', { name: 'Copy CSV' }));
    await waitFor(() => expect(writeTextMock).toHaveBeenCalledTimes(1));
    const csvPayload = String(writeTextMock.mock.calls[0]?.[0] ?? '');
    expect(csvPayload).toContain('name,size,meta');
    expect(csvPayload).toContain('alpha,120,"{""ok"":true,""note"":""comma, \\""quoted\\""""}"');

    fireEvent.click(screen.getByRole('button', { name: 'Copy JSON' }));
    await waitFor(() => expect(writeTextMock).toHaveBeenCalledTimes(2));
    expect(JSON.parse(String(writeTextMock.mock.calls[1]?.[0] ?? '[]'))).toHaveLength(2);

    // Search filtering + empty state
    fireEvent.change(filterInput, { target: { value: 'beta' } });
    expect(screen.getByText('1 of 2 rows')).toBeTruthy();
    expect(screen.queryByText('alpha')).toBeNull();

    fireEvent.change(filterInput, { target: { value: 'nonexistent-query' } });
    expect(screen.getByText('No matching rows')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: /Clear filter/i }));
    expect(screen.getByText('2 of 2 rows')).toBeTruthy();
  });

  it('normalizes key-value objects, renders Empty JSON collection for []/{}, and caps at 200 rows and 12 columns', () => {
    const { rerender } = render(<JsonLedgerTable data={[]} />);
    expect(screen.getByTestId('term-ledger-empty').textContent).toContain('Empty JSON collection');

    rerender(<JsonLedgerTable data={{ host: 'local', port: 8080 }} />);
    expect(screen.getByRole('button', { name: /^Field/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /^Value/i })).toBeTruthy();
    expect(screen.getByText('host')).toBeTruthy();
    expect(screen.getByText('local')).toBeTruthy();
    expect(screen.getByText('8080')).toBeTruthy();

    const wideRow = Object.fromEntries(Array.from({ length: 15 }, (_, i) => [`col_${i + 1}`, i]));
    const huge = Array.from({ length: 225 }, () => wideRow);
    rerender(<JsonLedgerTable data={huge} />);
    expect(screen.getByText(/Showing first 200 of 225 rows and 12 of 15 columns/i)).toBeTruthy();
  });
});
