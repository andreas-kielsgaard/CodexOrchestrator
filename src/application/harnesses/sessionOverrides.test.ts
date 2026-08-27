import { createHarnessId, createHarnessVersionNumber, createHarnessVersionRef } from './references';
import { InMemorySessionHarnessOverrideDraftCache } from './sessionOverrides';
import { exampleHarnessConfiguration } from './testFixtures';

describe('InMemorySessionHarnessOverrideDraftCache', () => {
  it('keeps one dirty draft per Session', () => {
    const cache = new InMemorySessionHarnessOverrideDraftCache();
    const reference = createHarnessVersionRef(
      createHarnessId('harness-epic-plan-builder'),
      createHarnessVersionNumber(4),
    );

    cache.save('session-one', reference, exampleHarnessConfiguration());

    expect(cache.get('session-one')).toEqual({
      baseHarnessRef: reference,
      configuration: exampleHarnessConfiguration(),
      isDirty: true,
    });
    expect(cache.get('session-two')).toBeNull();
  });

  it('defensively copies values at both cache boundaries', () => {
    const cache = new InMemorySessionHarnessOverrideDraftCache();
    const configuration = exampleHarnessConfiguration();
    const saved = cache.save(
      'session-one',
      createHarnessVersionRef(
        createHarnessId('harness-epic-plan-builder'),
        createHarnessVersionNumber(4),
      ),
      configuration,
    );

    (configuration.tools.items as { name: string; policy: 'available' }[])[0].name =
      'mutated-input';
    (saved.configuration.tools.items as { name: string; policy: 'available' }[])[0].name =
      'mutated-output';

    expect(cache.get('session-one')?.configuration.tools.items[0].name).toBe('submit_epic_plan');
  });

  it('discards one Session draft or clears the application cache', () => {
    const cache = new InMemorySessionHarnessOverrideDraftCache();
    const reference = createHarnessVersionRef(
      createHarnessId('harness-epic-plan-builder'),
      createHarnessVersionNumber(4),
    );
    cache.save('session-one', reference, exampleHarnessConfiguration());
    cache.save('session-two', reference, exampleHarnessConfiguration());

    expect(cache.discard('session-one')).toBe(true);
    expect(cache.discard('session-one')).toBe(false);
    expect(cache.get('session-two')).not.toBeNull();

    cache.clear();
    expect(cache.get('session-two')).toBeNull();
  });

  it('rejects ambiguous Session keys', () => {
    const cache = new InMemorySessionHarnessOverrideDraftCache();

    expect(() => cache.get('')).toThrow(/non-empty/);
    expect(() => cache.discard(' session-one')).toThrow(/whitespace/);
  });
});
