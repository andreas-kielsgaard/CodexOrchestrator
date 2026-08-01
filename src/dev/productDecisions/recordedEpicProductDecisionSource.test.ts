import { describe, expect, it } from 'vitest';
import { recordedEpicProductDecisionSource } from './recordedEpicProductDecisionSource';

describe('recorded Epic product decision source', () => {
  it('exposes deterministic development data only for the recorded Epic', async () => {
    const available = await recordedEpicProductDecisionSource.loadEpicProductDecisions(
      'epic-codex-runner-workspace',
    );
    const unavailable = await recordedEpicProductDecisionSource.loadEpicProductDecisions('other');

    expect(available.kind).toBe('available');
    if (available.kind !== 'available') throw new Error('Expected recorded product decisions.');
    expect(available.snapshot.evidence.map(({ originReference }) => originReference)).toEqual([
      { kind: 'human_interaction', opaqueId: 'layout-review-outcome-2026-07-29' },
      { kind: 'work_unit_approved', opaqueId: 'epic-detail-containment-approval' },
      { kind: 'sprint_completed', opaqueId: 'recorded-epic-detail-review-completion' },
    ]);
    expect(unavailable.kind).toBe('unavailable');
  });
});
