import { describe, expect, it } from 'vitest';
import { recordedEpicProductDecisionSource } from './recordedEpicProductDecisionSource';

describe('recorded Epic product decision source', () => {
  it('exposes deterministic development data only for the recorded Epic', async () => {
    const available = await recordedEpicProductDecisionSource.loadEpicProductDecisions(
      'epic-codex-runner-workspace',
    );
    const unavailable = await recordedEpicProductDecisionSource.loadEpicProductDecisions('other');

    expect(available.kind).toBe('available');
    expect(unavailable.kind).toBe('unavailable');
  });
});
