import { describe, expect, it } from 'vitest';
import {
  resolveProductDecisionEvidenceNavigation,
  validateEpicProductDecisionSnapshot,
  type EpicProductDecisionSnapshot,
} from './epicProductDecisions';

const snapshot: EpicProductDecisionSnapshot = {
  epicId: 'epic-1',
  recordedAt: '2026-08-06T09:00:00.000Z',
  evidence: [
    {
      evidenceId: 'evidence-1',
      originReference: { kind: 'agent_session_completed', opaqueId: 'opaque-session-record' },
      conversationCitation: {
        kind: 'agent_session_passage',
        sessionId: 'session-1',
        invocationId: 'invocation-1',
        passage: { kind: 'final_response', runtimeEventId: 'event-1' },
      },
      label: 'Recorded supporting passage',
      occurredAt: '2026-08-06T09:00:00.000Z',
    },
    {
      evidenceId: 'evidence-2',
      originReference: { kind: 'human_interaction', opaqueId: 'opaque-human-record' },
      label: 'Recorded human context',
      occurredAt: '2026-08-06T09:00:00.000Z',
    },
  ],
  decisions: [
    {
      decisionId: 'decision-1',
      title: 'Recorded decision',
      statement: 'Keep the supporting context separate from policy.',
      intent: 'Prevent evidence from becoming a causal authority claim.',
      evidenceIds: ['evidence-1', 'evidence-2'],
      lineage: { kind: 'introduced', supersedesDecisionIds: [] },
    },
  ],
  candidates: [],
  conflicts: [],
  complianceReviewRequests: [],
};

describe('Product Decision evidence navigation', () => {
  it('resolves only an exact typed, current Agent Session passage', () => {
    const validated = validateEpicProductDecisionSnapshot(snapshot);
    const evidence = validated.evidence[0]!;
    expect(
      resolveProductDecisionEvidenceNavigation(validated, {
        epicId: validated.epicId,
        evidenceId: evidence.evidenceId,
        originReference: evidence.originReference,
        conversationCitation: evidence.conversationCitation,
      }),
    ).toEqual({
      kind: 'available',
      destination: {
        kind: 'agent_session_passage',
        sessionId: 'session-1',
        invocationId: 'invocation-1',
        passage: { kind: 'final_response', runtimeEventId: 'event-1' },
      },
    });
  });

  it('fails closed for malformed, foreign, stale, mismatched, and unsupported references', () => {
    const evidence = snapshot.evidence[0]!;
    const request = {
      epicId: snapshot.epicId,
      evidenceId: evidence.evidenceId,
      originReference: evidence.originReference,
      conversationCitation: evidence.conversationCitation!,
    };
    for (const value of [
      null,
      { ...request, epicId: 'foreign-epic' },
      { ...request, evidenceId: 'stale-evidence' },
      {
        ...request,
        conversationCitation: {
          ...request.conversationCitation,
          invocationId: 'foreign-invocation',
        },
      },
      {
        ...request,
        originReference: { ...request.originReference, opaqueId: 'resembling-but-foreign' },
      },
      {
        ...request,
        originReference: { ...request.originReference, historical: true },
      },
      {
        ...request,
        conversationCitation: {
          ...request.conversationCitation,
          passage: {
            ...request.conversationCitation.passage,
            unsupported: 'opaque-id resemblance',
          },
        },
      },
      {
        epicId: snapshot.epicId,
        evidenceId: 'evidence-2',
        originReference: snapshot.evidence[1]!.originReference,
        conversationCitation: request.conversationCitation,
      },
    ])
      expect(resolveProductDecisionEvidenceNavigation(snapshot, value)).toEqual({
        kind: 'unavailable',
      });
  });
});
