import {
  assignAgentIdentity,
  harnessAgentNamePools,
  harnessVisualIdentities,
  productDefaultAgentNames,
  validateAgentNamePool,
  validateCuratedHarnessNamePools,
} from './agentIdentity';

describe('Agent identity assignment', () => {
  const base = {
    sessionId: 'session-42',
    harnessKey: 'epic_plan_builder',
    harnessRole: 'Epic Plan Builder',
    harnessRevision: 4,
    visualIdentity: harnessVisualIdentities.epic_plan_builder,
    permittedNames: harnessAgentNamePools.epic_plan_builder,
    assignedAt: '2026-07-28T08:00:00.000Z',
  } as const;

  it('provides 100 unique product names and valid curated role pools', () => {
    expect(productDefaultAgentNames).toHaveLength(100);
    expect(new Set(productDefaultAgentNames).size).toBe(100);
    expect(() => validateCuratedHarnessNamePools()).not.toThrow();
    expect(Object.values(harnessAgentNamePools).every((pool) => pool.length >= 10)).toBe(true);
    expect(new Set(Object.values(harnessVisualIdentities).map(({ token }) => token)).size).toBe(3);
    expect(new Set(Object.values(harnessVisualIdentities).map(({ accent }) => accent)).size).toBe(
      3,
    );
  });

  it('assigns deterministically from the permitted pool', () => {
    const first = assignAgentIdentity(base);
    const second = assignAgentIdentity(base);

    expect(second).toEqual(first);
    expect(harnessAgentNamePools.epic_plan_builder).toContain(first.name);
    expect(first).toMatchObject({
      harnessRole: 'Epic Plan Builder',
      appliedHarnessRevision: 4,
      assignment: { pool: 'harness_subset' },
    });
  });

  it('avoids names already assigned and adds a stable numeric fallback when a pool is exhausted', () => {
    const pool = ['Antoni Gaudi'];
    const unique = assignAgentIdentity({ ...base, permittedNames: pool, existingNames: [] });
    const fallback = assignAgentIdentity({
      ...base,
      permittedNames: pool,
      existingNames: [unique.name, `${unique.name} 2`],
    });

    expect(unique.name).toBe('Antoni Gaudi');
    expect(fallback.name).toBe('Antoni Gaudi 3');
  });

  it('rejects blank and duplicate names', () => {
    expect(() => validateAgentNamePool([])).toThrow(/must not be empty/i);
    expect(() => validateAgentNamePool(['Ada Lovelace', ' ada lovelace '])).toThrow(/unique/i);
    expect(() => validateAgentNamePool(['Grace Hopper', ' '])).toThrow(/blank/i);
  });
});
