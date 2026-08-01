import { describe, expect, it } from 'vitest';
import {
  validateEpicProductDecisionSnapshot,
  type EpicProductDecisionSnapshot,
} from './epicProductDecisions';

const snapshot: EpicProductDecisionSnapshot = {
  epicId: 'epic-1',
  recordedAt: '2026-08-01T10:00:00.000Z',
  evidence: [
    {
      evidenceId: 'evidence-1',
      kind: 'human_interaction',
      label: 'Human design review',
      occurredAt: '2026-08-01T09:00:00.000Z',
    },
  ],
  decisions: [
    {
      decisionId: 'decision-1',
      title: 'Stable layout',
      statement: 'Keep the outer workspace fixed.',
      intent: 'Preserve spatial orientation.',
      evidenceIds: ['evidence-1'],
      lineage: { kind: 'introduced', supersedesDecisionIds: [] },
    },
  ],
  candidates: [
    {
      candidateId: 'candidate-1',
      title: 'Whole-page scrolling',
      proposedStatement: 'Allow the whole workspace to scroll.',
      rationale: 'A later interaction suggested a different layout behavior.',
      relation: 'refine',
      targetDecisionIds: ['decision-1'],
      evidenceIds: ['evidence-1'],
    },
  ],
  conflicts: [
    {
      conflictId: 'conflict-1',
      candidateId: 'candidate-1',
      conflictsWithDecisionIds: ['decision-1'],
      explanation: 'The candidate reverses the current containment stance.',
      status: 'pending_human_review',
    },
  ],
  complianceReviewRequests: [],
};

describe('Epic product decision snapshot', () => {
  it('keeps canonical decisions, candidates, conflicts, evidence, and review requests distinct', () => {
    expect(validateEpicProductDecisionSnapshot(snapshot)).toBe(snapshot);
  });

  it('rejects unsourced policy and decision-tree cycles', () => {
    expect(() =>
      validateEpicProductDecisionSnapshot({
        ...snapshot,
        decisions: [
          {
            ...snapshot.decisions[0],
            parentDecisionId: 'decision-1',
            evidenceIds: [],
          },
        ],
      }),
    ).toThrow(/reference|cycle/);
  });

  it('rejects a conflict that does not link a real candidate to current policy', () => {
    expect(() =>
      validateEpicProductDecisionSnapshot({
        ...snapshot,
        conflicts: [{ ...snapshot.conflicts[0], candidateId: 'missing' }],
      }),
    ).toThrow('Conflict references an unknown candidate');
  });
});
