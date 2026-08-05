import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { recordedPresentationAdjunct } from './recordedPresentationAdjunct';
import {
  createRecordedDevelopmentOrchestrationPresentation,
  recordedDevelopmentOrchestrationClient,
} from './recordedOrchestrationClient';
import { WorkUnitDetailWorkspace } from '../../features/orchestrations/components/WorkUnitDetailWorkspace';

describe('recorded Work Unit review consumer', () => {
  it('exercises exact turn inspection, disclosure, evidence navigation, and read-only behavior', async () => {
    const user = userEvent.setup();
    const openFileEvidence = vi.fn();
    const loaded = await recordedDevelopmentOrchestrationClient.load();
    if (loaded.kind !== 'ready') throw new Error('Recorded orchestration fixture did not load.');
    const view = createRecordedDevelopmentOrchestrationPresentation({
      includeWorkUnitReview: true,
    }).present(loaded.readModels);
    const workspace = view.epics[0]!.plan.items.find(
      ({ id }) => id === 'sprint-control-surface',
    )!.workspace!;
    const revision = workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === 'ECS-R4',
    )!;
    const unit = revision.workUnits.find(({ workUnitId }) => workUnitId === 'WU-ECS2E')!;
    const sessions = recordedPresentationAdjunct.sprints?.[
      'sprint-control-surface'
    ]!.workspaceAdjunct!.workUnitSessions.filter(({ workUnitId }) => workUnitId === 'WU-ECS2E');
    if (!sessions) throw new Error('Recorded Work Unit sessions are missing.');

    render(
      <WorkUnitDetailWorkspace
        unit={unit}
        lifecycleEntries={workspace.workUnitLifecycle.filter(
          ({ workUnitId }) => workUnitId === unit.workUnitId,
        )}
        workSlicePlanningPointGroupTitle="Recorded integration"
        sessions={sessions}
        onBack={vi.fn()}
        onOpenFileEvidence={openFileEvidence}
      />,
    );

    expect(screen.getByRole('tab', { name: 'Activity' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.queryByLabelText('Selected activity turn')).toBeNull();
    expect(screen.getAllByRole('region', { name: 'Application summary' })).toHaveLength(3);
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(
      Array.from(
        screen
          .getByLabelText('Work Unit Activity')
          .querySelectorAll<HTMLButtonElement>('.work-unit-activity__list > li > button'),
      ).map((button) => button.textContent),
    ).toEqual([
      expect.stringContaining('Implementation returned'),
      expect.stringContaining('Implementation reviewed'),
      expect.stringContaining('Application acceptance recorded'),
      expect.stringContaining('Review judgment recorded'),
    ]);
    const matchedLifecycleLabels = Array.from(
      screen
        .getByLabelText('Work Unit lifecycle turn log')
        .querySelectorAll<HTMLButtonElement>('ol > li > button'),
    )
      .map((button) => button.textContent)
      .filter((label) =>
        [
          'Implementation returned',
          'Implementation reviewed',
          'Application acceptance recorded',
          'Review judgment recorded',
        ].some((expected) => label?.includes(expected)),
      );
    expect(matchedLifecycleLabels).toEqual([
      expect.stringContaining('Implementation returned'),
      expect.stringContaining('Implementation reviewed'),
      expect.stringContaining('Application acceptance recorded'),
      expect.stringContaining('Review judgment recorded'),
    ]);

    const activityTab = screen.getByRole('tab', { name: 'Activity' });
    activityTab.focus();
    await user.keyboard('{ArrowRight}');
    expect(screen.getByRole('tab', { name: 'Evidence' })).toHaveAttribute('aria-selected', 'true');
    expect(document.activeElement).toBe(screen.getByRole('tab', { name: 'Evidence' }));
    await user.click(activityTab);

    await user.click(
      screen.getByRole('button', {
        name: /Implementation reviewed.*recorded-handler-WU-ECS2E-first-review/,
      }),
    );
    const firstInspector = screen.getByLabelText(
      'Agent Session turn: recorded-handler-WU-ECS2E-first-review',
    );
    expect(firstInspector).toHaveAttribute('data-session-id', 'recorded-session-WU-ECS2E');
    expect(firstInspector).toHaveTextContent('Complete recorded turn');
    expect(firstInspector).toHaveTextContent('Recorded lifecycle step 1');

    await user.click(within(firstInspector).getByText('Recorded steps'));
    expect(
      within(firstInspector).getByLabelText(
        'Processing for invocation recorded-handler-WU-ECS2E-first-review',
      ),
    ).toBeVisible();

    await user.click(screen.getByRole('tab', { name: 'Evidence' }));
    await user.click(
      screen.getByRole('button', {
        name: /Open exact diff for src\/features\/orchestrations\/components\/WorkUnitDetailWorkspace.tsx/,
      }),
    );
    expect(openFileEvidence).toHaveBeenCalledWith(
      {
        reviewId: 'recorded-work-unit-review',
        changedFileId: 'recorded-file-work-unit-detail',
      },
      {
        inspectionState: {
          tab: 'evidence',
          activityId:
            'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-2:implementer-reporting:recorded-implementer-WU-ECS2E-second-return',
          sessionId: 'recorded-implementer-WU-ECS2E',
          invocationId: 'recorded-implementer-WU-ECS2E-second-return',
        },
      },
    );
    expect(
      screen.getByText('Focused Work Unit Activity and Evidence interaction checks'),
    ).toBeVisible();
    expect(screen.getByText('Unavailable diff')).toBeVisible();
    await user.click(
      within(screen.getByRole('region', { name: 'File evidence' })).getByRole('button', {
        name: 'View owning activity',
      }),
    );

    const selected = screen.getByRole('button', {
      name: /Application acceptance recorded.*recorded-implementer-WU-ECS2E-second-return/,
    });
    expect(selected).toHaveAttribute('aria-pressed', 'true');
    expect(selected.closest('li')).toHaveClass('is-selected');
    expect(
      screen.getByLabelText('Agent Session turn: recorded-implementer-WU-ECS2E-second-return'),
    ).toBeVisible();
    expect(screen.getByText('Previous input')).toBeVisible();

    await user.click(
      screen.getByRole('button', {
        name: /Review judgment recorded.*Recorded WU-ECS2E Work Unit Handler/,
      }),
    );
    expect(
      screen.getByLabelText('Agent Session turn: recorded-handler-WU-ECS2E-acceptance'),
    ).toBeVisible();
  });
});
