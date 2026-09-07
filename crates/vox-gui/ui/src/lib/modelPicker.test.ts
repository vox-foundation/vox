import { describe, expect, it } from 'vitest';
import {
  filterPickerModels,
  isModelSelectable,
  isRoutingTierId,
  normalizeModelCard,
  providerKey,
  shortModelLabel,
} from './modelPicker';

const openrouterUp = {
  provider: 'OpenRouter',
  key_present: true,
  is_local: false,
  local_reachable: null,
};
const openrouterDown = { ...openrouterUp, key_present: false };
const openaiDown = {
  provider: 'OpenAI',
  key_present: false,
  is_local: false,
  local_reachable: null,
};
const voxLocal = {
  provider: 'VoxLocal',
  key_present: true,
  is_local: true,
  local_reachable: null,
};
const ollamaDown = {
  provider: 'Ollama',
  key_present: true,
  is_local: true,
  local_reachable: false,
};

describe('normalizeModelCard', () => {
  it('reads id (list_model_cards), not the retired model_id-only shape', () => {
    expect(normalizeModelCard({ id: 'openrouter/anthropic/claude', provider: 'anthropic' })).toEqual({
      id: 'openrouter/anthropic/claude',
      label: 'openrouter/anthropic/claude',
      provider: 'anthropic',
      providerType: '',
    });
  });

  it('falls back to model_id / display_name for older mocks', () => {
    expect(
      normalizeModelCard({
        model_id: 'mens/e2e-smoke-metal',
        display_name: 'MENS Metal',
        provider: 'populi_local',
      }),
    ).toMatchObject({
      id: 'mens/e2e-smoke-metal',
      label: 'MENS Metal',
      provider: 'populi_local',
    });
  });

  it('drops cards with no id', () => {
    expect(normalizeModelCard({ provider: 'openrouter' })).toBeNull();
  });
});

describe('providerKey', () => {
  it('aliases populi_local / mens onto voxlocal', () => {
    expect(providerKey('populi_local')).toBe('voxlocal');
    expect(providerKey('mens')).toBe('voxlocal');
    expect(providerKey('VoxLocal')).toBe('voxlocal');
  });
});

describe('isModelSelectable', () => {
  it('shows OpenRouter catalog prefixes when the OpenRouter key is present', () => {
    const aion = normalizeModelCard({
      id: 'aion-labs/aion-1.0',
      provider: 'aion-labs',
      provider_type: 'OpenRouter',
    })!;
    expect(isModelSelectable(aion, [openrouterUp])).toBe(true);
    expect(isModelSelectable(aion, [openrouterDown])).toBe(false);
  });

  it('treats unmatched org prefixes as OpenRouter when that key is present', () => {
    const anthropicViaOr = normalizeModelCard({
      id: 'anthropic/claude-sonnet-4',
      provider: 'anthropic',
    })!;
    expect(isModelSelectable(anthropicViaOr, [openrouterUp])).toBe(true);
    expect(isModelSelectable(anthropicViaOr, [openrouterDown])).toBe(false);
  });

  it('hides a direct provider that has no key even if OpenRouter is up', () => {
    const openaiDirect = normalizeModelCard({
      id: 'openai/gpt-5.2-mini',
      provider: 'openai',
      provider_type: 'OpenAI',
    })!;
    expect(isModelSelectable(openaiDirect, [openrouterUp, openaiDown])).toBe(false);
  });

  it('keeps MENS / populi_local cards (VoxLocal)', () => {
    const mens = normalizeModelCard({
      id: 'mens/e2e-smoke-metal',
      provider: 'populi_local',
      provider_type: 'VoxLocal',
    })!;
    expect(isModelSelectable(mens, [openrouterUp, voxLocal])).toBe(true);
    expect(isModelSelectable(mens, [openrouterUp])).toBe(true);
  });

  it('hides a local provider the health probe says is unreachable', () => {
    const ollama = normalizeModelCard({ id: 'ollama/llama3', provider: 'ollama' })!;
    expect(isModelSelectable(ollama, [ollamaDown])).toBe(false);
  });

  it('shows everything when statuses failed to load', () => {
    const any = normalizeModelCard({ id: 'x', provider: 'mystery' })!;
    expect(isModelSelectable(any, [])).toBe(true);
  });
});

describe('filterPickerModels', () => {
  it('narrows by id, label, or provider', () => {
    const models = [
      normalizeModelCard({ id: 'openrouter/anthropic/claude', provider: 'anthropic' })!,
      normalizeModelCard({ id: 'mens/e2e-smoke-metal', provider: 'populi_local' })!,
    ];
    expect(filterPickerModels(models, [openrouterUp], 'mens').map(m => m.id)).toEqual([
      'mens/e2e-smoke-metal',
    ]);
    expect(filterPickerModels(models, [openrouterUp], 'claude').map(m => m.id)).toEqual([
      'openrouter/anthropic/claude',
    ]);
  });
});

describe('shortModelLabel / routing tiers', () => {
  it('keeps the last two id segments', () => {
    expect(shortModelLabel('openrouter/anthropic/claude-sonnet-4')).toBe('anthropic/claude-sonnet-4');
    expect(shortModelLabel('mens/e2e-smoke-metal')).toBe('mens/e2e-smoke-metal');
  });

  it('recognizes routing-tier ids', () => {
    expect(isRoutingTierId('auto')).toBe(true);
    expect(isRoutingTierId('mens/e2e-smoke-metal')).toBe(false);
  });
});
