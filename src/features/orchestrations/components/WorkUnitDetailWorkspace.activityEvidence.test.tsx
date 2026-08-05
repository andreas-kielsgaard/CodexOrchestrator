import { readFileSync } from 'node:fs';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import { projectAgentSessionTranscript } from '../../agentSessions/transcriptProjector';
import { runtimeEvent, sessionDetails } from '../../agentSessions/testFixtures';
import type { WorkUnitAgentSessionPresentation } from '../orchestrationModel';
import { WorkUnitDetailWorkspace } from './WorkUnitDetailWorkspace';

type PresentedWorkUnit =
  SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];

describe('WorkUnitDetailWorkspace Activity and Evidence', () => {
  it('selects exact agent turns, nests application detail, and navigates from Evidence', async () => {
    const user = userEvent.setup();
    render(<Workspace />);

    expect(screen.getByRole('tab', { name: 'Activity' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('Application summary')).toBeInTheDocument();
    expect(screen.getByLabelText('Application summary')).toHaveTextContent(
      'No application-owned MCP-call detail is available.',
    );
    expect(screen.queryByLabelText('Selected Agent Session turn')).toBeNull();

    await user.click(screen.getByRole('button', { name: /Review delivery recorded invocation-1/ }));
    expect(
      screen.getByRole('button', { name: /Review delivery recorded invocation-1/ }),
    ).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByLabelText('Agent Session turn: invocation-1')).toHaveTextContent(
      'Do the work',
    );
    expect(screen.getByLabelText('Agent Session turn: invocation-1')).toHaveTextContent(
      'Safe final response',
    );

    await user.click(screen.getByRole('tab', { name: 'Evidence' }));
    expect(screen.getByRole('tab', { name: 'Evidence' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('src/feature.ts')).toBeInTheDocument();
    const unavailableFile = screen.getByText('src/feature.ts').closest('li');
    expect(unavailableFile).toHaveClass('work-unit-file-evidence--unavailable');
    expect(unavailableFile).toHaveAttribute('data-evidence-id', 'evidence-1');
    expect(unavailableFile).toHaveAttribute('data-file-id', 'file-1');
    expect(unavailableFile).toHaveAttribute('data-source-activity-id', 'activity-implementer');
    expect(unavailableFile?.querySelector('button')).toBeNull();
    expect(screen.getByText('Unavailable diff')).toBeInTheDocument();
    expect(
      screen.getByText('Unavailable: No application-owned test detail is available.'),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText('Selected Agent Session turn')).toBeNull();

    await user.click(screen.getByRole('button', { name: 'View owning activity' }));
    expect(screen.getByRole('tab', { name: 'Activity' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByLabelText('Agent Session turn: invocation-2')).toBeInTheDocument();
  });

  it('fails closed when an activity points at a missing Session', async () => {
    const user = userEvent.setup();
    render(<Workspace missingSession />);

    expect(screen.getByLabelText('Chronological activity records')).not.toHaveTextContent(
      'invocation-2',
    );
    expect(screen.getByLabelText('Unsequenced activity records')).toHaveTextContent('invocation-2');
    await user.click(
      screen.getByRole('button', { name: /Recorded activity detail unavailable invocation-2/ }),
    );
    expect(screen.getByLabelText('Agent Session turn: invocation-2')).toHaveTextContent(
      'Agent Session turn unavailable',
    );
  });

  it('moves between peer views with the tab keyboard contract', async () => {
    const user = userEvent.setup();
    render(<Workspace />);

    const activityTab = screen.getByRole('tab', { name: 'Activity' });
    activityTab.focus();
    await user.keyboard('{ArrowRight}');

    expect(screen.getByRole('tab', { name: 'Evidence' })).toHaveAttribute('aria-selected', 'true');
    expect(document.activeElement).toBe(screen.getByRole('tab', { name: 'Evidence' }));
  });

  it('highlights only the exactly correlated Lifecycle step when an Activity receives focus', () => {
    render(<Workspace />);

    const activity = screen.getByRole('button', { name: /Review delivery recorded invocation-1/ });
    fireEvent.focus(activity);

    expect(
      screen
        .getByRole('button', { name: /Review delivery recorded.*Work Unit Handler/ })
        .closest('li'),
    ).toHaveClass('is-highlighted');
    expect(screen.getByText('Planner record').closest('section')).toHaveAttribute(
      'aria-label',
      'Planner context',
    );
  });

  it('highlights and selects an exact Activity from Lifecycle hover, focus, and click', async () => {
    const user = userEvent.setup();
    render(<Workspace />);

    const lifecycle = screen.getByRole('button', {
      name: /Review delivery recorded.*Work Unit Handler/,
    });
    const activity = screen.getByRole('button', { name: /Review delivery recorded invocation-1/ });

    fireEvent.mouseEnter(lifecycle);
    expect(activity.closest('li')).toHaveClass('is-highlighted');
    fireEvent.mouseLeave(lifecycle);
    expect(activity.closest('li')).not.toHaveClass('is-highlighted');

    fireEvent.focus(lifecycle);
    expect(activity.closest('li')).toHaveClass('is-highlighted');
    await user.click(lifecycle);
    expect(activity).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByLabelText('Agent Session turn: invocation-1')).toBeVisible();
  });

  it('fails closed for a Lifecycle entry with a mismatched typed agent role', () => {
    render(<Workspace mismatchedLifecycle />);

    const activity = screen.getByRole('button', { name: /Review delivery recorded invocation-1/ });
    const lifecycle = screen
      .getByLabelText('Work Unit lifecycle turn log')
      .querySelector<HTMLButtonElement>('ol > li > button');

    expect(lifecycle).toBeDisabled();
    fireEvent.focus(activity);
    expect(lifecycle?.closest('li')).not.toHaveClass('is-highlighted');
    expect(screen.getByLabelText('Unsequenced activity records')).toHaveTextContent(
      'Review delivery recorded',
    );
  });

  it('keeps duplicate Lifecycle correlations out of chronological Activity', () => {
    render(<Workspace duplicateLifecycle />);

    expect(screen.queryByLabelText('Chronological activity records')).toBeNull();
    expect(screen.getByLabelText('Unsequenced activity records')).toHaveTextContent(
      'Review delivery recorded',
    );
    for (const lifecycle of screen
      .getByLabelText('Work Unit lifecycle turn log')
      .querySelectorAll<HTMLButtonElement>('ol > li > button')) {
      expect(lifecycle).toBeDisabled();
    }
  });

  it('anchors the Lifecycle trace to the shared identity-center track', () => {
    const styles = readFileSync(
      'src/features/orchestrations/styles/orchestrationSubdetail.css',
      'utf8',
    );

    expect(styles).toContain('--work-unit-lifecycle-identity-size: 28px;');
    expect(styles).toContain('--work-unit-lifecycle-identity-center: calc(');
    expect(styles).toContain('left: var(--work-unit-lifecycle-identity-center);');
    expect(styles).toContain('transform: translateX(-50%);');
  });

  it('fails closed for a stale requested Activity restore state', () => {
    render(
      <Workspace
        initialInspectionState={{
          tab: 'activity',
          activityId: 'activity-handler',
          sessionId: 'session-1',
          invocationId: 'foreign-invocation',
        }}
      />,
    );

    expect(screen.queryByLabelText('Agent Session turn: invocation-1')).toBeNull();
    expect(
      screen.getByText('Select an activity to inspect its complete recorded turn.'),
    ).toBeVisible();
  });
});

function Workspace({
  missingSession = false,
  mismatchedLifecycle = false,
  duplicateLifecycle = false,
  initialInspectionState,
}: {
  readonly missingSession?: boolean;
  readonly mismatchedLifecycle?: boolean;
  readonly duplicateLifecycle?: boolean;
  readonly initialInspectionState?: {
    readonly tab: 'activity' | 'evidence';
    readonly activityId: string;
    readonly sessionId: string;
    readonly invocationId: string;
  };
}) {
  const inspection = {
    workUnitId: 'unit-1',
    materializationId: 'materialization-1',
    activities: [
      {
        activityId: 'activity-handler',
        attemptId: 'attempt-1',
        role: 'handler' as const,
        agentSessionId: 'session-1',
        invocationId: 'invocation-1',
        primaryStage: 'handler_action' as const,
        applicationSummary: {
          owner: 'application' as const,
          applicationEvents: ['review_delivery_persisted' as const],
          peerEvidenceActivityIds: [],
          mcpCallDetail: {
            owner: 'application' as const,
            reason: 'No application-owned MCP-call detail is available.',
          },
        },
      },
      {
        activityId: 'activity-implementer',
        attemptId: 'attempt-1',
        role: 'implementer' as const,
        agentSessionId: missingSession ? 'missing-session' : 'session-2',
        invocationId: 'invocation-2',
        primaryStage: 'implementer_reporting' as const,
      },
    ],
    fileEvidence: {
      status: 'available' as const,
      owner: 'application' as const,
      sourceActivityId: 'activity-implementer',
      changedFiles: [
        {
          evidenceRef: 'evidence-1',
          fileId: 'file-1',
          displayName: 'src/feature.ts',
          changeKind: 'modified' as const,
          contentFingerprint: 'content-1',
          diffDestination: {
            status: 'unavailable' as const,
            owner: 'application' as const,
            reason: 'No application-owned diff destination is available.',
          },
        },
      ],
    },
    testEvidence: {
      status: 'unavailable' as const,
      owner: 'application' as const,
      reason: 'No application-owned test detail is available.',
    },
  };
  const handlerLifecycle: SprintWorkspacePresentationV1['workUnitLifecycle'][number] = {
    entryId: 'handler-entry',
    sprintId: 'sprint-1',
    workUnitId: 'unit-1',
    sequence: 1,
    kind: 'review',
    title: 'Handler work',
    summary: 'The exact Handler Activity is recorded.',
    agentSessionId: 'session-1',
    agentRole: mismatchedLifecycle ? 'work_unit_implementer' : 'work_unit_handler',
    invocationId: 'invocation-1',
    source: {
      status: 'available',
      sourceKind: 'repository',
      sourceReferences: ['materialization-1'],
    },
  };
  return (
    <WorkUnitDetailWorkspace
      unit={{ ...unit(), inspection }}
      lifecycleEntries={[
        {
          entryId: 'planner-entry',
          sprintId: 'sprint-1',
          workUnitId: 'unit-1',
          sequence: 0,
          kind: 'planning',
          title: 'Planner record',
          summary: 'No primary Activity is recorded for this Planner lifecycle entry.',
          agentSessionId: 'planner-session',
          agentRole: 'work_slice_planner',
          invocationId: 'planner-invocation',
          source: {
            status: 'available',
            sourceKind: 'repository',
            sourceReferences: ['materialization-1'],
          },
        },
        handlerLifecycle,
        ...(duplicateLifecycle
          ? [{ ...handlerLifecycle, entryId: 'handler-entry-duplicate', sequence: 2 }]
          : []),
      ]}
      workSlicePlanningPointGroupTitle="Planning point"
      sessions={missingSession ? [handlerSession()] : [handlerSession(), implementerSession()]}
      onBack={vi.fn()}
      initialInspectionState={initialInspectionState}
    />
  );
}

function unit(): PresentedWorkUnit {
  return {
    workUnitId: 'unit-1',
    title: 'Bounded responsibility',
    summary: 'Implement one bounded change.',
    details: 'Accepted Work Slice responsibility.',
    source: {
      status: 'available',
      sourceKind: 'repository',
      sourceReferences: ['materialization-1'],
    },
    attemptHistory: [],
    retryAttempts: [],
    workUnitScopeId: 'scope-1',
    sprintPlanRevisionId: 'revision-1',
    fixedExecutionScopeIds: [],
    dependencies: [],
    gateIds: [],
    attempts: [],
    reviews: [],
    observed: {
      executionRequested: false,
      launched: false,
      returned: false,
      integrated: false,
      responsibilityAccepted: false,
    },
    presentationState: 'not_started',
  };
}

function handlerSession(): WorkUnitAgentSessionPresentation {
  return {
    sessionId: 'session-1',
    title: 'Work Unit Handler',
    workUnitId: 'unit-1',
    role: 'handler',
    transcript: projectAgentSessionTranscript(
      sessionDetails('completed', [
        runtimeEvent(1, 'agent_message', 'Safe final response', { role: 'final' }),
      ]),
    ),
  };
}

function implementerSession(): WorkUnitAgentSessionPresentation {
  const details = sessionDetails('completed', [
    runtimeEvent(1, 'agent_message', 'Implementer final response', { role: 'final' }),
  ]);
  details.session.id = 'session-2';
  details.invocations[0]!.invocation.id = 'invocation-2';
  details.invocations[0]!.invocation.sessionId = 'session-2';
  details.invocations[0]!.events[0]!.invocationId = 'invocation-2';
  return {
    sessionId: 'session-2',
    title: 'Work Unit Implementer',
    workUnitId: 'unit-1',
    role: 'implementer',
    transcript: projectAgentSessionTranscript(details),
  };
}
