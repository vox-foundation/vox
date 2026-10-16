import React from 'react';

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
      className={`mt-3 rounded-xl border p-4 space-y-2 ${
        verification.passed
          ? 'border-emerald-500/30 bg-emerald-500/[0.05]'
          : 'border-amber-500/30 bg-amber-500/[0.05]'
      }`}
    >
      <div className="flex items-center justify-between flex-wrap gap-2">
        <div className="flex items-center gap-2">
          <span
            className={`rounded px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider ${
              verification.passed
                ? 'bg-emerald-500/20 text-emerald-300'
                : 'bg-amber-500/20 text-amber-300'
            }`}
          >
            {verification.passed ? '✓ Compiler Verified' : '⚠ Compile Diagnostics'}
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
        <pre className="max-h-32 overflow-auto rounded border border-border-subtle bg-black/50 p-2 font-mono text-[11px] text-amber-200/90">
          {verification.diagnostics.join('\n')}
        </pre>
      )}
    </div>
  );
}
