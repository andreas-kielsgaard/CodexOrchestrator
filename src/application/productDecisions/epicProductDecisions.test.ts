import { describe, expect, it } from 'vitest';
import {
  validateEpicProductDecisionSnapshot,
  type EpicProductDecisionSnapshot,
  type ProductDecision,
  type ProductDecisionChangeCandidate,
  type ProductDecisionEvidence,
} from './epicProductDecisions';

const evidence: readonly ProductDecisionEvidence[] = [
  {
    evidenceId: 'evidence-1',
    originReference: { kind: 'human_interaction', opaqueId: 'review-outcome-1' },
    label: 'Human design review',
    occurredAt: '2026-08-01T09:00:00.000Z',
  },
];

const introducedDecision: ProductDecision = {
  decisionId: 'decision-1',
  title: 'Stable layout',
  statement: 'Keep the outer workspace fixed.',
  intent: 'Preserve spatial orientation.',
  evidenceIds: ['evidence-1'],
  lineage: { kind: 'introduced', supersedesDecisionIds: [] },
};

const refineCandidate: ProductDecisionChangeCandidate = {
  candidateId: 'candidate-1',
  title: 'Whole-page scrolling',
  proposedStatement: 'Allow the whole workspace to scroll.',
  rationale: 'A later interaction suggested a different layout behavior.',
  relation: 'refine',
  targetDecisionIds: ['decision-1'],
  evidenceIds: ['evidence-1'],
};

