// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePipelineStepper } from './usePipelineStepper';
import { RESEARCH_STAGES } from '../lib/pipeline';

describe('usePipelineStepper', () => {
  it('initial state has 8 stages with queued active or pending', () => {
    const { result } = renderHook(() => usePipelineStepper());

    expect(result.current.steps).toHaveLength(8);
    expect(result.current.steps.map(s => s.id)).toEqual([...RESEARCH_STAGES]);
    expect(result.current.currentStageIndex).toBe(0);
    expect(result.current.steps[0].id).toBe('queued');
    expect(['active', 'pending']).toContain(result.current.steps[0].status);
    expect(result.current.steps[1].status).toBe('pending');
  });

  it('stepNext() transitions current step to completed and next to active', () => {
    const { result } = renderHook(() => usePipelineStepper());

    expect(result.current.steps[0].status).toBe('active');
    expect(result.current.steps[1].status).toBe('pending');
    expect(result.current.currentStageIndex).toBe(0);

    act(() => {
      result.current.stepNext();
    });

    expect(result.current.steps[0].status).toBe('completed');
    expect(result.current.steps[1].status).toBe('active');
    expect(result.current.currentStageIndex).toBe(1);

    act(() => {
      result.current.stepNext();
    });

    expect(result.current.steps[1].status).toBe('completed');
    expect(result.current.steps[2].status).toBe('active');
    expect(result.current.currentStageIndex).toBe(2);
  });

  it('pause() halts stepping', () => {
    vi.useFakeTimers();
    try {
      const { result } = renderHook(() =>
        usePipelineStepper({ autoPlay: true, stepIntervalMs: 1000 })
      );

      expect(result.current.autoPlay).toBe(true);
      expect(result.current.isPaused).toBe(false);

      act(() => {
        vi.advanceTimersByTime(1000);
      });

      expect(result.current.currentStageIndex).toBe(1);

      // Calling pause halts stepping
      act(() => {
        result.current.pause();
      });

      expect(result.current.isPaused).toBe(true);
      expect(result.current.autoPlay).toBe(false);

      // Advancing timer further should not advance step
      act(() => {
        vi.advanceTimersByTime(3000);
      });

      expect(result.current.currentStageIndex).toBe(1);

      // Resume continues
      act(() => {
        result.current.resume();
      });
      expect(result.current.isPaused).toBe(false);
      expect(result.current.autoPlay).toBe(true);

      act(() => {
        vi.advanceTimersByTime(1000);
      });
      expect(result.current.currentStageIndex).toBe(2);
    } finally {
      vi.useRealTimers();
    }
  });

  it('overridePayload() updates payload for target stage', () => {
    const { result } = renderHook(() => usePipelineStepper());

    act(() => {
      result.current.overridePayload('retrieving', { sources: ['https://example.com/doc1'] });
    });

    const retrievingStep = result.current.steps.find(s => s.id === 'retrieving');
    expect(retrievingStep).toBeDefined();
    expect(retrievingStep?.outputPayload).toEqual({ sources: ['https://example.com/doc1'] });

    act(() => {
      result.current.overridePayload('planning', { query: 'research query' }, 'input');
    });

    const planningStep = result.current.steps.find(s => s.id === 'planning');
    expect(planningStep?.inputPayload).toEqual({ query: 'research query' });
  });

  it('stepPrev() transitions backwards', () => {
    const { result } = renderHook(() => usePipelineStepper());

    act(() => {
      result.current.stepNext();
      result.current.stepNext();
    });

    expect(result.current.currentStageIndex).toBe(2);
    expect(result.current.steps[2].status).toBe('active');

    act(() => {
      result.current.stepPrev();
    });

    expect(result.current.currentStageIndex).toBe(1);
    expect(result.current.steps[2].status).toBe('pending');
    expect(result.current.steps[1].status).toBe('active');
  });

  it('setStepStatus() updates status and error', () => {
    const { result } = renderHook(() => usePipelineStepper());

    act(() => {
      result.current.setStepStatus('retrieving', 'failed', 'Network timeout');
    });

    const step = result.current.steps.find(s => s.id === 'retrieving');
    expect(step?.status).toBe('failed');
    expect(step?.error).toBe('Network timeout');
  });

  it('reset() resets all steps to initial state', () => {
    const { result } = renderHook(() => usePipelineStepper());

    act(() => {
      result.current.stepNext();
      result.current.setStepStatus('planning', 'failed', 'Error');
    });

    expect(result.current.currentStageIndex).toBe(1);

    act(() => {
      result.current.reset();
    });

    expect(result.current.currentStageIndex).toBe(0);
    expect(result.current.steps[0].status).toBe('active');
    expect(result.current.steps[1].status).toBe('pending');
    expect(result.current.steps[1].error).toBeUndefined();
  });

  it('auditInvariants() detects honesty fake success violations', () => {
    const { result } = renderHook(() => usePipelineStepper());

    // Mark retrieving completed with empty sources payload
    act(() => {
      result.current.setStepStatus('retrieving', 'completed');
      result.current.overridePayload('retrieving', { sources: [] });
    });

    let violations: any[] = [];
    act(() => {
      violations = result.current.auditInvariants();
    });

    expect(violations.length).toBeGreaterThan(0);
    const honestyViolation = violations.find(v => v.kind === 'fake_success');
    expect(honestyViolation).toBeDefined();
    expect(honestyViolation?.location).toBe('retrieving.sources');
  });
});
