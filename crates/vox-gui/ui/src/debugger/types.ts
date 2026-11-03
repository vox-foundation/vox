export type StepStatus = 'pending' | 'active' | 'paused' | 'completed' | 'failed' | 'skipped';

export interface DebugStep<TInput = unknown, TOutput = unknown> {
  id: string;
  label: string;
  status: StepStatus;
  inputPayload?: TInput;
  outputPayload?: TOutput;
  error?: string;
  timingMs?: number;
}

export type InvariantSeverity = 'critical' | 'major' | 'minor' | 'info';

export interface InvariantViolation {
  kind: 'fake_success' | 'raw_error_leak' | 'occlusion' | 'watchdog_stalled';
  severity: InvariantSeverity;
  message: string;
  elementSelector?: string;
  location?: string;
  rawDetails?: unknown;
}
