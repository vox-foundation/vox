import { useReducer, useCallback, useEffect } from 'react';
import { RESEARCH_STAGES } from '../lib/pipeline';
import type { DebugStep, StepStatus, InvariantViolation } from './types';
import { checkHonestyInvariant } from './sentries/sentryHonesty';
import { checkErrorLeakInvariant } from './sentries/sentryErrorLeak';

const DEFAULT_STAGE_PAYLOADS: Record<string, { input: Record<string, unknown>; output: Record<string, unknown> }> = {
  queued: {
    input: { query: 'hybrid search architecture', scope: 'web', max_sources: 5 },
    output: { session_id: 1, status: 'queued', enqueued_at_ms: 1717000000000 },
  },
  planning: {
    input: { query: 'hybrid search architecture', entropy: 3.42, max_depth: 2 },
    output: {
      subqueries: [
        'BM25 vs dense retrieval tradeoffs',
        'Reciprocal Rank Fusion k=60',
        'cross-encoder reranking latency',
      ],
      target_sources: 5,
      domain_filter: ['arxiv.org', 'github.com', 'semanticscholar.org'],
    },
  },
  retrieving: {
    input: {
      subqueries: [
        'BM25 vs dense retrieval tradeoffs',
        'Reciprocal Rank Fusion k=60',
        'cross-encoder reranking latency',
      ],
      providers: ['searxng', 'tavily', 'openalex', 'arxiv', 'wikipedia'],
    },
    output: {
      sources: ['https://example.com/hybrid-search', 'https://example.com/rrf-fusion-benchmark', 'https://example.com/vector-db-ann'],
      hits_retrieved: 8,
      unique_domains: 5,
      rrf_top_score: 0.0328,
    },
  },
  verifying_claims: {
    input: { candidate_snippets: 8, nli_model: 'mens-judge-v1', threshold: 0.75 },
    output: {
      extracted_claims: 3,
      verdicts: { supported: 2, contested: 1, refuted: 0 },
      claims: [
        { claim_id: 'c1', text: 'Vector DBs trade exact recall for sub-millisecond retrieval latency at scale.', verdict: 'Supported', confidence: 0.95 },
        { claim_id: 'c2', text: 'Reciprocal Rank Fusion (RRF) with k=60 outperforms naive linear combination.', verdict: 'Supported', confidence: 0.92 },
        { claim_id: 'c3', text: 'Single-source retrieval without corroboration exhibits higher epistemic variance.', verdict: 'Contested', confidence: 0.65 },
      ],
    },
  },
  synthesizing: {
    input: { supported_claims: 2, contested_claims: 1, style: 'technical_deep_dive' },
    output: { markdown_length_chars: 4218, citation_density: 0.42, section_count: 4, synthesis_token_count: 850 },
  },
  auditing_citations: {
    input: { total_citations: 5, distinct_domains: 3, raw_claims_count: 3 },
    output: { citation_precision: 1.0, orphan_citations: 0, corroboration_ratio: 0.67, status: 'passed' },
  },
  persisting: {
    input: { session_id: 1, format: 'vox_db_and_markdown', doc_path: 'docs/research/hybrid-search.md' },
    output: { db_record_id: 1, file_bytes: 4218, persisted_at_ms: 1717000001250 },
  },
  completed: {
    input: { session_id: 1, duration_ms: 1250 },
    output: { status: 'success', total_claims: 3, verified_sources: 3, honesty_invariant_passed: true },
  },
};

export function createInitialSteps(): DebugStep[] {
  return RESEARCH_STAGES.map((stage, idx) => ({
    id: stage,
    label: stage.replace(/_/g, ' '),
    status: idx === 0 ? 'active' : 'pending',
    inputPayload: DEFAULT_STAGE_PAYLOADS[stage]?.input,
    outputPayload: DEFAULT_STAGE_PAYLOADS[stage]?.output,
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
