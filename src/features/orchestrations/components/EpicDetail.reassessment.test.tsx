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
});
