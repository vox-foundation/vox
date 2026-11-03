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

  it('stepNext() at final stage marks completed, halts autoPlay, and sets isPaused to true', () => {
    const { result } = renderHook(() =>
      usePipelineStepper({ autoPlay: true, stepIntervalMs: 1000 })
    );

    // Step through all stages to the end
    for (let i = 0; i < 7; i++) {
      act(() => {
        result.current.stepNext();
      });
    }

    expect(result.current.currentStageIndex).toBe(7);
    expect(result.current.steps[7].status).toBe('active');

    // Terminal stepNext
    act(() => {
      result.current.stepNext();
    });

    expect(result.current.currentStageIndex).toBe(7);
    expect(result.current.steps[7].status).toBe('completed');
    expect(result.current.autoPlay).toBe(false);
    expect(result.current.isPaused).toBe(true);
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

  it('overridePayload() honors explicit target parameter', () => {
    const { result } = renderHook(() => usePipelineStepper());

    // Explicit output
    act(() => {
      result.current.overridePayload('retrieving', { sources: ['https://example.com/doc1'] }, 'output');
    });

    const retrievingStep = result.current.steps.find(s => s.id === 'retrieving');
    expect(retrievingStep).toBeDefined();
    expect(retrievingStep?.outputPayload).toEqual({ sources: ['https://example.com/doc1'] });

    // Explicit input
    act(() => {
      result.current.overridePayload('retrieving', { query: 'test query' }, 'input');
    });

    const retrievingStepInput = result.current.steps.find(s => s.id === 'retrieving');
    expect(retrievingStepInput?.inputPayload).toEqual({ query: 'test query' });
    expect(retrievingStepInput?.outputPayload).toEqual({ sources: ['https://example.com/doc1'] });
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

  it('setStepStatus() updates status and clears error when transitioning to non-failed', () => {
    const { result } = renderHook(() => usePipelineStepper());

    act(() => {
      result.current.setStepStatus('retrieving', 'failed', 'Network timeout');
    });

    let step = result.current.steps.find(s => s.id === 'retrieving');
    expect(step?.status).toBe('failed');
    expect(step?.error).toBe('Network timeout');

    // Transition to completed clears error
    act(() => {
      result.current.setStepStatus('retrieving', 'completed');
    });

    step = result.current.steps.find(s => s.id === 'retrieving');
    expect(step?.status).toBe('completed');
    expect(step?.error).toBeUndefined();
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

  it('auditInvariants() scopes provider probe status strictly to retrieval stage', () => {
    const { result } = renderHook(() => usePipelineStepper());

    // Complete planning with valid data and retrieving with valid data
    act(() => {
      result.current.setStepStatus('planning', 'completed');
      result.current.overridePayload('planning', { plan: 'valid plan' });
      result.current.setStepStatus('retrieving', 'completed');
      result.current.overridePayload('retrieving', { sources: ['https://example.com'] });
    });

    // Run audit with all providers failed
    let violations: any[] = [];
    act(() => {
      violations = result.current.auditInvariants({
        providerProbeStatuses: [{ provider: 'searxng', ok: false, httpStatus: 502, hitCount: 0 }],
      });
    });

    // Only retrieving should have honesty violation due to failed search providers, NOT planning
    const fakeSuccessViolations = violations.filter(v => v.kind === 'fake_success');
    expect(fakeSuccessViolations).toHaveLength(1);
    expect(fakeSuccessViolations[0].location).toBe('retrieving');
  });
});
