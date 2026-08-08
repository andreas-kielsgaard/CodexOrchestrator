import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { ContextualFileReviewResult } from '../../application/contextualFileReview';
import {
  composeProductOrchestrationReadModels,
  type ProductSprintRunnerTransitionStatusV1,
} from '../../application/orchestrations';
import { presentProductOrchestrations } from '../../app/orchestrationPresentation';
import { recordedPresentationAdjunct } from '../../dev/orchestrationSection/recordedPresentationAdjunct';
import { recordedProductReadCompositionInput } from '../../dev/orchestrationSection/recordedProductReadCompositionInput';
import { recordedAgentSessionDetails } from '../../dev/orchestrationSection/recordedPresentationAdjunct';
import {
  createRecordedAgentSessionClient,
  createRecordedAgentSessionStore,
} from '../../dev/agentSessions';
import { movementLabel, type OrchestrationSectionView } from './orchestrationModel';
import { OrchestrationSection } from './OrchestrationSection';

const canonicalRecordedView = presentProductOrchestrations(
  composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
  recordedPresentationAdjunct,
);
/** Completed summary-only Sprints stay dialogs; proposed Plans keep their typed workspace. */
const disposableRecordedOrchestrationView = {
  epics: canonicalRecordedView.epics.map((epic) => ({
    ...epic,
    plan: {
      ...epic.plan,
      items: epic.plan.items.map((item) =>
        item.id === 'sprint-control-surface' || item.status === 'not_started'
          ? item
          : { ...item, workspace: undefined },
      ),
    },
  })),
};

