import { describe, expect, it } from 'vitest';
import { applySet, emptyLiveState, parseKnobPairs, snapshotCatalog } from './axisDrive';
import type { PickerModel, ProviderStatus } from './modelPicker';

describe('axisDrive', () => {
  it('applies execution and model knobs', () => {
    const next = applySet(emptyLiveState(), {
      model_override: 'mens/e2e-smoke',
      execution: 'sync',
    });
    expect(next.plane).toBe('live');
    expect(next.knobs.model_override).toBe('mens/e2e-smoke');
    expect(next.knobs.execution).toBe('sync');
    expect(next.pin).toBe('mens/e2e-smoke');
  });

  it('rejects unknown keys', () => {
    expect(() => applySet(emptyLiveState(), { nope: true } as never)).toThrow(/unknown_key/);
  });

  it('marks local models unselectable when probe is down', () => {
    const models: PickerModel[] = [
      { id: 'mens/e2e-smoke-metal', label: 'metal', provider: 'VoxLocal', providerType: 'local' },
    ];
    const statuses: ProviderStatus[] = [
      {
        provider: 'VoxLocal',
        key_present: false,
        is_local: true,
        local_reachable: false,
        local_models: [],
      },
    ];
    const rows = snapshotCatalog(models, statuses);
    expect(rows[0]?.selectable).toBe(false);
    expect(rows[0]?.reason).toMatch(/reachable|local/i);
  });

  it('fail-closes when provider statuses are missing', () => {
    const models: PickerModel[] = [
      { id: 'openai/gpt-4o', label: 'gpt', provider: 'openai', providerType: 'cloud' },
    ];
    const rows = snapshotCatalog(models, []);
    expect(rows[0]?.selectable).toBe(false);
    expect(rows[0]?.reason).toBe('status_unavailable');
  });

  it('parseKnobPairs splits key=value', () => {
    const set = parseKnobPairs([
      'model_override=mens/e2e-smoke',
      'execution=sync',
      'grounding_check_enabled=true',
    ]);
    expect(set.model_override).toBe('mens/e2e-smoke');
    expect(set.execution).toBe('sync');
    expect(set.grounding_check_enabled).toBe(true);
  });
});
