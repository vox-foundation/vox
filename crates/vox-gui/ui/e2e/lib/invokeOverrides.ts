import type { Page } from '@playwright/test';

export interface InvokeOverrides {
  /** Command → value answered instead of the base mock. */
  responses?: Record<string, unknown>;
  /** Commands that throw (to exercise failure paths). */
  reject?: string[];
}

/**
 * Wraps the Tauri invoke mock installed before it (add this AFTER `addMockInitScript` / `addRichMockInitScript`):
 * listed commands answer from `responses` or throw when in `reject`; everything else falls through to the base mock.
 * Self-contained: `addInitScript` serialises only the function body.
 */
export async function addInvokeOverrides(page: Page, overrides: InvokeOverrides): Promise<void> {
  await page.addInitScript(
    (arg: { responses: Record<string, unknown>; reject: string[] }) => {
      const internals = (window as any).__TAURI_INTERNALS__;
      const base = internals?.invoke;
      if (typeof base !== 'function') throw new Error('addInvokeOverrides must run after a base mock');
      internals.invoke = async (cmd: string, args?: any) => {
        if (arg.reject.includes(cmd)) throw new Error(`${cmd} unavailable (test)`);
        if (Object.prototype.hasOwnProperty.call(arg.responses, cmd)) return arg.responses[cmd];
        return base(cmd, args);
      };
    },
    { responses: overrides.responses ?? {}, reject: overrides.reject ?? [] },
  );
}
