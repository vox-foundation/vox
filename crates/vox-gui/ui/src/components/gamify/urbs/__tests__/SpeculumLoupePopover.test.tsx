import React from 'react';
import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { SpeculumLoupePopover } from '../SpeculumLoupePopover';
import type { FileDiffAndDiagnosticsDto } from '../../../../transport/t3Wave3Rpc';

describe('SpeculumLoupePopover (Phase 271 — URBS-LOUPE-01..04)', () => {
  const sampleDto: FileDiffAndDiagnosticsDto = {
    file_path: 'crates/core-compiler/src/parser.rs',
    added_lines: 18,
    deleted_lines: 4,
    unified_diff: [
      '--- a/crates/core-compiler/src/parser.rs',
      '+++ b/crates/core-compiler/src/parser.rs',
      '@@ -40,4 +40,6 @@',
      '-    parse_atom(tokens)',
      '+    parse_pratt_expr(tokens, 0)',
      '+    // verify operator precedence',
    ].join('\n'),
    diagnostics: [
      {
        file: 'crates/core-compiler/src/parser.rs',
        line: 42,
        column: 12,
        severity: 'error',
        code: 'E0308',
        message: 'mismatched types: expected Expr, found Result<Expr, ParseError>',
      },
    ],
  };

  it('renders unified diff excerpt, exact file:line:col diagnostic, and dispatches stage/revert/fix actions', () => {
    const onStage = vi.fn();
    const onRevert = vi.fn();
    const onFix = vi.fn();
    const onClose = vi.fn();

    render(
      <SpeculumLoupePopover
        diffData={sampleDto}
        districtLabel="core-compiler"
        onClose={onClose}
        onStageFile={onStage}
        onRevertFile={onRevert}
        onDispatchFix={onFix}
      />,
    );

    expect(screen.getByTestId('urbs-speculum-loupe-popover')).toBeTruthy();
    expect(screen.getByTestId('urbs-loupe-diagnostics-box').textContent).toContain(
      '[E0308 42:12]',
    );
    expect(screen.getByTestId('urbs-loupe-diff-hunk').textContent).toContain(
      '+    parse_pratt_expr(tokens, 0)',
    );

    fireEvent.click(screen.getByTestId('urbs-loupe-stage-btn'));
    expect(onStage).toHaveBeenCalledWith('crates/core-compiler/src/parser.rs');

    fireEvent.click(screen.getByTestId('urbs-loupe-revert-btn'));
    expect(onRevert).toHaveBeenCalledWith('crates/core-compiler/src/parser.rs');

    fireEvent.click(screen.getByTestId('urbs-loupe-fix-btn'));
    expect(onFix).toHaveBeenCalledWith(
      'crates/core-compiler/src/parser.rs',
      expect.objectContaining({ code: 'E0308', line: 42 }),
      sampleDto.unified_diff,
    );
  });
});
