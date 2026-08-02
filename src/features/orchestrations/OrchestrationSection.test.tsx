import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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
    expect(screen.getByLabelText('Managed Sprint objectives')).toHaveTextContent(
      'Clarify temporal planning and parallel implementation concerns.',
    );
    expect(screen.getByLabelText('Forecast task breakdown')).toHaveTextContent(
      'Explore planner-to-parallel-work relationship boundaries.',
    );
    expect(screen.queryByRole('tablist', { name: 'Sprint information' })).toBeNull();
    expect(screen.queryByRole('button', { name: /Open Work Unit/ })).toBeNull();
    expect(screen.queryByLabelText('Work Slice causal timeline')).toBeNull();
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
    const workflow = screen.getByLabelText('Work Slice causal timeline');
    expect(workflow).toHaveTextContent('Parallel Work Unit flow');
    expect(workflow).toHaveTextContent('Handler: Recorded WU-ECS2E Work Unit Handler');
    expect(workflow).toHaveTextContent('Worker: Recorded WU-ECS2E Work Unit Implementer');
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
    const handler = screen.getByRole('region', { name: 'Work Unit Handler Agent Session' });
    expect(within(handler).getByLabelText('Work Unit Handler conversation')).toBeVisible();
    expect(
      screen.getByRole('region', { name: 'Work Unit Implementer Agent Session' }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Work Slice Planner' }));
    const plannerSession = screen.getByRole('region', {
      name: 'Work Slice Planner Agent Session',
    });
    expect(within(plannerSession).getByLabelText('Work Slice Planner conversation')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Work Unit Handler' }));
    expect(handler).toHaveTextContent('Reviewed the first return');
    expect(
      screen.getByRole('button', { name: /Plan Work Unit.*Recorded planner R4 integration/ }),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: /Acceptance.*Work Unit Handler/ })).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Reviewer' })).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Reviewer' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Reviewer Agent Session' })).toBeNull();
    expect(screen.getByLabelText('Work Unit context')).not.toHaveTextContent(
      'Recorded/theoretical fixture only',
    );
    expect(
      screen.getByRole('separator', {
        name: 'Resize Planning and handling conversation and Work Unit Implementer conversation',
      }),
    ).toHaveAttribute('aria-orientation', 'vertical');
    expect(within(handler).queryByRole('button', { name: /Collapse/ })).toBeNull();

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
    expect(await within(planner).findByRole('textbox', { name: 'Message' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Sprint' }));
    fireEvent.click(screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ }));
    const handler = screen.getByRole('region', { name: 'Work Unit Handler Agent Session' });
    expect(await within(handler).findByRole('textbox', { name: 'Message' })).toBeVisible();
    expect(
      screen.getByRole('region', { name: 'Work Unit Implementer Agent Session' }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Work Slice Planner' }));
    expect(screen.getByRole('region', { name: 'Work Slice Planner Agent Session' })).toBeVisible();
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
    expect(
      screen.getByRole('button', {
        name: 'Open Work Slice planning point: Planning point ECS-R1',
      }),
    ).toBeDisabled();
    expect(screen.getByRole('combobox', { name: 'Plan revision' })).toHaveValue('ECS-R1');
    fireEvent.click(screen.getByRole('button', { name: /Open Work Unit WU-ECS2:/ }));
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-ECS2' })).toBeVisible();
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Superseded and never launched',
    );
    expect(screen.getByLabelText('Work Unit Handler unavailable')).toBeVisible();
    expect(screen.getByLabelText('Work Unit Implementer unavailable')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Work Slice planning point' }));
    expect(screen.getByRole('main', { name: 'Sprint detail' })).toBeVisible();
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
    const relationships = screen.getByLabelText('Work Slice causal timeline');
    expect(screen.getByLabelText('Work Slice Planner origin')).toHaveTextContent('Priya');
    expect(within(relationships).getByRole('article', { name: /Work Unit WU-RD1:/ })).toBeVisible();
    expect(within(relationships).getByRole('article', { name: /Work Unit WU-RD5:/ })).toBeVisible();
    expect(within(relationships).getAllByRole('article')).toHaveLength(5);
    const rd1 = within(relationships).getByRole('article', { name: /Work Unit WU-RD1:/ });
    expect(
      within(rd1).getByRole('button', { name: /Open WU-RD1 Handler lifecycle/ }),
    ).toHaveTextContent('Handler: RowanApproved and integrated');
    expect(
      within(rd1).getByRole('button', { name: /Open WU-RD1 Worker lifecycle/ }),
    ).toHaveTextContent('Worker: MinaCompleted');
    expect(within(relationships).getByLabelText('WU-RD5 Handler unavailable')).toBeVisible();
    expect(within(relationships).getByLabelText('WU-RD5 Worker unavailable')).toBeVisible();
    const independentPrerequisites = relationships.querySelector(
      'svg g[data-join-semantics="independent_prerequisites"]',
    );
    expect(independentPrerequisites).toHaveAttribute(
      'data-input-scope-ids',
      'RD-R2:WU-RD2 RD-R2:WU-RD3',
    );
    expect(independentPrerequisites?.querySelectorAll('[data-prerequisite-input]')).toHaveLength(2);
    expect(
      independentPrerequisites?.querySelector('[data-prerequisite-input="RD-R2:WU-RD1"]'),
    ).toBeNull();
    expect(
      relationships.querySelector(
        'svg g[data-join-semantics="merged_result"] [data-geometry="merged-output"]',
      ),
    ).toHaveAttribute('marker-end', 'url(#merged-output-arrow)');
    expect(screen.queryByLabelText('Detailed workflow unavailable')).toBeNull();
    expect(screen.queryByText(/historical Plan/)).toBeNull();

    fireEvent.click(
      within(rd1).getByRole('button', { name: /Open WU-RD1 Handler lifecycle at approved/ }),
    );
    expect(screen.getByRole('main', { name: 'Work Unit detail: WU-RD1' })).toBeVisible();
    expect(screen.getByRole('button', { name: /Completion/ })).toHaveAttribute(
      'aria-current',
      'step',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Work Slice Planner' }));
    expect(screen.getByRole('region', { name: 'Work Slice Planner Agent Session' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Work Unit Handler' }));
    expect(screen.getByRole('region', { name: 'Work Unit Handler Agent Session' })).toBeVisible();
    expect(
      screen.getByRole('region', { name: 'Work Unit Implementer Agent Session' }),
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
      'No sourced managed Sprint objectives are available.',
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
    expect(detail).toHaveTextContent('Recorded planner R4 integration');
    expect(detail).toHaveTextContent('Review');
    expect(detail).toHaveTextContent('Acceptance');
    expect(detail).toHaveTextContent('Recorded WU-ECS2E Work Unit Handler');
    expect(detail).not.toHaveTextContent('Reviewer');
    expect(detail).toHaveTextContent('Completed');
  });

  it('presents the managed in-progress Sprint plan and cycles objective focus', async () => {
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
    const objectives = within(context).getByLabelText('Managed Sprint objectives');
    expect(within(objectives).getAllByRole('button')).toHaveLength(3);
    expect(objectives).toHaveTextContent('Make typed product relationships directly reviewable.');
    expect(objectives).toHaveTextContent(
      'Keep lifecycle and document evidence connected to managed work.',
    );
    expect(context).not.toHaveTextContent('Develop Codex Epic Runner');
    expect(screen.getByRole('button', { name: /WU-RD1.*Completed/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD2.*Working/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD3.*Under review/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD4.*Waiting/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD5.*Not started/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD6.*Waiting/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /WU-RD6/ }).closest('article')).toHaveClass(
      'sprint-work-unit--divergent',
    );

    const managedObjective = within(context).getByRole('button', {
      name: /Make typed product relationships directly reviewable/,
    });
    const processingNode = screen.getByRole('button', { name: /WU-RD2.*Working/ });
    fireEvent.pointerEnter(processingNode);
    expect(managedObjective).toHaveClass('is-highlighted');
    fireEvent.pointerLeave(processingNode);
    fireEvent.pointerEnter(managedObjective);
    expect(processingNode.closest('article')).toHaveClass('is-managed-objective-highlighted');
    fireEvent.pointerLeave(managedObjective);

    fireEvent.click(managedObjective);
    await waitFor(() =>
      expect(
        screen.getByRole('button', {
          name: 'Open Work Slice planning point: Relationship foundation',
        }),
      ).toHaveFocus(),
    );
    fireEvent.click(managedObjective);
    await waitFor(() => expect(processingNode).toHaveFocus());
    fireEvent.click(managedObjective);
    await waitFor(() =>
      expect(screen.getByRole('button', { name: /WU-RD1.*Completed/ })).toHaveFocus(),
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
    expect(within(lifecycle).getAllByRole('button')).toHaveLength(9);
    fireEvent.click(within(lifecycle).getByRole('button', { name: /Plan Work Unit/ }));
    expect(screen.getByRole('region', { name: 'Work Slice Planner Agent Session' })).toBeVisible();
    await waitFor(() =>
      expect(
        document.querySelector('[data-invocation-id="recorded-planner-rd-r2-scope"]'),
      ).toHaveFocus(),
    );
    fireEvent.click(within(lifecycle).getByRole('button', { name: /Reprompt/ }));
    await waitFor(() =>
      expect(
        document.querySelector('[data-invocation-id="recorded-handler-WU-RD1-reprompt"]'),
      ).toHaveFocus(),
    );

    fireEvent.click(within(lifecycle).getAllByRole('button', { name: /^Review/ })[1]);
    expect(screen.getByRole('region', { name: 'Work Unit Handler Agent Session' })).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Reviewer Agent Session' })).toBeNull();
    await waitFor(() =>
      expect(
        document.querySelector('[data-invocation-id="recorded-handler-WU-RD1-second-review"]'),
      ).toHaveFocus(),
    );
    expect(
      screen.getByRole('separator', {
        name: 'Resize Planning and handling conversation and Work Unit Implementer conversation',
      }),
    ).toHaveAttribute('aria-orientation', 'vertical');
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
