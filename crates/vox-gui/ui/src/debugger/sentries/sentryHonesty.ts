import type { InvariantViolation, StepStatus } from '../types';

export interface HonestyAuditContext {
  status: StepStatus;
  payload: Record<string, unknown> | null | undefined;
  requiredFields: string[];
  stageName: string;
  providerProbeStatuses?: Array<{ provider: string; ok: boolean; httpStatus: number; hitCount: number }>;
  domContainer?: HTMLElement | null;
}

export function checkHonestyInvariant(ctx: HonestyAuditContext): InvariantViolation | null {
  if (ctx.status !== 'completed') return null;

  // 1. If providers all failed, this is an infrastructure failure (checked first)
  const allProvidersFailed =
    ctx.providerProbeStatuses &&
    ctx.providerProbeStatuses.length > 0 &&
    ctx.providerProbeStatuses.every((p) => !p.ok || p.httpStatus >= 400);

  if (allProvidersFailed) {
    return {
      kind: 'fake_success',
      severity: 'critical',
      message: `${ctx.stageName} claimed completion, but all search providers failed. Expected error state.`,
      location: ctx.stageName,
    };
  }

  const emptyField = ctx.requiredFields.find((f) => {
    const val = ctx.payload?.[f];
    return val === null || val === undefined || (Array.isArray(val) && val.length === 0) || val === '';
  });

  if (!emptyField) return null;

  // 2. Check if the UI honestly rendered an acknowledged empty state
  const hasEmptyNotice = Boolean(
    ctx.domContainer?.matches?.('[data-testid="empty-results-notice"]') ||
    ctx.domContainer?.querySelector?.('[data-testid="empty-results-notice"]')
  );
  const isAcknowledged = ctx.payload?.emptyStateAcknowledged === true;

  if (hasEmptyNotice || isAcknowledged) {
    return null; // Valid honest empty search
  }

  // 3. Groundless fake success
  return {
    kind: 'fake_success',
    severity: 'critical',
    message: `${ctx.stageName} completed with zero ${emptyField} without rendering an honest empty-state notice.`,
    location: `${ctx.stageName}.${emptyField}`,
  };
}
