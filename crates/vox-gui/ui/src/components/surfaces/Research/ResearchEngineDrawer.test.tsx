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
});
