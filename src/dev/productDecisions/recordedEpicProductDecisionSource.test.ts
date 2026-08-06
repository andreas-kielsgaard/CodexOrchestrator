import { describe, expect, it } from 'vitest';
import { recordedEpicProductDecisionSource } from './recordedEpicProductDecisionSource';
import { recordedPresentationAdjunct } from '../orchestrationSection/recordedPresentationAdjunct';
import { selectTranscriptRange } from '../../features/agentSessions/transcriptProjector';
import { projectAgentSessionTranscript } from '../../features/agentSessions/transcriptProjector';
import { createRecordedDevelopmentApplicationComposition } from '../orchestrationSection/recordedOrchestrationClient';

describe('recorded Product Decisions source', () => {
  it('exposes one exact recorded passage and leaves non-passage evidence unavailable', async () => {
    const loaded = await recordedEpicProductDecisionSource.loadEpicProductDecisions(
      'epic-codex-runner-workspace',
    );
    if (loaded.kind !== 'available') throw new Error('Expected recorded Product Decisions.');
    const cited = loaded.snapshot.evidence.find(({ conversationCitation }) => conversationCitation);
    if (!cited?.conversationCitation) throw new Error('Expected a cited evidence record.');
    expect(
      recordedEpicProductDecisionSource.resolveEvidenceNavigation({
        epicId: loaded.snapshot.epicId,
        evidenceId: cited.evidenceId,
        originReference: cited.originReference,
        conversationCitation: cited.conversationCitation,
      }),
    ).toMatchObject({
      kind: 'available',
      destination: { sessionId: cited.conversationCitation.sessionId },
    });
    expect(
      recordedEpicProductDecisionSource.resolveEvidenceNavigation({
        epicId: loaded.snapshot.epicId,
        evidenceId: 'evidence-human-layout-review',
        originReference: loaded.snapshot.evidence[0]!.originReference,
        conversationCitation: cited.conversationCitation,
      }),
    ).toEqual({ kind: 'unavailable' });
  });

  it('uses a passage pointer that exists in the matching recorded Session transcript', () => {
    const session = recordedPresentationAdjunct.epic?.epicRunnerSession;
    if (!session?.transcript) throw new Error('Expected recorded Epic Runner transcript.');
    const resolution = recordedEpicProductDecisionSource.resolveEvidenceNavigation({
      epicId: 'epic-codex-runner-workspace',
      evidenceId: 'evidence-agent-session-layout-passage',
      originReference: {
        kind: 'agent_session_completed',
        opaqueId: 'recorded-epic-runner-manual-continuation-ready',
      },
      conversationCitation: {
        kind: 'agent_session_passage',
        sessionId: session.sessionId,
        invocationId: 'recorded-epic-runner-manual-continuation-ready-recorded-turn',
        passage: {
          kind: 'final_response',
          runtimeEventId: 'recorded-epic-runner-manual-continuation-ready-recorded-turn-response',
        },
      },
    });
    if (resolution.kind !== 'available') throw new Error('Expected available evidence.');
    const anchor = {
      sessionId: resolution.destination.sessionId,
      invocationId: resolution.destination.invocationId,
      kind: resolution.destination.passage.kind,
      ...('runtimeEventId' in resolution.destination.passage
        ? { runtimeEventId: resolution.destination.passage.runtimeEventId }
        : {}),
    } as const;
    expect(selectTranscriptRange(session.transcript, { start: anchor, end: anchor })).toHaveLength(
      1,
    );
  });

  it('keeps the same exact passage in the recorded application Session client', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const details = await composition.agentSessionClient.loadSession({
      sessionId: 'recorded-epic-runner-manual-continuation-ready',
    });
    const transcript = projectAgentSessionTranscript(details);
    expect(
      selectTranscriptRange(transcript, {
        start: {
          sessionId: details.session.id,
          invocationId: 'recorded-epic-runner-manual-continuation-ready-recorded-turn',
          kind: 'final_response',
          runtimeEventId: 'recorded-epic-runner-manual-continuation-ready-recorded-turn-response',
        },
        end: {
          sessionId: details.session.id,
          invocationId: 'recorded-epic-runner-manual-continuation-ready-recorded-turn',
          kind: 'final_response',
          runtimeEventId: 'recorded-epic-runner-manual-continuation-ready-recorded-turn-response',
        },
      }),
    ).toHaveLength(1);
  });
});
