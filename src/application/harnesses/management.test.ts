import type {
  HarnessDraft,
  HarnessVersionReplacement,
  PublishedHarnessVersion,
} from './management';
import { createHarnessId, createHarnessVersionNumber, createHarnessVersionRef } from './references';
import { exampleHarnessConfiguration } from './testFixtures';

describe('Harness management contracts', () => {
  it('uses exact Harness version references without product revision identifiers', () => {
    const harnessId = createHarnessId('harness-epic-plan-builder');
    const reference = createHarnessVersionRef(harnessId, createHarnessVersionNumber(2));
    const version: PublishedHarnessVersion = {
      reference,
      scope: { kind: 'reusable' },
      configuration: exampleHarnessConfiguration(),
      createdAt: '2026-08-27T08:00:00Z',
    };
    const draft: HarnessDraft = {
      harnessId,
      basedOn: reference,
      configuration: exampleHarnessConfiguration(),
      savedAt: '2026-08-27T08:05:00Z',
    };

    expect(version.reference).toEqual({ harnessId, version: 2 });
    expect(draft.basedOn).toEqual(reference);
    expect(version).not.toHaveProperty('revisionId');
    expect(draft).not.toHaveProperty('draftRevision');
  });

  it('represents replacement as an exact relationship within one Harness', () => {
    const harnessId = createHarnessId('harness-epic-plan-builder');
    const replacement: HarnessVersionReplacement = {
      source: createHarnessVersionRef(harnessId, createHarnessVersionNumber(1)),
      target: createHarnessVersionRef(harnessId, createHarnessVersionNumber(2)),
    };

    expect(replacement).toEqual({
      source: { harnessId, version: 1 },
      target: { harnessId, version: 2 },
    });
  });
});
