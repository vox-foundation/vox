import React from 'react';
import type {
  CompilerDiagnosticSpanDto,
  FileDiffAndDiagnosticsDto,
} from '../../../transport/t3Wave3Rpc';

export interface SpeculumLoupePopoverProps {
  readonly diffData: FileDiffAndDiagnosticsDto;
  readonly districtLabel?: string;
  readonly staged?: boolean;
  readonly busyAction?: 'stage' | 'revert' | 'fix' | null;
  readonly statusBanner?: string | null;
  readonly onStageFile: (filePath: string) => void;
  readonly onRevertFile: (filePath: string) => void;
  readonly onDispatchFix: (
    filePath: string,
    diagnostic: CompilerDiagnosticSpanDto | null,
    unifiedDiff: string,
  ) => void;
  readonly onClose: () => void;
}

export const SpeculumLoupePopover: React.FC<SpeculumLoupePopoverProps> = ({
  diffData,
  districtLabel,
  staged = false,
  busyAction = null,
  statusBanner = null,
  onStageFile,
  onRevertFile,
  onDispatchFix,
  onClose,
}) => {
  const diffLines = diffData.unified_diff
    .split('\n')
    .filter((l) => !l.startsWith('diff --git') && !l.startsWith('index '))
    .slice(0, 12);
  const primaryDiag = diffData.diagnostics[0] ?? null;

  return (
    <div
      data-testid="urbs-speculum-loupe-popover"
      role="dialog"
      aria-label={`Speculum Codicis — ${diffData.file_path}`}
      style={{
        width: 396,
        maxWidth: 'calc(100vw - 32px)',
        background: 'var(--color-bg-surface)',
        border: '1.5px solid var(--color-border-strong)',
        borderRadius: 10,
        boxShadow:
          '0 18px 42px rgba(28, 20, 12, 0.28), inset 0 1px 0 rgba(255, 255, 255, 0.25)',
        color: 'var(--color-text-primary)',
        padding: '10px 12px',
        fontFamily: 'var(--font-sans, Inter, system-ui, sans-serif)',
        backdropFilter: 'blur(12px)',
        zIndex: 42,
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 8,
          borderBottom: '1px solid var(--color-border-subtle)',
          paddingBottom: 6,
          marginBottom: 8,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, minWidth: 0 }}>
          <span
            style={{
              fontSize: 10,
              fontWeight: 800,
              letterSpacing: '0.09em',
              color: 'var(--color-accent-default)',
              background: 'var(--color-bg-elevated)',
              border: '1px solid var(--color-border-strong)',
              borderRadius: 4,
              padding: '2px 6px',
              whiteSpace: 'nowrap',
            }}
          >
            🔍 SPECVLVM CODICIS
          </span>
          {districtLabel && (
            <span
              style={{
                fontSize: 10,
                color: 'var(--color-text-secondary)',
                fontWeight: 600,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {districtLabel}
            </span>
          )}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <span
            data-testid="urbs-loupe-churn-badge"
            style={{
              fontSize: 10,
              fontFamily: "var(--font-sans, 'Palatino', Georgia, serif)",
              fontWeight: 700,
              padding: '1px 6px',
              borderRadius: 4,
              background: 'var(--color-bg-elevated)',
              border: '1px solid var(--color-border-subtle)',
              color: 'var(--color-text-primary)',
            }}
          >
            +{diffData.added_lines} / -{diffData.deleted_lines}
          </span>
          {staged && (
            <span
              style={{
                fontSize: 9,
                fontWeight: 700,
                padding: '1px 5px',
                borderRadius: 4,
                background: 'var(--color-bg-elevated)',
                color: 'var(--color-status-info)',
              }}
            >
              STAGED
            </span>
          )}
          <button
            type="button"
            data-testid="urbs-loupe-close"
            onClick={onClose}
            aria-label="Close Speculum Loupe"
            style={{
              background: 'var(--color-bg-elevated)',
              border: '1px solid var(--color-border-subtle)',
              color: 'var(--color-text-primary)',
              borderRadius: 4,
              width: 20,
              height: 20,
              fontSize: 11,
              cursor: 'pointer',
              lineHeight: 1,
            }}
          >
            ×
          </button>
        </div>
      </div>

      {/* File Path */}
      <div
        data-testid="urbs-loupe-file-path"
        style={{
          fontSize: 11,
          fontFamily: "var(--font-sans, 'Palatino', Georgia, serif)",
          color: 'var(--color-text-primary)',
          marginBottom: 6,
          wordBreak: 'break-all',
        }}
      >
        {diffData.file_path}
      </div>

      {/* Compiler Diagnostic Span Callout (if any) */}
      {diffData.diagnostics.length > 0 && (
        <div
          data-testid="urbs-loupe-diagnostics-box"
          style={{
            background: 'var(--color-bg-elevated)',
            border: '1px solid var(--color-status-fail)',
            borderRadius: 6,
            padding: '5px 8px',
            marginBottom: 7,
          }}
        >
          {diffData.diagnostics.slice(0, 2).map((d, idx) => (
            <div
              key={`${d.file}:${d.line}:${idx}`}
              style={{
                fontSize: 10.5,
                fontFamily: "var(--font-sans, 'Palatino', Georgia, serif)",
                color: 'var(--color-text-primary)',
                lineHeight: 1.35,
              }}
            >
              <strong style={{ color: 'var(--color-status-fail)' }}>
                [{d.code ?? d.severity.toUpperCase()} {d.line}:{d.column}]
              </strong>{' '}
              {d.message}
            </div>
          ))}
        </div>
      )}

      {/* Unified Git Diff Hunk */}
      <div
        data-testid="urbs-loupe-diff-hunk"
        style={{
          background: 'var(--color-bg-elevated)',
          border: '1px solid var(--color-border-subtle)',
          borderRadius: 6,
          padding: '6px 8px',
          maxHeight: 132,
          overflowY: 'auto',
          fontFamily: "var(--font-sans, 'Palatino', Georgia, serif)",
          fontSize: 10.5,
          lineHeight: 1.4,
          marginBottom: 8,
        }}
      >
        {diffLines.map((line, i) => {
          const isAdd = line.startsWith('+') && !line.startsWith('+++');
          const isDel = line.startsWith('-') && !line.startsWith('---');
          const isHdr = line.startsWith('@@');
          return (
            <div
              key={i}
              style={{
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-all',
                padding: '0 4px',
                borderRadius: 2,
                background: isAdd
                  ? 'rgba(27, 107, 58, 0.14)'
                  : isDel
                    ? 'rgba(158, 43, 37, 0.14)'
                    : isHdr
                      ? 'var(--color-overlay-subtle)'
                      : 'transparent',
                color: isAdd
                  ? 'var(--color-status-pass)'
                  : isDel
                    ? 'var(--color-status-fail)'
                    : isHdr
                      ? 'var(--color-accent-default)'
                      : 'var(--color-text-primary)',
              }}
            >
              {line}
            </div>
          );
        })}
      </div>

      {statusBanner && (
        <div
          data-testid="urbs-loupe-status-banner"
          style={{
            fontSize: 10,
            color: 'var(--color-status-pass)',
            marginBottom: 6,
            fontWeight: 600,
          }}
        >
          {statusBanner}
        </div>
      )}

      {/* 1-Click Developer Action Bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          flexWrap: 'wrap',
        }}
      >
        <button
          type="button"
          data-testid="urbs-loupe-stage-btn"
          disabled={busyAction !== null}
          onClick={() => onStageFile(diffData.file_path)}
          style={{
            flex: 1,
            padding: '5px 8px',
            fontSize: 10.5,
            fontWeight: 700,
            borderRadius: 5,
            cursor: 'pointer',
            background: 'var(--color-bg-elevated)',
            border: '1px solid var(--color-status-pass)',
            color: 'var(--color-status-pass)',
            whiteSpace: 'nowrap',
          }}
        >
          ✓ Stage File
        </button>
        <button
          type="button"
          data-testid="urbs-loupe-revert-btn"
          disabled={busyAction !== null}
          onClick={() => onRevertFile(diffData.file_path)}
          style={{
            flex: 1,
            padding: '5px 8px',
            fontSize: 10.5,
            fontWeight: 700,
            borderRadius: 5,
            cursor: 'pointer',
            background: 'var(--color-bg-elevated)',
            border: '1px solid var(--color-status-fail)',
            color: 'var(--color-status-fail)',
            whiteSpace: 'nowrap',
          }}
        >
          ↺ Revert File
        </button>
        <button
          type="button"
          data-testid="urbs-loupe-fix-btn"
          disabled={busyAction !== null}
          onClick={() =>
            onDispatchFix(diffData.file_path, primaryDiag, diffData.unified_diff)
          }
          style={{
            flex: 1.2,
            padding: '5px 8px',
            fontSize: 10.5,
            fontWeight: 700,
            borderRadius: 5,
            cursor: 'pointer',
            background: 'var(--color-bg-elevated)',
            border: '1px solid var(--color-border-strong)',
            color: 'var(--color-text-primary)',
            whiteSpace: 'nowrap',
          }}
        >
          ⚒ Fix Exact Error
        </button>
      </div>
    </div>
  );
};
