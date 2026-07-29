import { fireEvent, render, screen, within } from '@testing-library/react';
import { vi } from 'vitest';
import { composeProductOrchestrationReadModels } from '../../application/orchestrations';
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
/** Dialog-specific assertions keep non-started/empty Sprint workspace details out of this UI suite. */
const disposableRecordedOrchestrationView = {
  epics: canonicalRecordedView.epics.map((epic) => ({
    ...epic,
    plan: {
      ...epic.plan,
      items: epic.plan.items.map((item) =>
        item.id === 'sprint-control-surface' ? item : { ...item, workspace: undefined },
      ),
    },
  })),
};

describe('OrchestrationSection', () => {
  it('renders truthful empty movement and specific application-owned ready work', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);

    expect(screen.getAllByRole('columnheader').map((header) => header.textContent)).toEqual([
      'Epic',
      'Current movement',
      'State and next action',
    ]);
    expect(screen.getByText('No work in motion')).toBeVisible();
    expect(screen.getByText('Paused')).toBeVisible();
    expect(
      screen.getByRole('button', {
        name: 'Continue with Planner and Work Unit Interaction Discovery',
      }),
    ).toBeVisible();
    expect(screen.queryByText('Ready to continue')).toBeNull();
    expect(screen.queryByText('Human input')).toBeNull();
    expect(screen.queryByRole('columnheader', { name: /attention|continuation/i })).toBeNull();
  });

  it('summarizes typed movement items without display-string inference', () => {
    const target = {
      kind: 'sprint' as const,
      epicId: 'epic',
      sprintId: 'sprint',
      revisionId: 'revision',
    };
    expect(movementLabel({ kind: 'available', items: [] })).toBe('No work in motion');
    expect(
      movementLabel({
        kind: 'available',
        items: [
          { movementItemId: '1', label: 'One', state: 'processing', target },
          { movementItemId: '2', label: 'Two', state: 'processing', target },
          { movementItemId: '3', label: 'Three', state: 'reviewing', target },
        ],
      }),
    ).toBe('2 processing · 1 reviewing');
    expect(movementLabel({ kind: 'unavailable', reason: 'No source.' })).toBe(
      'Unavailable: No source.',
    );
  });

  it('exposes the description from the Epic title on hover and keyboard focus', () => {
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);

    const goal = canonicalRecordedView.epics[0].goal;
    const title = screen.getByRole('button', {
      name: 'Open Codex Epic Runner workspace development',
    });
    const titleArea = title.closest('.epic-title-help');
    expect(titleArea).not.toBeNull();
    expect(titleArea?.querySelectorAll('button')).toHaveLength(1);
    expect(
      screen.queryByRole('button', {
        name: 'About Codex Epic Runner workspace development',
      }),
    ).toBeNull();
    expect(screen.queryByText(goal)).toBeNull();

    fireEvent.mouseEnter(title);
    expect(screen.getByRole('tooltip')).toHaveTextContent(goal);
    expect(title).toHaveAttribute('aria-describedby', screen.getByRole('tooltip').id);
    expect(title.contains(screen.getByRole('tooltip'))).toBe(false);
    fireEvent.mouseLeave(titleArea!);
    expect(screen.queryByRole('tooltip')).toBeNull();

    fireEvent.focus(title);
    expect(screen.getByRole('tooltip')).toHaveTextContent(goal);
    expect(title).toHaveAttribute('aria-describedby', screen.getByRole('tooltip').id);
    fireEvent.blur(title);
    expect(screen.queryByRole('tooltip')).toBeNull();
  });

  it('opens the Epic from both the title and row blank space', () => {
    const first = render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);

    fireEvent.click(
      screen.getByRole('button', {
        name: 'Open Codex Epic Runner workspace development',
      }),
    );
    expect(screen.getByRole('main', { name: 'Epic detail' })).toBeVisible();

    first.unmount();
    render(<OrchestrationSection view={disposableRecordedOrchestrationView} />);

    fireEvent.click(screen.getByText('No work in motion'));
    expect(screen.getByRole('main', { name: 'Epic detail' })).toBeVisible();
  });

  it('keeps planning drafts distinct from initiated Epics in the recorded overview', () => {
    const onOpenPlanningDraft = vi.fn();
    render(
      <OrchestrationSection
        view={disposableRecordedOrchestrationView}
        planningDrafts={[
          {
            epicPlanningDraftId: 'draft-1',
            agentSessionId: 'session-draft-1',
            title: 'Codex Epic Runner workspace development',
            status: 'active',
            createdAt: '2026-07-29T09:00:00.000Z',
            updatedAt: '2026-07-29T09:00:00.000Z',
          },
        ]}
        onOpenPlanningDraft={onOpenPlanningDraft}
      />,
    );

    expect(
      screen.getByRole('button', {
        name: 'Open planning draft Codex Epic Runner workspace development',
      }),
    ).toBeVisible();
    expect(
      screen.getByRole('button', { name: 'Open Codex Epic Runner workspace development' }),
    ).toBeVisible();
    expect(screen.getByText('Pre-initiation planning draft')).toBeVisible();
    fireEvent.click(screen.getByText('Draft'));
    expect(onOpenPlanningDraft).toHaveBeenCalledOnce();
  });

  it('opens movement, ready-work, and human-input targets at their exact product locations', () => {
    const base = disposableRecordedOrchestrationView.epics[0];
    const workUnitTarget = {
      kind: 'work_unit' as const,
      epicId: base.id,
      sprintId: 'sprint-control-surface',
      revisionId: 'ECS-R4',
      sprintPlannerActivityId: 'planner-r4-integration',
      workUnitId: 'WU-ECS2E',
    };
    const plannerTarget = {
      kind: 'sprint_planner_activity' as const,
      epicId: base.id,
      sprintId: 'sprint-control-surface',
      revisionId: 'ECS-R4',
      sprintPlannerActivityId: 'planner-r4-integration',
    };
    const view: OrchestrationSectionView = {
      epics: [
        {
          ...base,
          movement: {
            kind: 'available',
            items: [
              {
                movementItemId: 'processing-plan',
                label: 'Refine the integration plan',
                state: 'processing',
                target: plannerTarget,
              },
              {
                movementItemId: 'review-work-unit',
                label: 'Review WU-ECS2E',
                state: 'reviewing',
                target: workUnitTarget,
              },
            ],
          },
          state: 'running',
          readyWork: [
            {
              actionId: 'open-plan-review',
              label: 'Review the integration plan',
              target: plannerTarget,
            },
          ],
          humanInput: {
            actionId: 'decide-work-unit-review',
            label: 'Decide the WU-ECS2E review',
            target: workUnitTarget,
          },
        },
      ],
    };

    const movementRender = render(<OrchestrationSection view={view} />);
    const movement = screen.getByRole('button', { name: '1 processing · 1 reviewing' });
    expect(screen.getByRole('button', { name: 'Review the integration plan' })).toBeVisible();
    expect(
      screen.getByRole('button', { name: 'Human input required: Decide the WU-ECS2E review' }),
    ).toBeVisible();
    fireEvent.click(movement);
    const popover = screen.getByRole('dialog', { name: 'Current movement details' });
    expect(
      within(popover).getByRole('button', { name: /Refine the integration plan Processing/ }),
    ).toBeVisible();
    const reviewMovement = within(popover).getByRole('button', {
      name: /Review WU-ECS2E Reviewing/,
    });
    reviewMovement.focus();
    fireEvent.keyDown(reviewMovement, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Current movement details' })).toBeNull();
    expect(movement).toHaveFocus();
    fireEvent.click(movement);
    fireEvent.click(
      within(screen.getByRole('dialog', { name: 'Current movement details' })).getByRole('button', {
        name: /Review WU-ECS2E Reviewing/,
      }),
    );
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    movementRender.unmount();

    const readyRender = render(<OrchestrationSection view={view} />);
    fireEvent.click(screen.getByRole('button', { name: 'Review the integration plan' }));
    expect(
      screen.getByRole('main', { name: 'Plan detail: Integrated detail surfaces' }),
    ).toBeVisible();
    readyRender.unmount();

    render(<OrchestrationSection view={view} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Human input required: Decide the WU-ECS2E review' }),
    );
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
  });

  it('makes the ordered plan primary, opens the completed Sprint workspace, and leaves future Sprints inert', () => {
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
    expect(items).toHaveLength(5);
    expect(items[0]).toHaveClass('sprint-plan-item--completed');
    expect(items[1]).toHaveClass('sprint-plan-item--completed');
    expect(items[2]).toHaveClass('sprint-plan-item--completed');
    expect(items[3]).toHaveClass('sprint-plan-item--not_started');
    expect(
      screen.queryByRole('button', { name: /Planner and Work Unit Interaction Discovery/ }),
    ).toBeNull();

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
    expect(screen.getByRole('region', { name: 'Sprint planning workspace' })).toHaveTextContent(
      'Plan flow',
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

  it('starts the Sprint Agent Session collapsed and uses the injected recorded controller', async () => {
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
    expect(session.querySelector('.shared-agent-session__compact')).not.toBeNull();
    expect(session).toHaveTextContent(
      'Agent SessionSprint control surface discoveryOpen Agent Session',
    );
    const open = within(session).getByRole('button', { name: 'Open Agent Session' });
    expect(open).toHaveAttribute('aria-expanded', 'false');
    expect(within(session).queryByLabelText('Sprint Agent Session conversation')).toBeNull();
    expect(screen.queryByLabelText('Epic Runner Agent Session conversation')).toBeNull();

    fireEvent.click(open);
    expect(within(session).getByLabelText('Sprint Agent Session conversation')).toBeVisible();
    const collapse = within(session).getByRole('button', { name: 'Collapse Agent Session' });
    expect(collapse).toHaveAttribute('aria-expanded', 'true');
    const message = await within(session).findByRole('textbox', { name: 'Message' });
    fireEvent.change(message, { target: { value: 'Record Sprint feedback' } });
    fireEvent.click(within(session).getByRole('button', { name: 'Send' }));
    expect(await within(session).findByText('Record Sprint feedback')).toBeVisible();
    expect(within(session).queryByText(/No live agent was invoked/)).toBeNull();

    fireEvent.click(collapse);
    expect(within(session).queryByLabelText('Sprint Agent Session conversation')).toBeNull();
    expect(within(session).getByRole('button', { name: 'Open Agent Session' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
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
      name: 'Open Plan: Integrated detail surfaces',
    });
    planOpener.focus();
    fireEvent.click(planOpener);
    expect(
      screen.getByRole('main', { name: 'Plan detail: Integrated detail surfaces' }),
    ).toBeVisible();
    const workflow = screen.getByLabelText('Plan actor and conversation workflow');
    expect(workflow).toHaveTextContent('Recorded review');
    expect(screen.queryByLabelText('Recorded Plan lifecycle')).toBeNull();
    expect(screen.queryByLabelText(/Integrated detail surfaces flow/)).toBeNull();
    expect(screen.queryByLabelText('Sprint feedback plan change')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    expect(
      screen.getByRole('button', { name: 'Open Plan: Integrated detail surfaces' }),
    ).toHaveFocus();

    const workUnitOpener = screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ });
    fireEvent.click(workUnitOpener);
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    const worker = screen.getByRole('region', { name: 'Implementation worker Agent Session' });
    expect(within(worker).getByLabelText('Implementation worker conversation')).toBeVisible();
    expect(worker).toHaveTextContent('Recorded worker conversation');
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Recorded/theoretical fixture only',
    );
    fireEvent.click(within(worker).getByRole('button', { name: 'Collapse Agent Session' }));
    expect(within(worker).getByRole('button', { name: 'Open Agent Session' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(screen.getByLabelText('Handler and worker Agent Sessions')).toHaveAttribute(
      'data-dominant',
      'handler',
    );

    fireEvent.click(screen.getByRole('button', { name: 'Back to Plan' }));
    expect(
      screen.getByRole('main', { name: 'Plan detail: Integrated detail surfaces' }),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Back to Sprint' })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    expect(screen.getByRole('main', { name: 'Sprint detail' })).toBeVisible();
    expect(screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ })).toHaveFocus();
    expect(screen.queryByRole('main', { name: /Plan detail/ })).toBeNull();
  });

  it('resolves planner and reviewer references through the injected embedded controller path', async () => {
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
    fireEvent.click(screen.getByRole('button', { name: 'Open Plan: Integrated detail surfaces' }));

    const planner = screen.getByRole('region', { name: 'Plan Agent Sessions' });
    expect(
      within(planner).getByRole('region', {
        name: /Recorded planner R4 integration Agent Session/,
      }),
    ).toBeVisible();
    fireEvent.click(within(planner).getByRole('button', { name: 'Open Agent Session' }));
    expect(await within(planner).findByRole('textbox', { name: 'Message' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    fireEvent.click(screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ }));
    const reviewer = screen.getByRole('region', { name: 'Reviewer Agent Session' });
    fireEvent.click(within(reviewer).getByRole('button', { name: 'Open Agent Session' }));
    expect(await within(reviewer).findByRole('textbox', { name: 'Message' })).toBeVisible();
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
    fireEvent.click(screen.getByRole('button', { name: 'Open Plan: Planner activity ECS-R1' }));
    expect(
      screen.getByRole('main', { name: 'Plan detail: Planner activity ECS-R1' }),
    ).toBeVisible();
    expect(screen.getByLabelText('Plan workflow unavailable')).toHaveTextContent(
      'No recorded workflow for this historical Plan',
    );

    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    expect(screen.getByRole('combobox', { name: 'Plan revision' })).toHaveValue('ECS-R1');
    fireEvent.click(screen.getByRole('button', { name: /Open Work Unit WU-ECS2:/ }));
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2' })).toBeVisible();
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Superseded and never launched',
    );
    expect(screen.getByLabelText('Handler / planner fork unavailable')).toBeVisible();
    expect(screen.getByLabelText('Implementation worker unavailable')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Plan' }));
    expect(
      screen.getByRole('main', { name: 'Plan detail: Planner activity ECS-R1' }),
    ).toBeVisible();
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
    expect(within(controls).getByText('Recorded only')).toBeVisible();
    const descriptionId = policy.getAttribute('aria-describedby');
    expect(descriptionId).toBeTruthy();
    expect(document.getElementById(descriptionId!)).toHaveTextContent(
      'accepted child Work Units should start the next planning round',
    );
    expect(document.getElementById(descriptionId!)).toHaveTextContent(
      'does not evaluate eligibility or execute work',
    );

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
    expect(screen.getByRole('region', { name: 'Sprint planning workspace' })).toBeVisible();
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
    expect(within(overview).getAllByRole('button')).toHaveLength(1);
    expect(within(overview).getByText('Responsibility accepted')).toBeVisible();
    const multiUnitConcern = within(overview).getByRole('button', {
      name: /Sprint control surface.*7 linked Work Units/,
    });
    expect(multiUnitConcern).toBeVisible();

    const convergence = multiUnitConcern;
    convergence.focus();
    fireEvent.click(convergence);
    expect(screen.getByLabelText('Concern detail: Sprint control surface')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back to concerns' }));
    expect(screen.getByRole('button', { name: /Sprint control surface/ })).toHaveFocus();

    fireEvent.click(screen.getByRole('button', { name: /Sprint control surface/ }));
    const detail = screen.getByLabelText('Concern detail: Sprint control surface');
    const unit = within(detail).getByRole('button', { name: /WU-ECS2E/ });
    fireEvent.click(unit);
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back to Concern' }));
    expect(screen.getByLabelText('Concern detail: Sprint control surface')).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-ECS2E/ })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Back to concerns' }));
    expect(screen.getByRole('button', { name: /Sprint control surface/ })).toHaveFocus();
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
    fireEvent.click(screen.getByRole('button', { name: /Sprint control surface/ }));
    const concern = screen.getByLabelText('Concern detail: Sprint control surface');
    const activeUnit = within(concern).getByRole('button', { name: /WU-ECS2E/ });
    fireEvent.click(activeUnit);

    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back to Concern' }));
    expect(screen.getByRole('button', { name: /WU-ECS2E/ })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Back to concerns' }));
    expect(screen.getByRole('button', { name: /Sprint control surface/ })).toHaveFocus();

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
      expect.stringContaining('Original ECS-R1 plan'),
      expect.stringContaining('G1 feedback and ECS-R2 replan'),
      expect.stringContaining('WU-ECS2E corrected visual review'),
    ]);
    expect(documents[0]).toHaveTextContent('Provenance: recorded-development');
    expect(documents[2]).toHaveTextContent('Work Unit scope ECS-R4:WU-ECS2E');
    expect(documents[1]).toHaveTextContent('Plan ECS-R2');
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
    expect(mainColumn?.children).toHaveLength(2);
    const context = screen.getByLabelText('Epic context');
    expect(context).toHaveTextContent('Codex Epic Runner workspace development');
    expect(context).toHaveTextContent('Sprint Control Surface Discovery');
    expect(context).toHaveTextContent('Completed');
    expect(document.querySelector('.epic-plan')).not.toBeNull();
    expect(document.querySelector('.shared-agent-session')).not.toBeNull();

    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );
    const sprintDetail = screen.getByRole('main', { name: 'Sprint detail' });
    expect(sprintDetail).toHaveClass('detail-workspace');
    expect(sprintDetail).toHaveAttribute('data-viewport-contained', 'true');
    expect(sprintDetail.querySelector('.detail-workspace__layout')?.children).toHaveLength(2);
    expect(sprintDetail.querySelector('.detail-workspace__main-column')?.children).toHaveLength(2);
    expect(screen.getByLabelText('Sprint context').querySelectorAll('h1, p')).toHaveLength(2);
    expect(screen.getByLabelText('Sprint controls')).not.toHaveTextContent(
      'Codex Epic Runner workspace development',
    );
    expect(sprintDetail.querySelector('.shared-agent-session__compact')).not.toBeNull();
  });

  it('shows both canonical ECS2E attempts and their recorded review outcomes', () => {
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
    expect(detail).toHaveTextContent('WU-ECS2E-attempt-1Returned · needs correction');
    expect(detail).toHaveTextContent('WU-ECS2E-attempt-2Returned · accepted');
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
