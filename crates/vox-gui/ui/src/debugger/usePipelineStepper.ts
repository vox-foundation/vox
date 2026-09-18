import { useReducer, useCallback, useEffect } from 'react';
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

interface StepperState {
  steps: DebugStep[];
  currentStageIndex: number;
  isPaused: boolean;
  autoPlay: boolean;
  violations: InvariantViolation[];
}

type StepperAction =
  | { type: 'STEP_NEXT' }
  | { type: 'STEP_PREV' }
  | { type: 'PAUSE' }
  | { type: 'RESUME' }
  | { type: 'RESET'; initialSteps?: DebugStep[]; initialPaused: boolean }
  | { type: 'OVERRIDE_PAYLOAD'; stageId: string; payload: unknown; target?: 'input' | 'output' }
  | { type: 'SET_STEP_STATUS'; stageId: string; status: StepStatus; error?: string }
  | { type: 'SET_VIOLATIONS'; violations: InvariantViolation[] };

function stepperReducer(state: StepperState, action: StepperAction): StepperState {
  switch (action.type) {
    case 'STEP_NEXT': {
      const maxIdx = state.steps.length - 1;
      if (state.currentStageIndex >= maxIdx) {
        return {
          ...state,
          steps: state.steps.map((s, idx) =>
            idx === state.currentStageIndex ? { ...s, status: 'completed' } : s
          ),
          autoPlay: false,
          isPaused: true,
        };
      }
      const nextIdx = state.currentStageIndex + 1;
      return {
        ...state,
        currentStageIndex: nextIdx,
        steps: state.steps.map((s, idx) => {
          if (idx === state.currentStageIndex) return { ...s, status: 'completed' };
          if (idx === nextIdx) return { ...s, status: 'active' };
          return s;
        }),
      };
    }
    case 'STEP_PREV': {
      if (state.currentStageIndex <= 0) return state;
      const prevIdx = state.currentStageIndex - 1;
      return {
        ...state,
        currentStageIndex: prevIdx,
        steps: state.steps.map((s, idx) => {
          if (idx === state.currentStageIndex) return { ...s, status: 'pending' };
          if (idx === prevIdx) return { ...s, status: 'active' };
          return s;
        }),
      };
    }
    case 'PAUSE':
      return {
        ...state,
        isPaused: true,
        autoPlay: false,
      };
    case 'RESUME':
      return {
        ...state,
        isPaused: false,
        autoPlay: true,
      };
    case 'RESET':
      return {
        steps: action.initialSteps ? [...action.initialSteps] : createInitialSteps(),
        currentStageIndex: 0,
        isPaused: action.initialPaused,
        autoPlay: false,
        violations: [],
      };
    case 'OVERRIDE_PAYLOAD':
      return {
        ...state,
        steps: state.steps.map(step => {
          if (step.id !== action.stageId) return step;
          if (action.target === 'input') {
            return { ...step, inputPayload: action.payload };
          }
          if (action.target === 'output') {
            return { ...step, outputPayload: action.payload };
          }
          if (
            action.payload &&
            typeof action.payload === 'object' &&
            ('input' in action.payload || 'output' in action.payload)
          ) {
            const p = action.payload as { input?: unknown; output?: unknown };
            return {
              ...step,
              inputPayload: p.input !== undefined ? p.input : step.inputPayload,
              outputPayload: p.output !== undefined ? p.output : step.outputPayload,
            };
          }
          return { ...step, outputPayload: action.payload };
        }),
      };
    case 'SET_STEP_STATUS':
      return {
        ...state,
        steps: state.steps.map(step => {
          if (step.id !== action.stageId) return step;
          return {
            ...step,
            status: action.status,
            error: action.status === 'failed' ? (action.error ?? step.error ?? 'Step failed') : undefined,
          };
        }),
      };
    case 'SET_VIOLATIONS':
      return {
        ...state,
        violations: action.violations,
      };
    default:
      return state;
  }
}

export function usePipelineStepper(options?: UsePipelineStepperOptions): UsePipelineStepperReturn {
  const initialPaused = options?.initialPaused ?? !options?.autoPlay;

  const [state, dispatch] = useReducer(stepperReducer, undefined, () => ({
    steps: options?.initialSteps ? [...options.initialSteps] : createInitialSteps(),
    currentStageIndex: 0,
    isPaused: initialPaused,
    autoPlay: options?.autoPlay ?? false,
    violations: [],
  }));

  const stepNext = useCallback(() => {
    dispatch({ type: 'STEP_NEXT' });
  }, []);

  const stepPrev = useCallback(() => {
    dispatch({ type: 'STEP_PREV' });
  }, []);

  const pause = useCallback(() => {
    dispatch({ type: 'PAUSE' });
  }, []);

  const resume = useCallback(() => {
    dispatch({ type: 'RESUME' });
  }, []);

  const reset = useCallback(() => {
    dispatch({
      type: 'RESET',
      initialSteps: options?.initialSteps,
      initialPaused: options?.initialPaused ?? !options?.autoPlay,
    });
  }, [options?.initialSteps, options?.initialPaused, options?.autoPlay]);

  const overridePayload = useCallback(
    (stageId: string, payload: unknown, target?: 'input' | 'output') => {
      dispatch({ type: 'OVERRIDE_PAYLOAD', stageId, payload, target });
    },
    []
  );

  const setStepStatus = useCallback((stageId: string, status: StepStatus, error?: string) => {
    dispatch({ type: 'SET_STEP_STATUS', stageId, status, error });
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
      const allProviderStatuses = auditOpts?.providerProbeStatuses ?? options?.providerProbeStatuses;
      for (const step of state.steps) {
        if (step.status === 'completed') {
          // Scope providerProbeStatuses strictly to retrieval stages
          const providerStatuses = step.id === 'retrieving' ? allProviderStatuses : undefined;
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

      dispatch({ type: 'SET_VIOLATIONS', violations: foundViolations });
      return foundViolations;
    },
    [state.steps, options?.providerProbeStatuses]
  );

  useEffect(() => {
    if (!state.autoPlay || state.isPaused) return;
    const interval = setInterval(() => {
      dispatch({ type: 'STEP_NEXT' });
    }, options?.stepIntervalMs ?? 1000);
    return () => clearInterval(interval);
  }, [state.autoPlay, state.isPaused, options?.stepIntervalMs]);

  const currentStep = state.steps[state.currentStageIndex];

  return {
    steps: state.steps,
    currentStageIndex: state.currentStageIndex,
    currentStep,
    isPaused: state.isPaused,
    autoPlay: state.autoPlay,
    violations: state.violations,
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
