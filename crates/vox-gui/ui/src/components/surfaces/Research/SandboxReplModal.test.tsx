// @vitest-environment jsdom
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { SandboxReplModal } from './SandboxReplModal';
import * as researchActions from './researchActions';

describe('SandboxReplModal', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders modal with run compiler probe button without calling raw invoke', () => {
    render(
      <SandboxReplModal
        isOpen={true}
        onClose={() => {}}
        initialCode="pub fn probe() {}"
      />
    );
    expect(screen.getByText('Sandbox REPL Probe')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /run compiler probe/i })).toBeInTheDocument();
  });

  it('does not render when isOpen is false', () => {
    const { container } = render(
      <SandboxReplModal
        isOpen={false}
        onClose={() => {}}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it('allows language selection and runs probe with output display', async () => {
    const probeSpy = vi.spyOn(researchActions, 'executeSandboxProbe').mockResolvedValue({
      passed: true,
      stdout: 'Compilation successful',
      stderr: '',
    });

    render(
      <SandboxReplModal
        isOpen={true}
        onClose={() => {}}
        initialCode="print('hello')"
        initialLanguage="python"
      />
    );

    const select = screen.getByRole('combobox', { name: /language/i });
    expect(select).toHaveValue('python');

    const runBtn = screen.getByRole('button', { name: /run compiler probe/i });
    fireEvent.click(runBtn);

    await waitFor(() => {
      expect(probeSpy).toHaveBeenCalledWith("print('hello')", 'python');
      expect(screen.getByText(/Compilation successful/i)).toBeInTheDocument();
      expect(screen.getByText(/passed/i)).toBeInTheDocument();
    });
  });
});
