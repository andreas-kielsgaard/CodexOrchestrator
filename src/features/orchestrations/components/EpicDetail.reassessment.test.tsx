import { render, screen } from '@testing-library/react';
import { EpicDetail } from './EpicDetail';

describe('Epic reassessment presentation', () => {
  it('shows Epic reassessment and preserves the unresolved Sprint boundary', () => {
    render(
      <EpicDetail
        epic={{
          id: 'epic-1',
          name: 'Escalated Epic',
          goal: 'Preserve the concern.',
          movement: { kind: 'reevaluating_direction' },
          state: 'running',
          epicEscalationReceivers: [
            {
              epicId: 'epic-1',
              sprintId: 'sprint-1',
              deliveryRequestedAt: '2026-08-05T00:00:00Z',
              launchAcceptedAt: '2026-08-05T00:00:02Z',
              semanticReassessmentRecordedAt: '2026-08-05T00:00:03Z',
              disposition: {
                movementKind: 'human_or_external_attention',
                rationale: 'The concern remains unresolved.',
                humanExternalAttention: {
                  reason: 'A bounded authority decision is needed.',
                  authorityNeeded: 'External dependency owner.',
                  evidenceContext: 'The exact Sprint concern.',
                  resumptionPath: 'Resume from the unchanged concern.',
                },
              },
            },
          ],
          plan: { items: [] },
        }}
        artifactAccessController={undefined as never}
        selectedSprintId={null}
        selectedRevisionId={null}
        detailLocation={{ kind: 'sprint' }}
        onOpenSprint={vi.fn()}
        onCloseSprint={vi.fn()}
        onSelectedRevisionChange={vi.fn()}
        onDetailLocationChange={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    expect(screen.getByRole('region', { name: 'Epic reassessment' })).toHaveTextContent(
      'The concern remains unresolved',
    );
    expect(screen.getByRole('region', { name: 'Epic reassessment' })).toHaveTextContent(
      'not Sprint selection, start, settlement, completion, or acceptance',
    );
  });

  it('labels direct Sprint-result stages without claiming Epic settlement or acceptance', () => {
    render(
      <EpicDetail
        epic={{
          id: 'epic-1', name: 'Result Epic', goal: 'Preserve stage ownership.',
          movement: { kind: 'reevaluating_direction' }, state: 'running', plan: { items: [] },
          sprintResultProjections: [{
            resultId: 'result-1', decisionId: 'decision-1', sprintId: 'sprint-1', epicId: 'epic-1',
            resultKind: 'settled', recordedAt: '2026-08-05T00:00:00Z',
            receiver: {
              deliveryRequestedAt: '2026-08-05T00:00:01Z', deliveryPersistedAt: '2026-08-05T00:00:02Z',
              semanticReassessmentRecordedAt: '2026-08-05T00:00:03Z',
            },
            realization: {
              outcomeKind: 'retained_attention', consideredAt: '2026-08-05T00:00:04Z',
              retainedAttentionCode: 'concern_preserved', retainedAttentionRecordedAt: '2026-08-05T00:00:05Z',
            },
          }],
        }}
        artifactAccessController={undefined as never}
        selectedSprintId={null} selectedRevisionId={null} detailLocation={{ kind: 'sprint' }}
        onOpenSprint={vi.fn()} onCloseSprint={vi.fn()} onSelectedRevisionChange={vi.fn()}
        onDetailLocationChange={vi.fn()} onBack={vi.fn()}
      />,
    );
    const region = screen.getByRole('region', { name: 'Sprint result receipt' });
    expect(region).toHaveTextContent('Local Sprint result: settled');
    expect(region).toHaveTextContent('Epic receipt requested');
    expect(region).toHaveTextContent('Retained concern/attention recorded');
    expect(region).toHaveTextContent('do not settle the Epic');
  });
});
