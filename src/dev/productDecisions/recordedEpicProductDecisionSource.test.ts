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
      {
        kind: 'agent_session_completed',
        opaqueId: 'recorded-epic-runner-manual-continuation-ready',
      },
      { kind: 'work_unit_approved', opaqueId: 'epic-detail-containment-approval' },
      { kind: 'sprint_completed', opaqueId: 'recorded-epic-detail-review-completion' },
    ]);
    expect(available.snapshot.decisions[0]).toMatchObject({
      title: 'Contained Epic detail',
      statement:
        'In Epic detail, keep the menu and outer frame fixed; scroll only a contained region whose content exceeds its bounds.',
    });
    expect(available.snapshot.evidence[1].conversationCitation).toMatchObject({
      kind: 'agent_session_passage',
      sessionId: 'recorded-epic-runner-manual-continuation-ready',
      passage: { kind: 'final_response' },
    });
    expect(unavailable.kind).toBe('unavailable');
  });
});
