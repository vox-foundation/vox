import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { __resetBackendAvailabilityForTests } from '../../../lib/backendGuard';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { OratioStudioView } from './OratioStudioView';
import { surfaceDecorators } from '../decoratorRegistry';

describe('OratioStudioView (Slice 435-A · AXIS-BENTO-01)', () => {
  beforeEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_gui_preference') return Promise.resolve(null);
      if (cmd === 'set_gui_preference') return Promise.resolve();
      if (cmd === 'execute_command') {
        const path = (args?.path as string[] | undefined)?.join(' ');
        if (path === 'oratio doctor') {
          return Promise.resolve({
            exit_code: 0,
            stdout: 'runtime: healthy | backend: whisper-local',
            stderr: '',
          });
        }
        if (path === 'oratio status') {
          return Promise.resolve({
            exit_code: 0,
            stdout: 'session: idle | sample_rate: 16kHz | VAD: silero-v5',
            stderr: '',
          });
        }
        return Promise.resolve({ exit_code: 0, stdout: 'ok', stderr: '' });
      }
      if (cmd === 'oratio_transcribe') {
        return Promise.resolve({
          text: 'vox check --workspace',
          raw_text: 'vox check workspace',
          refined_text: 'vox check --workspace',
        });
      }
      return Promise.resolve(null);
    });
  });

  it('registers OratioStudioView on surfaceDecorators.oratio and renders Bento hero studio without raw <pre> dumps', async () => {
    expect(surfaceDecorators.oratio).toBe(OratioStudioView);
    const pushToast = vi.fn();
    const { container } = render(<OratioStudioView pushToast={pushToast} />);

    expect(screen.getByRole('heading', { name: 'Vox Oratio' })).toBeInTheDocument();
    expect(screen.getByTestId('oratio-vu-stage')).toBeInTheDocument();
    expect(screen.getByTestId('oratio-harmonic-bars')).toBeInTheDocument();
    expect(container.querySelector('[data-bento-preset="hero"]')).not.toBeNull();
    expect(screen.getByRole('region', { name: 'Oratio transcript history' })).toBeInTheDocument();
    expect(screen.getByTestId('oratio-empty-ledger')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('whisper-local')).toBeInTheDocument();
      expect(screen.getByText('silero-v5')).toBeInTheDocument();
    });

    expect(container.querySelectorAll('pre').length).toBe(0);
    const bar = screen.getByTestId('oratio-harmonic-bars').querySelector('[style*="background-color"]');
    expect(bar?.getAttribute('style')).toMatch(/var\(--color-(primary|border-strong)\)/);
  });

  it('captures transcript via Push-to-Talk, updates transcript ledger & gauge, and persists history', async () => {
    const pushToast = vi.fn();
    render(<OratioStudioView pushToast={pushToast} />);

    await waitFor(() => expect(screen.getByText('2/2 OK')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('oratio-ptt-capture-btn'));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('oratio_transcribe', { seconds: 5 });
      expect(screen.getByTestId('oratio-utterance-row')).toHaveTextContent('vox check --workspace');
      expect(screen.getByTestId('oratio-utterance-row')).toHaveTextContent('raw: vox check workspace');
      expect(screen.getByText('1 LOG')).toBeInTheDocument();
    });

    expect(invokeMock).toHaveBeenCalledWith(
      'set_gui_preference',
      expect.objectContaining({ key: 'vox.oratio.history.v1' }),
    );
  });

  it('persists RomanRotaryKnob changes and rolls back with sanitized toast when preference write fails', async () => {
    const pushToast = vi.fn();
    render(<OratioStudioView pushToast={pushToast} />);

    const vadKnob = screen.getByRole('slider', { name: 'Oratio VAD threshold' });
    const winKnob = screen.getByRole('slider', { name: 'Oratio capture window seconds' });
    expect(vadKnob).toHaveAttribute('aria-valuenow', '45');
    expect(winKnob).toHaveAttribute('aria-valuenow', '5');

    fireEvent.keyDown(vadKnob, { key: 'ArrowUp' });
    await waitFor(() => {
      expect(vadKnob).toHaveAttribute('aria-valuenow', '50');
      expect(invokeMock).toHaveBeenCalledWith('set_gui_preference', {
        key: 'vox.oratio.vad_threshold.v1',
        value: '50',
      });
    });

    // Reject next preference write with a raw Tauri error to verify rollback + sanitization
    invokeMock.mockImplementationOnce(() =>
      Promise.reject(new TypeError('window.__TAURI_INTERNALS__.invoke failed')),
    );
    fireEvent.keyDown(winKnob, { key: 'ArrowUp' });

    await waitFor(() => {
      expect(winKnob).toHaveAttribute('aria-valuenow', '5');
      expect(pushToast).toHaveBeenCalledWith(
        expect.objectContaining({
          tone: 'warn',
          title: 'Capture window rollback',
          body: 'An unexpected error occurred.',
        }),
      );
    });
  });

  it('surfaces sanitized capture error alert and filters studio focus via RomanBrassGauge clicks', async () => {
    const pushToast = vi.fn();
    render(<OratioStudioView pushToast={pushToast} />);

    await waitFor(() => expect(screen.getByText('2/2 OK')).toBeInTheDocument());

    const probesGauge = screen.getByRole('button', { name: /Probes/i });
    fireEvent.click(probesGauge);
    expect(screen.getByTestId('oratio-studio-view')).toHaveAttribute('data-studio-focus', 'probes');

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'oratio_transcribe') {
        return Promise.reject(new Error('No default input microphone found'));
      }
      return Promise.resolve(null);
    });

    fireEvent.click(screen.getByTestId('oratio-ptt-capture-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('oratio-capture-error')).toHaveTextContent(
        'No default input microphone found',
      );
      expect(pushToast).toHaveBeenCalledWith(
        expect.objectContaining({
          tone: 'warn',
          title: 'Oratio capture failed',
        }),
      );
    });
  });
});
