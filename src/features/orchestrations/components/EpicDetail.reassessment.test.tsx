import { fireEvent, render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import { recordedEpicProductDecisionSource } from '../../../dev/productDecisions/recordedEpicProductDecisionSource';
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

  it('suppresses the Product Decisions local Back when global history is available', () => {
    renderProductDecisions(true);
    expect(screen.queryByRole('button', { name: 'Back to Epics' })).toBeNull();
  });

  it('keeps the Product Decisions local Back as a truthful fallback without history', () => {
    const onBack = renderProductDecisions(false);
    fireEvent.click(screen.getByRole('button', { name: 'Back to Epics' }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});

function renderProductDecisions(globalBackAvailable: boolean) {
  const onBack = vi.fn();
  render(
    <EpicDetail
      epic={{
        id: 'epic-codex-runner-workspace',
        name: 'Recorded Epic',
        goal: 'Review.',
        movement: { kind: 'planning_next_work' },
        state: 'running',
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
      onBack={onBack}
      globalBackAvailable={globalBackAvailable}
      requestedProductDecisions
      epicProductDecisionSource={recordedEpicProductDecisionSource}
    />,
  );
  return onBack;
}
