import { useState, useCallback, useEffect, useRef } from 'react';
import { RESEARCH_STAGES } from '../lib/pipeline';
import type { DebugStep, StepStatus, InvariantViolation } from './types';
import { checkHonestyInvariant } from './sentries/sentryHonesty';
import { checkErrorLeakInvariant } from './sentries/sentryErrorLeak';

export function createInitialSteps(): DebugStep[] {
  return RESEARCH_STAGES.map((stage, idx) => ({
    id: stage,
    label: stage.replace(/_/g, ' '),
    status: idx === 0 ? 'active' : 'pending',
  }));
}

export interface UsePipelineStepperOptions {
  initialSteps?: DebugStep[];
  autoPlay?: boolean;
  stepIntervalMs?: number;
  initialPaused?: boolean;
  providerProbeStatuses?: Array<{ provider: string; ok: boolean; httpStatus: number; hitCount: number }>;
}

export interface UsePipelineStepperReturn {
  steps: DebugStep[];
  currentStageIndex: number;
  currentStep: DebugStep | undefined;
  isPaused: boolean;
  autoPlay: boolean;
  violations: InvariantViolation[];
  stepNext: () => void;
  stepPrev: () => void;
  pause: () => void;
  resume: () => void;
  reset: () => void;
  overridePayload: (stageId: string, payload: unknown, target?: 'input' | 'output') => void;
  setStepStatus: (stageId: string, status: StepStatus, error?: string) => void;
  auditInvariants: (opts?: {
    root?: Element | null;
    providerProbeStatuses?: Array<{ provider: string; ok: boolean; httpStatus: number; hitCount: number }>;
    domContainer?: HTMLElement | null;
    requiredFieldsMap?: Record<string, string[]>;
  }) => InvariantViolation[];
}

export function usePipelineStepper(options?: UsePipelineStepperOptions): UsePipelineStepperReturn {
  const [steps, setSteps] = useState<DebugStep[]>(() =>
    options?.initialSteps ? [...options.initialSteps] : createInitialSteps()
  );
  const [currentStageIndex, setCurrentStageIndex] = useState(0);
  const [isPaused, setIsPaused] = useState(options?.initialPaused ?? false);
  const [autoPlay, setAutoPlay] = useState(options?.autoPlay ?? false);
  const [violations, setViolations] = useState<InvariantViolation[]>([]);

  const stepsLengthRef = useRef(steps.length);
  useEffect(() => {
    stepsLengthRef.current = steps.length;
  }, [steps.length]);

  const stepNext = useCallback(() => {
    setCurrentStageIndex(curr => {
      const maxIdx = stepsLengthRef.current - 1;
      if (curr >= maxIdx) {
        setSteps(prev => prev.map((s, idx) => (idx === curr ? { ...s, status: 'completed' } : s)));
        setAutoPlay(false);
        return curr;
      }
      const next = curr + 1;
      setSteps(prev =>
        prev.map((s, idx) => {
          if (idx === curr) return { ...s, status: 'completed' };
          if (idx === next) return { ...s, status: 'active' };
          return s;
        })
      );
      return next;
    });
  }, []);

  const stepPrev = useCallback(() => {
    setCurrentStageIndex(curr => {
      if (curr <= 0) return 0;
      const prevIdx = curr - 1;
      setSteps(prev =>
        prev.map((s, idx) => {
          if (idx === curr) return { ...s, status: 'pending' };
          if (idx === prevIdx) return { ...s, status: 'active' };
          return s;
        })
      );
      return prevIdx;
    });
  }, []);

  const pause = useCallback(() => {
    setIsPaused(true);
    setAutoPlay(false);
  }, []);

  const resume = useCallback(() => {
    setIsPaused(false);
    setAutoPlay(true);
  }, []);

  const reset = useCallback(() => {
    setSteps(options?.initialSteps ? [...options.initialSteps] : createInitialSteps());
    setCurrentStageIndex(0);
    setIsPaused(options?.initialPaused ?? false);
    setAutoPlay(false);
    setViolations([]);
  }, [options?.initialSteps, options?.initialPaused]);

  const overridePayload = useCallback(
    (stageId: string, payload: unknown, target: 'input' | 'output' = 'output') => {
      setSteps(prev =>
        prev.map(step => {
          if (step.id !== stageId) return step;
          if (payload && typeof payload === 'object' && ('input' in payload || 'output' in payload)) {
            const p = payload as { input?: unknown; output?: unknown };
            return {
              ...step,
              inputPayload: p.input !== undefined ? p.input : step.inputPayload,
              outputPayload: p.output !== undefined ? p.output : step.outputPayload,
            };
          }
          if (target === 'input') {
            return { ...step, inputPayload: payload };
          }
          return { ...step, outputPayload: payload };
        })
      );
    },
    []
  );

  const setStepStatus = useCallback((stageId: string, status: StepStatus, error?: string) => {
    setSteps(prev =>
      prev.map(step => {
        if (step.id !== stageId) return step;
        return {
          ...step,
          status,
          ...(error !== undefined ? { error } : {}),
        };
      })
    );
  }, []);

  const auditInvariants = useCallback(
    (auditOpts?: {
      root?: Element | null;
      providerProbeStatuses?: Array<{ provider: string; ok: boolean; httpStatus: number; hitCount: number }>;
      domContainer?: HTMLElement | null;
      requiredFieldsMap?: Record<string, string[]>;
    }): InvariantViolation[] => {
      const foundViolations: InvariantViolation[] = [];

      // 1. Check for runtime exceptions leaked into system chrome
      const leaks = checkErrorLeakInvariant(auditOpts?.root);
      foundViolations.push(...leaks);

      // 2. Check for fake success honesty invariants across completed steps
      const providerStatuses = auditOpts?.providerProbeStatuses ?? options?.providerProbeStatuses;
      for (const step of steps) {
        if (step.status === 'completed') {
          const requiredFields =
            auditOpts?.requiredFieldsMap?.[step.id] ?? (step.id === 'retrieving' ? ['sources'] : []);
          const violation = checkHonestyInvariant({
            status: step.status,
            payload: (step.outputPayload as Record<string, unknown>) ?? null,
            requiredFields,
            stageName: step.id,
            providerProbeStatuses: providerStatuses,
            domContainer: auditOpts?.domContainer,
          });
          if (violation) {
            foundViolations.push(violation);
          }
        }
      }

      setViolations(foundViolations);
      return foundViolations;
    },
    [steps, options?.providerProbeStatuses]
  );

  useEffect(() => {
    if (!autoPlay || isPaused) return;
    const interval = setInterval(() => {
      stepNext();
    }, options?.stepIntervalMs ?? 1000);
    return () => clearInterval(interval);
  }, [autoPlay, isPaused, stepNext, options?.stepIntervalMs]);

  const currentStep = steps[currentStageIndex];

  return {
    steps,
    currentStageIndex,
    currentStep,
    isPaused,
    autoPlay,
    violations,
    stepNext,
    stepPrev,
    pause,
    resume,
    reset,
    overridePayload,
    setStepStatus,
    auditInvariants,
  };
}
