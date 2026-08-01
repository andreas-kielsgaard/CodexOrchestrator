import {
  validateEpicProductDecisionSnapshot,
  type EpicProductDecisionSnapshot,
  type EpicProductDecisionSource,
} from '../../application/productDecisions';

const recordedEpicId = 'epic-codex-runner-workspace';

const recordedSnapshot: EpicProductDecisionSnapshot = validateEpicProductDecisionSnapshot({
  epicId: recordedEpicId,
  recordedAt: '2026-08-01T12:00:00.000Z',
  evidence: [
    {
      evidenceId: 'evidence-human-layout-review',
      originReference: {
        kind: 'human_interaction',
        opaqueId: 'layout-review-outcome-2026-07-29',
      },
      label: 'Human layout review: preserve pane positions while reading',
      occurredAt: '2026-07-29T09:15:00.000Z',
    },
    {
      evidenceId: 'evidence-work-unit-detail-approval',
      originReference: {
        kind: 'work_unit_approved',
        opaqueId: 'epic-detail-containment-approval',
      },
      label: 'Epic detail containment Work Unit approval',
      occurredAt: '2026-07-29T11:45:00.000Z',
    },
    {
      evidenceId: 'evidence-sprint-detail-completion',
      originReference: {
        kind: 'sprint_completed',
        opaqueId: 'recorded-epic-detail-review-completion',
      },
      label: 'Recorded Epic detail review Sprint completion',
      occurredAt: '2026-07-29T12:00:00.000Z',
    },
  ],
  decisions: [
    {
      decisionId: 'decision-stable-workspace',
      title: 'Stable workspace',
      statement:
        'Keep the outer workspace fixed; only a contained region with more content than it can display may scroll.',
      intent: 'Protect spatial orientation while people move between related panes and details.',
      evidenceIds: ['evidence-human-layout-review', 'evidence-work-unit-detail-approval'],
      lineage: { kind: 'introduced', supersedesDecisionIds: [] },
    },
    {
      decisionId: 'decision-progressive-detail',
      parentDecisionId: 'decision-stable-workspace',
      title: 'Progressive detail',
      statement:
        'Keep the primary flow calm and reveal supporting evidence within its local context.',
      intent: 'Let people understand outcomes before exposing implementation machinery.',
      evidenceIds: ['evidence-sprint-detail-completion'],
      lineage: { kind: 'introduced', supersedesDecisionIds: [] },
    },
  ],
  candidates: [
    {
      candidateId: 'candidate-page-scroll',
      title: 'Let the full Epic page scroll',
      proposedStatement:
        'Allow the complete Epic workspace to scroll when its combined content exceeds the viewport.',
      rationale: 'A later interaction could be read as preferring one continuous document.',
      relation: 'refine',
      targetDecisionIds: ['decision-stable-workspace'],
      evidenceIds: ['evidence-sprint-detail-completion'],
    },
  ],
  conflicts: [
    {
      conflictId: 'conflict-page-scroll',
      candidateId: 'candidate-page-scroll',
      conflictsWithDecisionIds: ['decision-stable-workspace'],
      explanation:
        'This reverses the current policy that the workspace itself stays fixed. Human judgment is required.',
      status: 'pending_human_review',
    },
  ],
  complianceReviewRequests: [
    {
      requestId: 'review-request-contained-scroll',
      triggeredByDecisionId: 'decision-stable-workspace',
      reason:
        'A later manual review should check existing Epic detail surfaces against the recorded policy.',
      status: 'requested',
    },
  ],
});

/** Recorded development adapter. It performs no extraction, reconciliation, persistence, or audit. */
export const recordedEpicProductDecisionSource: EpicProductDecisionSource = {
  async loadEpicProductDecisions(epicId) {
    if (epicId !== recordedEpicId)
      return { kind: 'unavailable', reason: 'No recorded product decisions exist for this Epic.' };
    return { kind: 'available', snapshot: recordedSnapshot };
  },
};
