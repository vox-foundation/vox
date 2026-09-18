// @vitest-environment jsdom
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { ResearchEngineDrawer } from './ResearchEngineDrawer';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'get_research_engine_status') {
      return {
        active_lane: 'fast',
        fast_timeout_ms: 2500,
        deep_timeout_ms: 15000,
        providers: [
          { id: 'wikipedia', name: 'Wikipedia', is_keyless: true, is_enabled: true, has_key: false },
          { id: 'tavily', name: 'Tavily', is_keyless: false, is_enabled: true, has_key: false },
        ],
        free_key_offers: [
          {
            provider_id: 'tavily',
            name: 'Tavily Search',
            signup_url: 'https://app.tavily.com/sign-up',
            free_tier_description: '1,000 queries/month free web search for AI agents and LLMs.',
            quota_summary: '1,000 searches/mo',
            requires_credit_card: false,
            secret_id: 'tavily_api_key',
          },
        ],
      };
    }
    return null;
  }),
}));

describe('ResearchEngineDrawer', () => {
  it('renders Zero-Key Guarantee and free signup links', () => {
    render(<ResearchEngineDrawer isOpen={true} onClose={vi.fn()} />);
    expect(screen.getByText(/Zero-Key Guarantee/i)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Claim Free Key/i })).toBeInTheDocument();
  });

  it('stops propagation on Escape keypress', () => {
    const onClose = vi.fn();
    render(<ResearchEngineDrawer isOpen={true} onClose={onClose} />);
    const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
    const stopSpy = vi.spyOn(event, 'stopPropagation');
    document.dispatchEvent(event);
    expect(stopSpy).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it('traps focus circularly: Tab on last element cycles to first, Shift+Tab on first cycles to last', async () => {
    const { container } = render(<ResearchEngineDrawer isOpen={true} onClose={vi.fn()} />);
    const focusable = Array.from(
      container.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      )
    );
    expect(focusable.length).toBeGreaterThan(1);
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    // Case 1: Shift+Tab on first element wraps to last
    first.focus();
    expect(document.activeElement).toBe(first);
    const shiftTabEvent = new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true });
    const shiftPreventSpy = vi.spyOn(shiftTabEvent, 'preventDefault');
    document.dispatchEvent(shiftTabEvent);
    expect(shiftPreventSpy).toHaveBeenCalled();
    expect(document.activeElement).toBe(last);

    // Case 2: Tab on last element wraps to first
    last.focus();
    expect(document.activeElement).toBe(last);
    const tabEvent = new KeyboardEvent('keydown', { key: 'Tab', shiftKey: false, bubbles: true });
    const tabPreventSpy = vi.spyOn(tabEvent, 'preventDefault');
    document.dispatchEvent(tabEvent);
    expect(tabPreventSpy).toHaveBeenCalled();
    expect(document.activeElement).toBe(first);

    // Case 3: Tab when activeElement is outside drawer pulls focus to first
    const outsideBtn = document.createElement('button');
    document.body.appendChild(outsideBtn);
    outsideBtn.focus();
    expect(document.activeElement).toBe(outsideBtn);
    const outsideTabEvent = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true });
    const outsidePreventSpy = vi.spyOn(outsideTabEvent, 'preventDefault');
    document.dispatchEvent(outsideTabEvent);
    expect(outsidePreventSpy).toHaveBeenCalled();
    expect(document.activeElement).toBe(first);
    outsideBtn.remove();
  });

  it('provides accessible labels for all interactive inputs', async () => {
    render(<ResearchEngineDrawer isOpen={true} onClose={vi.fn()} />);
    expect(screen.getByLabelText(/Fast Lane Timeout/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Deep Lane Timeout/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/API key for Tavily/i)).toBeInTheDocument();
    expect(await screen.findByLabelText(/Enable Wikipedia/i)).toBeInTheDocument();
  });
});