const snapshot: EpicProductDecisionSnapshot = {
  epicId: 'epic-1',
  recordedAt: '2026-08-01T10:00:00.000Z',
  evidence,
  decisions: [introducedDecision],
  candidates: [refineCandidate],
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

function validateWith(overrides: Partial<EpicProductDecisionSnapshot>) {
  return validateEpicProductDecisionSnapshot({ ...snapshot, ...overrides });
}

describe('Epic product decision snapshot', () => {
  it('keeps canonical decisions, candidates, conflicts, evidence, and review requests distinct', () => {
    expect(validateEpicProductDecisionSnapshot(snapshot)).toBe(snapshot);
  });

  it('accepts typed opaque origins for every eligible evidence kind', () => {
    const kinds = [
      'human_interaction',
      'agent_session_completed',
      'work_unit_approved',
      'sprint_completed',
      'epic_completed',
    ] as const;
    const completeEvidence = kinds.map((kind, index) => ({
      evidenceId: `evidence-${index}`,
      originReference: { kind, opaqueId: `origin-${index}` },
      label: `Eligible evidence ${index}`,
      occurredAt: '2026-08-01T09:00:00.000Z',
    }));

    expect(
      validateWith({
        evidence: completeEvidence,
        decisions: [
          {
            ...introducedDecision,
            evidenceIds: completeEvidence.map(({ evidenceId }) => evidenceId),
          },
        ],
        candidates: [],
        conflicts: [],
      }),
    ).toMatchObject({ evidence: completeEvidence });
  });

  it('rejects missing, ineligible, and malformed evidence origin references', () => {
    expect(() =>
      validateWith({
        evidence: [
          { ...evidence[0], originReference: undefined } as unknown as ProductDecisionEvidence,
        ],
      }),
    ).toThrow('Evidence origin reference kind is not eligible');
    expect(() =>
      validateWith({
        evidence: [
          {
            ...evidence[0],
            originReference: { kind: 'message_sent', opaqueId: 'message-1' },
          } as unknown as ProductDecisionEvidence,
        ],
      }),
    ).toThrow('Evidence origin reference kind is not eligible');
    expect(() =>
      validateWith({
        evidence: [
          { ...evidence[0], originReference: { ...evidence[0].originReference, opaqueId: ' ' } },
        ],
      }),
    ).toThrow('Evidence origin reference identity cannot be empty');
  });

  it('accepts relation-aware canonical lineage and change-candidate graphs', () => {
    const decisions: readonly ProductDecision[] = [
      introducedDecision,
      {
        ...introducedDecision,
        decisionId: 'decision-2',
        lineage: { kind: 'refined', supersedesDecisionIds: ['decision-1'] },
      },
      {
        ...introducedDecision,
        decisionId: 'decision-3',
        lineage: { kind: 'combined', supersedesDecisionIds: ['decision-1', 'decision-2'] },
      },
    ];
    const candidates: readonly ProductDecisionChangeCandidate[] = [
      {
        ...refineCandidate,
        candidateId: 'candidate-introduce',
        relation: 'introduce',
        targetDecisionIds: [],
      },
      refineCandidate,
      {
        ...refineCandidate,
        candidateId: 'candidate-combine',
        relation: 'combine',
        targetDecisionIds: ['decision-1', 'decision-2'],
      },
    ];

    expect(validateWith({ decisions, candidates, conflicts: [] })).toMatchObject({
      decisions,
      candidates,
    });
  });

  it('rejects invalid canonical lineage cardinality and references', () => {
    const decision = (lineage: ProductDecision['lineage']) => [
      introducedDecision,
      { ...introducedDecision, decisionId: 'decision-2', lineage },
    ];

    expect(() =>
      validateWith({
        decisions: decision({ kind: 'introduced', supersedesDecisionIds: ['decision-1'] }),
      }),
    ).toThrow('introduction cannot contain references');
    expect(() =>
      validateWith({ decisions: decision({ kind: 'refined', supersedesDecisionIds: [] }) }),
    ).toThrow('refinement must identify exactly one reference');
    expect(() =>
      validateWith({
        decisions: decision({ kind: 'combined', supersedesDecisionIds: ['decision-1'] }),
      }),
    ).toThrow('combination must identify at least two references');
    expect(() =>
      validateWith({
        decisions: decision({
          kind: 'combined',
          supersedesDecisionIds: ['decision-1', 'decision-1'],
        }),
      }),
    ).toThrow('references must be distinct');
    expect(() =>
      validateWith({
        decisions: decision({ kind: 'refined', supersedesDecisionIds: ['decision-2'] }),
      }),
    ).toThrow('cannot reference its own identity');
    expect(() =>
      validateWith({
        decisions: decision({ kind: 'refined', supersedesDecisionIds: ['missing'] }),
      }),
    ).toThrow('references an unknown decision');
    expect(() =>
      validateWith({ decisions: decision({ kind: 'refined', supersedesDecisionIds: [' '] }) }),
    ).toThrow('reference cannot be empty');
  });

  it('rejects cyclic canonical lineage', () => {
    expect(() =>
      validateWith({
        decisions: [
          {
            ...introducedDecision,
            lineage: { kind: 'refined', supersedesDecisionIds: ['decision-2'] },
          },
          {
            ...introducedDecision,
            decisionId: 'decision-2',
            lineage: { kind: 'refined', supersedesDecisionIds: ['decision-1'] },
          },
        ],
      }),
    ).toThrow('Decision lineage cannot contain a cycle');
  });

  it('rejects invalid change-candidate cardinality and references', () => {
    const candidate = (
      relation: ProductDecisionChangeCandidate['relation'],
      targetDecisionIds: readonly string[],
      candidateId = 'candidate-1',
    ) => ({ ...refineCandidate, candidateId, relation, targetDecisionIds });
    const selfTargetDecision = { ...introducedDecision, decisionId: 'candidate-self' };

    expect(() => validateWith({ candidates: [candidate('introduce', ['decision-1'])] })).toThrow(
      'introduction cannot contain references',
    );
    expect(() => validateWith({ candidates: [candidate('refine', [])] })).toThrow(
      'refinement must identify exactly one reference',
    );
    expect(() => validateWith({ candidates: [candidate('combine', ['decision-1'])] })).toThrow(
      'combination must identify at least two references',
    );
    expect(() =>
      validateWith({ candidates: [candidate('combine', ['decision-1', 'decision-1'])] }),
    ).toThrow('references must be distinct');
    expect(() => validateWith({ candidates: [candidate('refine', ['missing'])] })).toThrow(
      'references an unknown decision',
    );
    expect(() => validateWith({ candidates: [candidate('refine', [' '])] })).toThrow(
      'reference cannot be empty',
    );
    expect(() =>
      validateWith({
        decisions: [selfTargetDecision],
        candidates: [candidate('refine', ['candidate-self'], 'candidate-self')],
        conflicts: [],
      }),
    ).toThrow('cannot reference its own identity');
  });

  it('rejects unsourced policy, decision-tree cycles, and invalid conflicts', () => {
    expect(() => validateWith({ decisions: [{ ...introducedDecision, evidenceIds: [] }] })).toThrow(
      'Decision evidence must retain at least one reference',
    );
    expect(() =>
      validateWith({ decisions: [{ ...introducedDecision, parentDecisionId: 'decision-1' }] }),
    ).toThrow('Decision tree cannot contain a cycle');
    expect(() =>
      validateWith({ conflicts: [{ ...snapshot.conflicts[0], candidateId: 'missing' }] }),
    ).toThrow('Conflict references an unknown candidate');
  });
});