describe('OrchestrationSection', () => {
  it('renders exactly the three semantic overview columns from structured movement and state', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);

    expect(screen.getAllByRole('columnheader').map((header) => header.textContent)).toEqual([
      'Epic',
      'Current movement',
      'State',
    ]);
    expect(screen.getByText(/0 processing.*0 reviewing/)).toBeVisible();
    const ready = screen.getByText('Ready to continue');
    expect(ready).toHaveClass('epic-state--ready_to_continue');
    expect(screen.queryByRole('columnheader', { name: /attention|continuation/i })).toBeNull();
  });

  it('uses the accepted movement vocabulary without display-string input', () => {
    expect(movementLabel({ kind: 'preparing_next_sprint' })).toBe('Preparing next Sprint');
    expect(movementLabel({ kind: 'reviewing_sprint_completion' })).toBe(
      'Reviewing Sprint completion',
    );
    expect(movementLabel({ kind: 'planning_next_work' })).toBe('Planning next work');
    expect(movementLabel({ kind: 'starting_work_units', count: 2 })).toBe('Starting 2 Work Units');
    expect(movementLabel({ kind: 'executing_work', processingCount: 2, reviewingCount: 1 })).toBe(
      '2 processing · 1 reviewing',
    );
    expect(movementLabel({ kind: 'reviewing_returned_work', count: 3 })).toBe(
      'Reviewing 3 returned Work Units',
    );
    expect(movementLabel({ kind: 'integrating_accepted_work' })).toBe('Integrating accepted work');
    expect(movementLabel({ kind: 'reevaluating_direction' })).toBe('Reevaluating direction');
  });

  it('makes the ordered Plan primary and opens both completed and proposed Sprint Plans', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );

    expect(screen.getByRole('main', { name: 'Epic detail' })).toBeVisible();
    expect(screen.getByRole('region', { name: 'Epic plan' })).toBeVisible();
    expect(screen.queryByText('High-value attention')).toBeNull();
    expect(screen.queryByText('Current movement')).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Epic plan' })).toBeNull();
    expect(screen.queryByText('Ordered plan')).toBeNull();
    expect(screen.queryByText('Latest Epic Runner agent turn')).toBeNull();

    const plan = within(screen.getByRole('region', { name: 'Epic plan' })).getByRole('list');
    const items = within(plan).getAllByRole('listitem');
    expect(items).toHaveLength(6);
    expect(items[0]).toHaveClass('sprint-plan-item--completed');
    expect(items[1]).toHaveClass('sprint-plan-item--completed');
    expect(items[2]).toHaveClass('sprint-plan-item--completed');
    expect(items[3]).toHaveClass('sprint-plan-item--in_progress');
    expect(items[4]).toHaveClass('sprint-plan-item--not_started');
    const proposed = screen.getByRole('button', {
      name: 'View proposed Plan: Planner and Work Unit Interaction Discovery',
    });
    fireEvent.click(proposed);
    expect(screen.getByRole('main', { name: 'Sprint detail' })).toBeVisible();
    expect(screen.getByLabelText('Sprint context')).toHaveTextContent('Planned');
    expect(screen.getByLabelText('Sprint Runner pre-start forecast')).toHaveTextContent(
      'Concerns before Sprint start',
    );
    expect(screen.queryByRole('tablist', { name: 'Sprint information' })).toBeNull();
    expect(screen.queryByRole('button', { name: /Open Work Unit/ })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Back to Epic' }));

    fireEvent.click(
      screen.getByRole('button', {
        name: 'Open Sprint: Preparation Canvas Recovery and Foundation Acceptance',
      }),
    );
    expect(screen.getByRole('dialog', { name: /Preparation Canvas Recovery/ })).toBeVisible();
    expect(
      screen.getByText(
        'The preparation canvas and reusable Agent Session foundation were accepted.',
      ),
    ).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Close Sprint detail' }));

    const sprintOpener = screen.getByRole('button', {
      name: 'Open Sprint: Sprint Control Surface Discovery',
    });
    fireEvent.click(sprintOpener);
    expect(screen.getByRole('main', { name: 'Sprint detail' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Sprint Control Surface Discovery' })).toBeVisible();
    expect(screen.getByRole('region', { name: 'Sprint Runner plan' })).toHaveTextContent(
      'Recorded branch and repository state was reevaluated at Sprint start.',
    );
    const sprintContext = screen.getByLabelText('Sprint context');
    expect(sprintContext).toHaveTextContent('Sprint Control Surface Discovery');
    expect(sprintContext).toHaveTextContent(
      'Determine the minimum in-app surface needed to understand and supervise one started Sprint.',
    );
    expect(sprintContext).not.toHaveTextContent('Current movement');
    expect(sprintContext).not.toHaveTextContent('Attention');

    const returnToEpic = screen.getByRole('button', { name: 'Back to Epic' });
    expect(returnToEpic).toHaveFocus();
    fireEvent.click(returnToEpic);
    expect(screen.getByRole('main', { name: 'Epic detail' })).toBeVisible();
    expect(
      screen.getByRole('button', {
        name: 'Open Sprint: Sprint Control Surface Discovery',
      }),
    ).toHaveFocus();

    const back = screen.getByRole('button', { name: 'Back to Epics' });
    back.focus();
    expect(back).toHaveFocus();
    fireEvent.click(back);
    expect(screen.getByRole('main', { name: 'Orchestration' })).toBeVisible();
  }, 10_000);

  it('keeps the Sprint Agent Session in the reusable vertical split', async () => {
    render(
      <OrchestrationSection
        view={disposableRecordedOrchestrationView}
        agentSessionComposition={recordedComposition()}
      />,
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );

    const session = screen.getByRole('region', { name: 'Sprint Agent Session' });
    expect(session.querySelector('.shared-agent-session__compact')).toBeNull();
    expect(session).toHaveTextContent('Agent SessionSprint control surface discovery');
    expect(within(session).getByLabelText('Sprint Agent Session conversation')).toBeVisible();
    expect(
      screen.getByRole('separator', { name: 'Resize Detail flow and Agent Session' }),
    ).toHaveAttribute('aria-orientation', 'horizontal');
    expect(screen.getByRole('button', { name: 'Maximize flow' })).toBeVisible();
    expect(screen.queryByLabelText('Epic Runner Agent Session conversation')).toBeNull();

    const message = await within(session).findByRole('textbox', { name: 'Message' });
    fireEvent.change(message, { target: { value: 'Record Sprint feedback' } });
    fireEvent.click(within(session).getByRole('button', { name: 'Send' }));
    expect(await within(session).findByText('Record Sprint feedback')).toBeVisible();
    expect(within(session).queryByText(/No live agent was invoked/)).toBeNull();
    expect(within(session).queryByRole('button', { name: /Collapse/ })).toBeNull();
  });

  it('shows only durable Sprint Runner activation evidence on the existing Sprint surface', () => {
    const view = {
      epics: canonicalRecordedView.epics.map((epic) => ({
        ...epic,
        plan: {
          ...epic.plan,
          items: epic.plan.items.map((item) =>
            item.name === 'Planner and Work Unit Interaction Discovery' && item.workspace
              ? {
                  ...item,
                  workspace: {
                    ...item.workspace,
                    sprint: {
                      ...item.workspace.sprint,
                      sprintRunnerTransition: {
                        label: 'Sprint Runner launch accepted — pre-start ready',
                        requestedAt: 't',
                        authorizedAt: 't',
                        sessionCreatedAt: 't',
                        harnessAppliedAt: 't',
                        launchAcceptedAt: 't',
                        preStartReady: true,
                        lifecycleObserved: false as const,
                        accepted: false as const,
                        downstreamNotStarted: true,
                      },
                    },
                  },
                }
              : item,
          ),
        },
      })),
    };
    render(<OrchestrationSection view={view} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'View proposed Plan: Planner and Work Unit Interaction Discovery',
      }),
    );
    const activation = screen.getByLabelText('Sprint Runner activation');
    expect(activation).toHaveTextContent('Sprint Runner launch accepted — pre-start ready');
    expect(activation).toHaveTextContent('Provider/receiver activation has not been observed');
    expect(activation).toHaveTextContent('No Work Slice or Work Unit has been created');
    expect(activation).not.toHaveTextContent('delivery has been observed');
    expect(activation).not.toHaveTextContent('Sprint has not started');
  });

  it('renders persisted-not-launched and planning-ready activation branches without overstating delivery', () => {
    const withTransition = (sprintRunnerTransition: ProductSprintRunnerTransitionStatusV1) => ({
      epics: canonicalRecordedView.epics.map((epic) => ({
        ...epic,
        plan: {
          ...epic.plan,
          items: epic.plan.items.map((item) =>
            item.name === 'Planner and Work Unit Interaction Discovery' && item.workspace
              ? {
                  ...item,
                  workspace: {
                    ...item.workspace,
                    sprint: { ...item.workspace.sprint, sprintRunnerTransition },
                  },
                }
              : item,
          ),
        },
      })),
    });
    const openSprint = () => {
      fireEvent.click(
        screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
      );
      fireEvent.click(
        screen.getByRole('button', {
          name: 'View proposed Plan: Planner and Work Unit Interaction Discovery',
        }),
      );
    };
    const persisted = render(
      <OrchestrationSection
        view={withTransition({
          label: 'Epic continuation invocation persisted; launch acceptance pending',
          requestedAt: 't',
          authorizedAt: 't',
          sessionCreatedAt: 't',
          harnessAppliedAt: 't',
          launchAcceptedAt: 't',
          preStartReady: true,
          lifecycleObserved: true,
          accepted: true,
          preStartSemanticOutcomeRecordedAt: 't',
          preStartLifecycleObservedAt: 't',
          preStartOutcomeAcceptedAt: 't',
          parentContinuationDeliveryRequestedAt: 't',
          parentContinuationDeliveryPersistedAt: 't',
          downstreamNotStarted: true,
        })}
      />,
    );
    openSprint();
    const pending = screen.getByLabelText('Sprint Runner activation');
    expect(pending).toHaveTextContent('Epic continuation delivery requested');
    expect(pending).toHaveTextContent('Epic continuation invocation persisted');
    expect(pending).toHaveTextContent('Provider/receiver activation has not been observed');
    expect(pending).not.toHaveTextContent('Provider/receiver activation has been observed');
    persisted.unmount();

    render(
      <OrchestrationSection
        view={withTransition({
          label: 'Sprint planning-ready; downstream not started',
          requestedAt: 't',
          authorizedAt: 't',
          sessionCreatedAt: 't',
          harnessAppliedAt: 't',
          launchAcceptedAt: 't',
          preStartReady: true,
          lifecycleObserved: true,
          accepted: true,
          sprintStartAuthorizedAt: 't',
          sprintStartPersistedAt: 't',
          sprintContinuationLaunchAcceptedAt: 't',
          repositoryBranchReevaluationRecordedAt: 't',
          planningReadyAt: 't',
          providerReceiverActivationObservedAt: 't',
          downstreamNotStarted: true,
        })}
      />,
    );
    openSprint();
    const ready = screen.getByLabelText('Sprint Runner activation');
    expect(ready).toHaveTextContent('Sprint start authorized and persisted');
    expect(ready).toHaveTextContent('Repository and branch reevaluation recorded');
    expect(ready).toHaveTextContent('Planning-ready; downstream has not started');
    expect(ready).toHaveTextContent('Provider/receiver activation has been observed');
    expect(ready).not.toHaveTextContent('Provider/receiver activation has not been observed');
  });

  it('keeps Sprint-owned File Review invocation and bounded status in the persistent header', async () => {
    let settle!: (result: ContextualFileReviewResult) => void;
    const requestFileReview = vi.fn(
      () =>
        new Promise<ContextualFileReviewResult>((resolve) => {
          settle = resolve;
        }),
    );
    render(
      <OrchestrationSection
        view={disposableRecordedOrchestrationView}
        onRequestFileReview={requestFileReview}
      />,
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );

    const action = screen.getByRole('button', { name: 'Review files' });
    const control = action.closest('.sprint-file-review-control') as HTMLElement;
    fireEvent.click(screen.getByRole('tab', { name: 'Documents' }));
    expect(action).toBeVisible();
    expect(action.closest('.detail-workspace__control')).toBeVisible();
    expect(action.closest('.sprint-start-assessment')).toBeNull();

    fireEvent.click(action);
    expect(action).toBeDisabled();
    expect(within(control).getByRole('status')).toHaveTextContent('Preparing File Review…');
    expect(requestFileReview).toHaveBeenCalledWith(
      'sprint-control-surface',
      expect.objectContaining({ kind: 'sprint', sprintId: 'sprint-control-surface' }),
    );

    await act(async () => {
      settle({
        status: 'failed',
        reason: 'source_not_ready',
        message: 'The Sprint source is not ready for File Review.',
      });
    });
    expect(within(control).getByRole('alert')).toHaveTextContent(
      'The Sprint source is not ready for File Review.',
    );
    expect(action).toBeEnabled();
  });

  it('retains the shared Sprint File Review request across planning-point and Work Unit detail', async () => {
    let settle!: (result: ContextualFileReviewResult) => void;
    const requestFileReview = vi.fn(
      () =>
        new Promise<ContextualFileReviewResult>((resolve) => {
          settle = resolve;
        }),
    );
    render(
      <OrchestrationSection
        view={disposableRecordedOrchestrationView}
        onRequestFileReview={requestFileReview}
      />,
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'Open Work Slice planning point: Integrated detail surfaces',
      }),
    );

    expect(
      screen.getByRole('main', {
        name: 'Work Slice planning point detail: Integrated detail surfaces',
      }),
    ).toBeVisible();
    let action = screen.getByRole('button', { name: 'Review files' });
    let control = action.closest('.sprint-file-review-control') as HTMLElement;
    expect(action.closest('.detail-workspace__control')).toBeVisible();
    fireEvent.click(action);
    expect(action).toBeDisabled();
    expect(within(control).getByRole('status')).toHaveTextContent('Preparing File Review…');
    expect(requestFileReview).toHaveBeenNthCalledWith(
      1,
      'sprint-control-surface',
      expect.objectContaining({
        kind: 'work_slice_planning_point',
        workSlicePlanningPointId: 'planner-r4-integration',
      }),
    );

    fireEvent.click(screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ }));
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    action = screen.getByRole('button', { name: 'Review files' });
    control = action.closest('.sprint-file-review-control') as HTMLElement;
    expect(action.closest('.detail-workspace__control')).toBeVisible();
    expect(action).toBeDisabled();
    expect(within(control).getByRole('status')).toHaveTextContent('Preparing File Review…');
    expect(requestFileReview).toHaveBeenCalledTimes(1);
    expect(requestFileReview).toHaveBeenCalledTimes(1);

    await act(async () => {
      settle({
        status: 'failed',
        reason: 'conflict',
        message: 'File Review could not confirm one current Sprint source.',
      });
    });
    const failure = within(control).getByRole('alert');
    expect(failure).toHaveAttribute('data-reason', 'conflict');
    expect(failure).toHaveTextContent('File Review could not confirm one current Sprint source.');
    expect(action).toBeEnabled();
  });

  it('navigates Plan and Work Unit details, restores focus, and coordinates dual sessions', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );

    const planOpener = screen.getByRole('button', {
      name: 'Open Work Slice planning point: Integrated detail surfaces',
    });
    planOpener.focus();
    fireEvent.click(planOpener);
    expect(
      screen.getByRole('main', {
        name: 'Work Slice planning point detail: Integrated detail surfaces',
      }),
    ).toBeVisible();
    const workflow = screen.getByLabelText(
      'Work Slice planning point actor and conversation workflow',
    );
    expect(workflow).toHaveTextContent('Recorded review');
    expect(workflow).toHaveTextContent(
      'Handler: Work Unit Handler · Implementer: Work Unit Implementer',
    );
    expect(screen.queryByLabelText('Recorded Plan lifecycle')).toBeNull();
    expect(screen.queryByLabelText(/Integrated detail surfaces flow/)).toBeNull();
    expect(screen.queryByLabelText('Sprint feedback plan change')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    expect(
      screen.getByRole('button', {
        name: 'Open Work Slice planning point: Integrated detail surfaces',
      }),
    ).toHaveFocus();

    const workUnitOpener = screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ });
    fireEvent.click(workUnitOpener);
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    expect(screen.getByLabelText('Work Unit Activity and Evidence')).toBeVisible();
    expect(screen.getByRole('tab', { name: 'Activity' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.queryByRole('region', { name: /Agent Session$/ })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Reviewer' })).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Reviewer' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Reviewer Agent Session' })).toBeNull();
    expect(screen.getByLabelText('Work Unit context')).not.toHaveTextContent(
      'Recorded/theoretical fixture only',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Back to Work Slice planning point' }));
    expect(
      screen.getByRole('main', {
        name: 'Work Slice planning point detail: Integrated detail surfaces',
      }),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Back to Sprint' })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    expect(screen.getByRole('main', { name: 'Sprint detail' })).toBeVisible();
    expect(screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ })).toHaveFocus();
    expect(screen.queryByRole('main', { name: /Work Slice planning point detail/ })).toBeNull();
  });

  it('resolves the WU-ECS2E planner and handler without a Reviewer Session', async () => {
    render(
      <OrchestrationSection
        view={canonicalRecordedView}
        agentSessionComposition={recordedComposition()}
      />,
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'Open Work Slice planning point: Integrated detail surfaces',
      }),
    );

    const planner = screen.getByRole('region', { name: 'Work Slice Planner Agent Session' });
    expect(
      within(planner).getByRole('region', {
        name: 'Work Slice Planner Agent Session conversation surface',
      }),
    ).toBeVisible();
    fireEvent.click(within(planner).getByRole('button', { name: 'Open Agent Session' }));
    expect(await within(planner).findByRole('textbox', { name: 'Message' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    fireEvent.click(screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ }));
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    expect(screen.queryByRole('region', { name: /Agent Session$/ })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Reviewer' })).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Reviewer' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Reviewer Agent Session' })).toBeNull();
  });

  it('keeps historical revision context while opening Plan and Work Unit details', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );

    const revision = screen.getByRole('combobox', { name: 'Plan revision' });
    fireEvent.change(revision, { target: { value: 'ECS-R1' } });
    fireEvent.click(
      screen.getByRole('button', {
        name: 'Open Work Slice planning point: Planning point ECS-R1',
      }),
    );
    expect(
      screen.getByRole('main', {
        name: 'Work Slice planning point detail: Planning point ECS-R1',
      }),
    ).toBeVisible();
    expect(screen.getByLabelText('Detailed workflow unavailable')).toHaveTextContent(
      'No detailed turn sequence is recorded for this Work Slice planning point.',
    );
    expect(screen.queryByText(/historical Plan/)).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    expect(screen.getByRole('combobox', { name: 'Plan revision' })).toHaveValue('ECS-R1');
    fireEvent.click(screen.getByRole('button', { name: /Open Work Unit WU-ECS2:/ }));
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2' })).toBeVisible();
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Superseded and never launched',
    );
    expect(
      screen.getByText(
        'No exact chronological Activity correlation is available for this Work Unit.',
      ),
    ).toBeVisible();
    expect(screen.queryByRole('region', { name: /Agent Session$/ })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Work Slice planning point' }));
    expect(
      screen.getByRole('main', {
        name: 'Work Slice planning point detail: Planning point ECS-R1',
      }),
    ).toBeVisible();
  });

  it('renders the actual current planning point from typed Work Units and Session references', () => {
    render(<OrchestrationSection view={canonicalRecordedView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint and Epic Detail Review' }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'Open Work Slice planning point: Relationship foundation',
      }),
    );

    expect(
      screen.getByRole('main', {
        name: 'Work Slice planning point detail: Relationship foundation',
      }),
    ).toBeVisible();
    const relationships = screen.getByLabelText('Work Slice planning point relationships');
    expect(
      within(relationships).getByLabelText('Work Slice Planner relationship'),
    ).toHaveTextContent('Recorded review Work Slice Planner');
    expect(
      within(relationships).getByRole('listitem', { name: /Work Unit WU-RD1:/ }),
    ).toBeVisible();
    expect(
      within(relationships).getByRole('listitem', { name: /Work Unit WU-RD5:/ }),
    ).toBeVisible();
    expect(
      within(relationships).getByRole('button', { name: /Open Work Unit WU-RD5:/ }),
    ).toBeVisible();
    expect(
      within(relationships).getByLabelText('WU-RD1 Work Unit Handler relationship'),
    ).toHaveTextContent('Relationship Work Unit Handler');
    expect(
      within(relationships).getByLabelText('WU-RD1 Work Unit Implementer relationship'),
    ).toHaveTextContent('Relationship Work Unit Implementer');
    expect(
      within(relationships).getByLabelText('WU-RD5 Work Unit Handler unavailable'),
    ).toBeVisible();
    expect(
      within(relationships).getByLabelText('WU-RD5 Work Unit Implementer unavailable'),
    ).toBeVisible();
    expect(screen.getByLabelText('Detailed workflow unavailable')).toBeVisible();
    expect(screen.queryByText(/historical Plan/)).toBeNull();

    fireEvent.click(
      within(relationships).getByRole('button', {
        name: 'Open Work Unit WU-RD1: Model review relationships',
      }),
    );
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-RD1' })).toBeVisible();
    expect(
      screen.getByText(
        'No exact chronological Activity correlation is available for this Work Unit.',
      ),
    ).toBeVisible();
    expect(screen.queryByRole('region', { name: /Agent Session$/ })).toBeNull();
  });

  it('describes Sprint Auto-flow without claiming eligibility or execution', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );

    const controls = screen.getByLabelText('Sprint controls');
    const policy = within(controls).getByRole('switch', { name: /Sprint Auto-flow/ });
    expect(policy).not.toBeChecked();
    expect(within(controls).queryByText('Recorded only')).toBeNull();
    const descriptionId = policy.getAttribute('aria-describedby');
    expect(descriptionId).toBeTruthy();
    expect(document.getElementById(descriptionId!)).toHaveTextContent(
      'accepted child Work Units should start the next planning round',
    );
    expect(document.getElementById(descriptionId!)).not.toHaveTextContent('recorded');

    fireEvent.click(policy);
    expect(policy).not.toBeChecked();
    expect(
      disposableRecordedOrchestrationView.epics[0].plan.items[2].workspace?.continuation.policy
        ?.automaticEnabled,
    ).toBe(false);
    expect(screen.getByRole('status')).toHaveTextContent('unsupported');
  });

  it('provides semantic Sprint tabs with roving keyboard focus and one contained panel', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );

    const tablist = screen.getByRole('tablist', { name: 'Sprint information' });
    const flow = within(tablist).getByRole('tab', { name: 'Flow' });
    const concerns = within(tablist).getByRole('tab', { name: 'Concerns' });
    const documents = within(tablist).getByRole('tab', { name: 'Documents' });
    expect(flow).toHaveAttribute('aria-selected', 'true');
    expect(screen.getAllByRole('tabpanel')).toHaveLength(1);
    expect(document.querySelector('.detail-workspace__hotbar-navigation')).toContainElement(
      tablist,
    );

    flow.focus();
    fireEvent.keyDown(flow, { key: 'ArrowRight' });
    expect(concerns).toHaveFocus();
    expect(concerns).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tabpanel')).toHaveAttribute('id', 'sprint-concerns-panel');

    fireEvent.keyDown(concerns, { key: 'End' });
    expect(documents).toHaveFocus();
    expect(screen.getByRole('tabpanel')).toHaveAttribute('id', 'sprint-documents-panel');

    fireEvent.keyDown(documents, { key: 'Home' });
    expect(flow).toHaveFocus();
    expect(screen.getByRole('region', { name: 'Sprint Runner plan' })).toBeVisible();
  });

  it('maximizes data-driven concerns, restores focus, and opens truthful Work Unit detail', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );
    fireEvent.click(screen.getByRole('tab', { name: 'Concerns' }));

    const overview = screen.getByLabelText('Sprint concerns overview');
    expect(within(overview).getAllByRole('button')).toHaveLength(3);
    expect(within(overview).getAllByText('Completed')).toHaveLength(3);
    expect(within(overview).queryByText('Responsibility accepted')).toBeNull();
    const multiUnitConcern = within(overview).getByRole('button', {
      name: /Flow and detail navigation.*2 linked Work Units/,
    });
    expect(multiUnitConcern).toBeVisible();

    const convergence = multiUnitConcern;
    convergence.focus();
    fireEvent.click(convergence);
    expect(screen.getByLabelText('Concern detail: Flow and detail navigation')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back to concerns' }));
    expect(screen.getByRole('button', { name: /Flow and detail navigation/ })).toHaveFocus();

    fireEvent.click(screen.getByRole('button', { name: /Flow and detail navigation/ }));
    const detail = screen.getByLabelText('Concern detail: Flow and detail navigation');
    const unit = within(detail).getByRole('button', { name: /WU-ECS2E/ });
    fireEvent.click(unit);
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back to Concern' }));
    expect(screen.getByLabelText('Concern detail: Flow and detail navigation')).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-ECS2E/ })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Back to concerns' }));
    expect(screen.getByRole('button', { name: /Flow and detail navigation/ })).toHaveFocus();
  });

  it('keeps a historical Flow selection while Concerns opens linked Work Units from the active revision', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );

    const revision = screen.getByRole('combobox', { name: 'Plan revision' });
    fireEvent.change(revision, { target: { value: 'ECS-R1' } });
    expect(revision).toHaveValue('ECS-R1');

    fireEvent.click(screen.getByRole('tab', { name: 'Concerns' }));
    fireEvent.click(screen.getByRole('button', { name: /Flow and detail navigation/ }));
    const concern = screen.getByLabelText('Concern detail: Flow and detail navigation');
    const activeUnit = within(concern).getByRole('button', { name: /WU-ECS2E/ });
    fireEvent.click(activeUnit);

    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back to Concern' }));
    expect(screen.getByRole('button', { name: /WU-ECS2E/ })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Back to concerns' }));
    expect(screen.getByRole('button', { name: /Flow and detail navigation/ })).toHaveFocus();

    fireEvent.click(screen.getByRole('tab', { name: 'Flow' }));
    expect(screen.getByRole('combobox', { name: 'Plan revision' })).toHaveValue('ECS-R1');
  });

  it('orders document rows newest first', async () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );
    fireEvent.click(screen.getByRole('tab', { name: 'Documents' }));

    const list = screen
      .getByLabelText('Sprint documents')
      .querySelector<HTMLElement>('.sprint-documents__list')!;
    const documents = within(list).getAllByRole('article');
    expect(documents.map((document) => document.textContent)).toEqual([
      expect.stringContaining('Application-owned file review'),
      expect.stringContaining('Original ECS-R1 plan'),
      expect.stringContaining('G1 feedback and ECS-R2 replan'),
      expect.stringContaining('WU-ECS2E corrected visual review'),
    ]);
    expect(documents[0]).toHaveTextContent('Provenance: recorded-development');
    expect(documents[3]).toHaveTextContent('Work Unit scope ECS-R4:WU-ECS2E');
    expect(documents[2]).toHaveTextContent('Plan ECS-R2');
  });

  it('focuses the Sprint dialog and restores its opener after Escape closes it', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );

    const opener = screen.getByRole('button', {
      name: 'Open Sprint: Orchestration Orientation Discovery',
    });
    opener.focus();
    fireEvent.click(opener);

    const dialog = screen.getByRole('dialog', { name: 'Orchestration Orientation Discovery' });
    const close = screen.getByRole('button', { name: 'Close Sprint detail' });
    expect(close).toHaveFocus();

    fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(opener).toHaveFocus();
  });

  it('contains forward and reverse Tab focus within the Sprint dialog', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'Open Sprint: Preparation Canvas Recovery and Foundation Acceptance',
      }),
    );

    const close = screen.getByRole('button', { name: 'Close Sprint detail' });
    expect(close).toHaveFocus();
    fireEvent.keyDown(close, { key: 'Tab' });
    expect(close).toHaveFocus();
    fireEvent.keyDown(close, { key: 'Tab', shiftKey: true });
    expect(close).toHaveFocus();
  });

  it('shows a true blocker only beside its affected plan item', () => {
    const base = disposableRecordedOrchestrationView.epics[0];
    const blockedView: OrchestrationSectionView = {
      epics: [
        {
          ...base,
          state: 'blocked',
          plan: {
            items: base.plan.items.map((item) =>
              item.id === 'sprint-orientation'
                ? {
                    ...item,
                    blocker: {
                      id: 'blocker-1',
                      summary: 'Authority choice required',
                      detail: 'Two incompatible product directions remain.',
                      needs: 'Choose the direction before work can continue.',
                    },
                  }
                : item,
            ),
          },
        },
      ],
    };
    render(<OrchestrationSection view={blockedView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );

    const blocker = screen
      .getByText('Blocked: Authority choice required')
      .closest('.sprint-blocker');
    expect(blocker).toHaveTextContent('Blocked: Authority choice required');
    expect(blocker?.closest('li')).toHaveTextContent('Orchestration Orientation Discovery');
    expect(screen.queryByRole('columnheader', { name: /blocker|attention/i })).toBeNull();
  });

  it('expands the latest turn into the reused writable recorded conversation', async () => {
    render(
      <OrchestrationSection
        view={disposableRecordedOrchestrationView}
        agentSessionComposition={recordedComposition()}
      />,
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );

    expect(screen.getByText('Orientation discovery handler')).toBeVisible();
    expect(screen.queryByText(sessionInputText())).toBeNull();
    const sessionToggle = screen.getByRole('button', { name: 'Open Agent Session' });
    expect(sessionToggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();

    fireEvent.click(sessionToggle);
    expect(screen.getByLabelText('Epic Runner Agent Session conversation')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Collapse Agent Session' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
    const message = await screen.findByRole('textbox', { name: 'Message' });
    fireEvent.change(message, { target: { value: 'Record this fixture message' } });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(await screen.findByText('Record this fixture message')).toBeVisible();
    expect(screen.queryByText(/No live agent was invoked/)).toBeNull();
  });

  it('keeps Auto-flow compact, accessible, and local to the fixture projection', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    const hotbar = screen.getByLabelText('Epic controls');
    expect(within(hotbar).getByRole('button', { name: 'Back to Epics' })).toBeVisible();
    const policy = within(hotbar).getByRole('switch', { name: 'Auto-flow' });
    expect(policy).not.toBeChecked();
    expect(within(hotbar).getByText('Auto-flow')).toBeVisible();
    expect(within(hotbar).queryByText('Recorded only · no execution effect')).toBeNull();
    expect(within(hotbar).queryByText('Not eligible')).toBeNull();
    expect(within(hotbar).queryByText('Enabled in fixture')).toBeNull();

    fireEvent.mouseEnter(policy);
    const tooltip = within(hotbar).getByRole('tooltip');
    expect(tooltip).toHaveTextContent(
      'Automatically starts the next Sprint when the current one finishes.',
    );
    expect(policy).toHaveAttribute('aria-describedby', tooltip.id);
    fireEvent.mouseLeave(policy);
    expect(within(hotbar).queryByRole('tooltip')).toBeNull();

    fireEvent.focus(policy);
    expect(within(hotbar).getByRole('tooltip')).toBeVisible();
    fireEvent.blur(policy);
    expect(within(hotbar).queryByRole('tooltip')).toBeNull();

    fireEvent.click(policy);
    expect(policy).not.toBeChecked();
    expect(disposableRecordedOrchestrationView.epics[0].continuation!.automaticEnabled).toBe(false);
    expect(screen.getByRole('status')).toHaveTextContent('unsupported');
    expect(document.querySelector('.continuation-projection')).not.toBeInstanceOf(
      HTMLDetailsElement,
    );
  });

  it('reuses the contained zero-spacing detail composition for Epic and Sprint', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );

    const detail = screen.getByRole('main', { name: 'Epic detail' });
    expect(detail).toHaveClass('detail-workspace');
    expect(detail).toHaveAttribute('data-viewport-contained', 'true');
    const layout = detail.querySelector('.detail-workspace__layout');
    const mainColumn = detail.querySelector('.detail-workspace__main-column');
    expect(layout).not.toBeNull();
    expect(layout?.children).toHaveLength(2);
    expect(mainColumn?.children).toHaveLength(1);
    const context = screen.getByLabelText('Epic context');
    expect(context).toHaveTextContent('Codex Epic Runner workspace development');
    expect(context).toHaveTextContent('Sprint and Epic Detail Review');
    expect(context).toHaveTextContent('In progress');
    expect(document.querySelector('.epic-plan')).not.toBeNull();
    expect(document.querySelector('.shared-agent-session')).not.toBeNull();

    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );
    const sprintDetail = screen.getByRole('main', { name: 'Sprint detail' });
    expect(sprintDetail).toHaveClass('detail-workspace');
    expect(sprintDetail).toHaveAttribute('data-viewport-contained', 'true');
    expect(sprintDetail.querySelector('.detail-workspace__layout')?.children).toHaveLength(2);
    expect(sprintDetail.querySelector('.detail-workspace__main-column')?.children).toHaveLength(1);
    expect(screen.getByLabelText('Sprint context')).toHaveTextContent('Completed');
    expect(screen.getByLabelText('Sprint context')).toHaveTextContent(
      'No recorded Epic Runner Sprint objectives.',
    );
    expect(screen.getByLabelText('Sprint context')).not.toHaveTextContent(
      'Develop Codex Epic Runner',
    );
    expect(screen.getByLabelText('Sprint controls')).not.toHaveTextContent(
      'Codex Epic Runner workspace development',
    );
    expect(sprintDetail.querySelector('.shared-agent-session__compact')).toBeNull();
  });

  it('keeps the WU-ECS2E lifecycle on its Work Slice Planner and one handler', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: /Open Work Unit WU-ECS2E: Plan and Work Unit detail surfaces/,
      }),
    );
    const detail = screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' });
    expect(detail).toHaveTextContent('Plan Work Unit');
    expect(detail).toHaveTextContent('Planner context is recorded separately from agent Activity.');
    expect(detail).toHaveTextContent('Review');
    expect(detail).toHaveTextContent('Acceptance');
    expect(detail).toHaveTextContent('Recorded WU-ECS2E Work Unit Handler');
    expect(detail).not.toHaveTextContent('Reviewer');
    expect(detail).toHaveTextContent('Completed');
  });

  it('presents the in-progress parallel review Sprint and cycles sprintRunnerConcern focus by state', async () => {
    render(<OrchestrationSection view={canonicalRecordedView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint and Epic Detail Review' }),
    );

    const context = screen.getByLabelText('Sprint context');
    expect(context).toHaveTextContent('Processing');
    expect(context).toHaveAttribute('data-scrollable-context', 'true');
    expect(context).toHaveAttribute('tabindex', '0');
    context.focus();
    expect(context).toHaveFocus();
    expect(
      within(context).getByLabelText('Epic Runner objectives').querySelectorAll('li'),
    ).toHaveLength(4);
    expect(within(context).getByLabelText('Epic Runner objectives')).toHaveTextContent(
      'Model explicit relationships between Sprint Runner concerns and planned work.',
    );
    expect(within(context).getByLabelText('Epic Runner objectives')).toHaveTextContent(
      'Open complete Sprint documents with a truthful Sprint-start comparison.',
    );
    expect(context).not.toHaveTextContent('Develop Codex Epic Runner');
    const sprintRunnerConcerns = within(context).getByLabelText('Sprint Runner concerns');
    expect(sprintRunnerConcerns).toBeVisible();
    expect(sprintRunnerConcerns.querySelectorAll('li')).toHaveLength(3);
    expect(screen.getByRole('button', { name: /WU-RD1.*Completed/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD2.*Working/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD3.*Under review/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD4.*Waiting/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD5.*Not started/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD6.*Waiting/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD6/ }).closest('article')).toHaveClass(
      'sprint-work-unit--divergent',
    );

    const sprintRunnerConcern = within(context).getByRole('button', {
      name: 'Keep Epic Runner Sprint objectives while adding Sprint Runner concerns.',
    });
    const processingNode = screen.getByRole('button', { name: /WU-RD2.*Working/ });
    fireEvent.pointerEnter(processingNode);
    expect(sprintRunnerConcern).toHaveClass('is-highlighted');
    fireEvent.pointerLeave(processingNode);
    fireEvent.pointerEnter(sprintRunnerConcern);
    expect(processingNode.closest('article')).toHaveClass('is-runner-concern-highlighted');
    fireEvent.pointerLeave(sprintRunnerConcern);

    fireEvent.click(sprintRunnerConcern);
    await waitFor(() => expect(processingNode).toHaveFocus());
    fireEvent.click(sprintRunnerConcern);
    await waitFor(() =>
      expect(screen.getByRole('button', { name: /WU-RD1.*Completed/ })).toHaveFocus(),
    );
    fireEvent.click(sprintRunnerConcern);
    await waitFor(() =>
      expect(
        screen.getByRole('button', {
          name: 'Open Work Slice planning point: Relationship foundation',
        }),
      ).toHaveFocus(),
    );

    fireEvent.click(screen.getByRole('tab', { name: 'Documents' }));
    expect(screen.getByText('Sprint detail review evidence')).toBeVisible();
  });

  it('navigates the recorded Work Unit lifecycle to Agent Session turns', async () => {
    render(<OrchestrationSection view={canonicalRecordedView} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint and Epic Detail Review' }),
    );
    fireEvent.click(screen.getByRole('button', { name: /Open Work Unit WU-RD1/ }));

    const lifecycle = screen.getByLabelText('Work Unit lifecycle turn log');
    const lifecycleTurns = within(lifecycle).getAllByRole('button');
    expect(lifecycleTurns).toHaveLength(8);
    lifecycleTurns.forEach((turn) => expect(turn).toBeDisabled());
    expect(
      screen.getByText(
        'No exact chronological Activity correlation is available for this Work Unit.',
      ),
    ).toBeVisible();
  });
});

function sessionInputText() {
  return disposableRecordedOrchestrationView.epics[0].epicRunnerSession!.transcript.invocations.at(
    -1,
  )!.submittedText;
}

function recordedComposition() {
  return {
    client: createRecordedAgentSessionClient({
      store: createRecordedAgentSessionStore(recordedAgentSessionDetails),
    }),
    writableSessionIds: new Set(recordedAgentSessionDetails.map(({ session }) => session.id)),
  };
}
