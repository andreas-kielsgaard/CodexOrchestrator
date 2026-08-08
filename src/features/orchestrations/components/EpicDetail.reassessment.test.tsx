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

  it('shows settled and unresolved Epic settlement facts with calm non-private wording', () => {
    const base = {
      id: 'epic-1', name: 'Settlement Epic', goal: 'Preserve settlement facts.',
      movement: { kind: 'reevaluating_direction' as const }, state: 'running' as const,
      plan: { items: [] },
    };
    const { rerender } = render(
      <EpicDetail
        epic={{ ...base, epicSettlement: {
          kind: 'settled', settlementId: 'private-settlement-id', persistedAt: '2026-08-05T00:00:00Z',
        } }}
        artifactAccessController={undefined as never}
        selectedSprintId={null} selectedRevisionId={null} detailLocation={{ kind: 'sprint' }}
        onOpenSprint={vi.fn()} onCloseSprint={vi.fn()} onSelectedRevisionChange={vi.fn()}
        onDetailLocationChange={vi.fn()} onBack={vi.fn()}
      />,
    );
    const settled = screen.getByRole('region', { name: 'Epic settlement' });
    expect(settled).toHaveTextContent('Settlement was recorded');
    expect(settled).not.toHaveTextContent('private-settlement-id');
    expect(settled).not.toHaveTextContent('publication');

    rerender(
      <EpicDetail
        epic={{ ...base, epicSettlement: {
          kind: 'unresolved', reasonCode: 'needs_authority',
          resumptionFact: 'Restore the exact settlement authority.', recordedAt: '2026-08-05T00:00:00Z',
        } }}
        artifactAccessController={undefined as never}
        selectedSprintId={null} selectedRevisionId={null} detailLocation={{ kind: 'sprint' }}
        onOpenSprint={vi.fn()} onCloseSprint={vi.fn()} onSelectedRevisionChange={vi.fn()}
        onDetailLocationChange={vi.fn()} onBack={vi.fn()}
      />,
    );
    const unresolved = screen.getByRole('region', { name: 'Epic settlement' });
    expect(unresolved).toHaveTextContent('Epic settlement remains unresolved');
    expect(unresolved).toHaveTextContent('Resume by: Restore the exact settlement authority.');
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

  it('reinitializes every disclosure collapsed after leaving and re-entering Product Decisions', async () => {
    renderProductDecisions(false);
    const firstSummary = (await screen.findAllByText('Intent and evidence'))[0]!;
    fireEvent.click(firstSummary);
    expect(firstSummary.closest('details')).toHaveAttribute('open');

    fireEvent.click(screen.getByRole('button', { name: 'Plan' }));
    fireEvent.click(screen.getByRole('button', { name: 'Product decisions' }));

    expect(
      (await screen.findAllByText('Intent and evidence')).every(
        (summary) => !summary.closest('details')?.hasAttribute('open'),
      ),
    ).toBe(true);
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
