// @vitest-environment jsdom
import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { InspectorDrawer } from './InspectorDrawer';
import { RESEARCH_STAGES } from '../lib/pipeline';
import type { UsePipelineStepperReturn } from './usePipelineStepper';

describe('InspectorDrawer', () => {
  it('renders nothing when open is false', () => {
    const { container } = render(<InspectorDrawer open={false} onClose={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders when open is true with data-testid="inspector-drawer"', () => {
    render(<InspectorDrawer open={true} onClose={vi.fn()} />);
    const drawer = screen.getByTestId('inspector-drawer');
    expect(drawer).toBeInTheDocument();
    expect(screen.getByText(/Pipeline Inspector & Debugger/i)).toBeInTheDocument();
  });

  it('displays the 8 canonical stages', () => {
    render(<InspectorDrawer open={true} onClose={vi.fn()} />);

    for (const stage of RESEARCH_STAGES) {
      const elements = screen.getAllByText(stage);
      expect(elements.length).toBeGreaterThan(0);
      expect(elements[0]).toBeInTheDocument();
    }
  });

  it('closes when close button is clicked', () => {
    const onClose = vi.fn();
    render(<InspectorDrawer open={true} onClose={onClose} />);

    const closeBtn = screen.getByRole('button', { name: /close inspector/i });
    fireEvent.click(closeBtn);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('closes when Escape key is pressed', () => {
    const onClose = vi.fn();
    render(<InspectorDrawer open={true} onClose={onClose} />);

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('controls render correctly based on isPlaying and invoke actions', () => {
    const stepNext = vi.fn();
    const pause = vi.fn();
    const resume = vi.fn();
    const reset = vi.fn();

    // Case 1: isPlaying = false (autoPlay: false, isPaused: true)
    const mockStepperPaused: UsePipelineStepperReturn = {
      steps: RESEARCH_STAGES.map((s, i) => ({ id: s, label: s, status: i === 0 ? 'active' : 'pending' })),
      currentStageIndex: 0,
      currentStep: { id: 'queued', label: 'queued', status: 'active' },
      isPaused: true,
      autoPlay: false,
      violations: [],
      stepNext,
      stepPrev: vi.fn(),
      pause,
      resume,
      reset,
      overridePayload: vi.fn(),
      setStepStatus: vi.fn(),
      auditInvariants: vi.fn().mockReturnValue([]),
    };

    const { rerender } = render(
      <InspectorDrawer open={true} onClose={vi.fn()} stepper={mockStepperPaused} />
    );

    // Play button should be shown when not playing
    const playBtn = screen.getByRole('button', { name: /^play$/i });
    expect(playBtn).toBeInTheDocument();
    fireEvent.click(playBtn);
    expect(resume).toHaveBeenCalledTimes(1);

    // Step Next button
    const stepNextBtn = screen.getByRole('button', { name: /step next/i });
    fireEvent.click(stepNextBtn);
    expect(stepNext).toHaveBeenCalledTimes(1);

    // Reset button
    const resetBtn = screen.getByRole('button', { name: /reset/i });
    fireEvent.click(resetBtn);
    expect(reset).toHaveBeenCalledTimes(1);

    // Case 2: isPlaying = true (autoPlay: true, isPaused: false)
    const mockStepperPlaying: UsePipelineStepperReturn = {
      ...mockStepperPaused,
      isPaused: false,
      autoPlay: true,
    };

    rerender(<InspectorDrawer open={true} onClose={vi.fn()} stepper={mockStepperPlaying} />);

    // Pause button should be shown when playing
    const pauseBtn = screen.getByRole('button', { name: /^pause$/i });
    expect(pauseBtn).toBeInTheDocument();
    fireEvent.click(pauseBtn);
    expect(pause).toHaveBeenCalledTimes(1);
  });

  it('clears stage selection when reset is clicked', () => {
    const reset = vi.fn();
    const mockStepper: UsePipelineStepperReturn = {
      steps: RESEARCH_STAGES.map((s, i) => ({ id: s, label: s, status: i === 0 ? 'active' : 'pending' })),
      currentStageIndex: 0,
      currentStep: { id: 'queued', label: 'queued', status: 'active' },
      isPaused: true,
      autoPlay: false,
      violations: [],
      stepNext: vi.fn(),
      stepPrev: vi.fn(),
      pause: vi.fn(),
      resume: vi.fn(),
      reset,
      overridePayload: vi.fn(),
      setStepStatus: vi.fn(),
      auditInvariants: vi.fn().mockReturnValue([]),
    };

    render(<InspectorDrawer open={true} onClose={vi.fn()} stepper={mockStepper} />);

    // Select planning stage
    const planningBtn = screen.getByRole('button', { name: /planning/i });
    fireEvent.click(planningBtn);
    expect(screen.getByText('Stage: planning')).toBeInTheDocument();

    // Click Reset
    const resetBtn = screen.getByRole('button', { name: /reset/i });
    fireEvent.click(resetBtn);
    expect(reset).toHaveBeenCalledTimes(1);

    // Selection should be cleared back to active stage (queued)
    expect(screen.getByText('Stage: queued')).toBeInTheDocument();
  });

  it('renders formatted payloads for inspected step', () => {
    const mockStepper: UsePipelineStepperReturn = {
      steps: [
        {
          id: 'retrieving',
          label: 'retrieving',
          status: 'completed',
          inputPayload: { query: 'test query' },
          outputPayload: { sources: ['https://example.com/1'] },
        },
      ],
      currentStageIndex: 0,
      currentStep: {
        id: 'retrieving',
        label: 'retrieving',
        status: 'completed',
        inputPayload: { query: 'test query' },
        outputPayload: { sources: ['https://example.com/1'] },
      },
      isPaused: false,
      autoPlay: false,
      violations: [],
      stepNext: vi.fn(),
      stepPrev: vi.fn(),
      pause: vi.fn(),
      resume: vi.fn(),
      reset: vi.fn(),
      overridePayload: vi.fn(),
      setStepStatus: vi.fn(),
      auditInvariants: vi.fn().mockReturnValue([]),
    };

    render(<InspectorDrawer open={true} onClose={vi.fn()} stepper={mockStepper} />);

    expect(screen.getByText(/"query": "test query"/i)).toBeInTheDocument();
    expect(screen.getByText(/"https:\/\/example.com\/1"/i)).toBeInTheDocument();
  });

  it('switches to Violations tab and renders violations', () => {
    const mockStepper: UsePipelineStepperReturn = {
      steps: RESEARCH_STAGES.map((s, i) => ({ id: s, label: s, status: i === 0 ? 'active' : 'pending' })),
      currentStageIndex: 0,
      currentStep: { id: 'queued', label: 'queued', status: 'active' },
      isPaused: false,
      autoPlay: false,
      violations: [
        {
          kind: 'fake_success',
          severity: 'critical',
          message: 'Zero sources without empty-state notice',
          location: 'retrieving.sources',
        },
      ],
      stepNext: vi.fn(),
      stepPrev: vi.fn(),
      pause: vi.fn(),
      resume: vi.fn(),
      reset: vi.fn(),
      overridePayload: vi.fn(),
      setStepStatus: vi.fn(),
      auditInvariants: vi.fn().mockReturnValue([]),
    };

    render(<InspectorDrawer open={true} onClose={vi.fn()} stepper={mockStepper} />);

    const violationsTab = screen.getByRole('button', { name: /violations/i });
    fireEvent.click(violationsTab);

    expect(screen.getByText('fake_success')).toBeInTheDocument();
    expect(screen.getByText('Zero sources without empty-state notice')).toBeInTheDocument();
    expect(screen.getByText('critical')).toBeInTheDocument();
  });
});
