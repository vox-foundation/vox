import React from 'react';
import { Icon } from '../../ui/Icons';

export interface CodeGenVerificationDto {
  passed: boolean;
  compiler?: string;
  compiler_exit_code?: number;
  snippet_count?: number;
  repair_waves?: number;
  diagnostics?: string[];
  verified_snippet?: string;
  verified_symbols?: string[];
}

export type CodeGenVerificationPayload = CodeGenVerificationDto;

export function CodeGenVerificationCard({
  verification,
}: {
  verification: CodeGenVerificationDto;
}) {
  return (
    <div
      data-testid="codegen-verification-card"
      className={`mt-3 rounded-xl border p-4 space-y-2 bg-bg-surface ${
        verification.passed
          ? 'border-(--color-status-pass)/40'
          : 'border-(--color-status-warn)/40'
      }`}
    >
      <div className="flex items-center justify-between flex-wrap gap-2">
        <div className="flex items-center gap-2">
          <span
            className={`inline-flex items-center gap-1 rounded border px-2 py-0.5 text-[11px] font-medium uppercase tracking-wider bg-bg-elevated ${
              verification.passed
                ? 'border-(--color-status-pass)/35 text-(--color-status-pass)'
                : 'border-(--color-status-warn)/35 text-(--color-status-warn)'
            }`}
          >
            {verification.passed ? (
              <>
                <Icon.check className="size-3 shrink-0" aria-hidden="true" />
                <span>Compiler Verified</span>
              </>
            ) : (
              <>
                <Icon.alert className="size-3 shrink-0" aria-hidden="true" />
                <span>Compile Diagnostics</span>
              </>
            )}
          </span>
          <span className="font-mono text-xs text-text-secondary">
            {verification.compiler ?? 'cargo check / rustc'}
          </span>
        </div>
        {verification.repair_waves != null && (
          <span className="font-mono text-[11px] text-text-muted">
            Repair waves: {verification.repair_waves}
          </span>
        )}
      </div>

      {verification.diagnostics && verification.diagnostics.length > 0 && (
        <pre className="max-h-32 overflow-auto rounded border border-border-subtle bg-bg-elevated p-2 font-mono text-[11px] text-(--color-status-warn)">
          {verification.diagnostics.join('\n')}
        </pre>
      )}
    </div>
  );
}
