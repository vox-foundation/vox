import React from 'react';
import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import {
  ViaTemporisScrubber,
  toRomanNumeral,
  type ViaTemporisMilestoneItem,
} from '../ViaTemporisScrubber';

describe('ViaTemporisScrubber (Phase 275 — URBS-TIME-01..04)', () => {
  const milestones: ViaTemporisMilestoneItem[] = [
    {
      turnNumber: 1,
      romanNumeral: 'I',
      refName: 'refs/vox/checkpoints/main/turn/1',
      commitOidShort: 'a1b2c3d',
      addedLines: 10,
      deletedLines: 2,
      modifiedFiles: ['src/lib.rs'],
      testShieldStatus: 'pass',
      tokenCostBu: 110,
    },
    {
      turnNumber: 2,
      romanNumeral: 'II',
      refName: 'refs/vox/checkpoints/main/turn/2',
      commitOidShort: 'd4e5f6a',
      addedLines: 42,
      deletedLines: 9,
      modifiedFiles: ['src/lib.rs', 'src/parser.rs'],
      testShieldStatus: 'pass',
      tokenCostBu: 240,
    },
  ];

  it('formats Roman numerals and dispatches milestone select & rollback callbacks', () => {
    expect(toRomanNumeral(1)).toBe('I');
    expect(toRomanNumeral(4)).toBe('IV');
    expect(toRomanNumeral(9)).toBe('IX');

    const onSelect = vi.fn();
    const onCapture = vi.fn();
    const onRollback = vi.fn();

    render(
      <ViaTemporisScrubber
        milestones={milestones}
        selectedTurn={1}
        activeHeadTurn={2}
        onSelectTurn={onSelect}
        onCaptureCheckpoint={onCapture}
        onRollbackToTurn={onRollback}
      />,
    );

    expect(screen.getByTestId('urbs-via-temporis-scrubber')).toBeTruthy();
    fireEvent.click(screen.getByTestId('urbs-milestone-stone-2'));
    expect(onSelect).toHaveBeenCalledWith(2);

    fireEvent.click(screen.getByTestId('urbs-scrubber-rollback-btn'));
    expect(onRollback).toHaveBeenCalledWith(1);
  });
});
