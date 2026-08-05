import { useEffect, useState } from 'react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import {
  AgentSessionTurnInspector,
  type EmbeddedAgentSessionComposition,
} from '../../agentSessions';
import type { WorkUnitAgentSessionPresentation } from '../orchestrationModel';
import { DetailWorkspace } from './DetailWorkspace';
import type {
  ProductWorkUnitHandlerDecisionV1,
  ProductWorkUnitHandlerReviewV1,
  ProductWorkUnitIntegrationV1,
  ProductWorkUnitIncompleteDispositionV1,
  ProductWorkUnitImplementerOutcomeV1,
  ProductWorkUnitInspectionActivityV1,
  ProductWorkUnitInspectionV1,
  ProductWorkUnitRetryAttemptV1,
} from '../../../application/orchestrations/productReadModels';
import '../styles/orchestrationSubdetail.css';
import type { ReactNode } from 'react';

export interface WorkUnitDetailWorkspaceProps {
  readonly unit: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];
  readonly lifecycleEntries: SprintWorkspacePresentationV1['workUnitLifecycle'];
  readonly workSlicePlanningPointGroupTitle: string;
  readonly sessions: readonly WorkUnitAgentSessionPresentation[];
  /** Retained for callers during the read-only migration; this detail renders no embedded workspace. */
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly backLabel?: string;
  readonly onBack: () => void;
  readonly onOpenActivitySession?: (target: WorkUnitActivitySessionTarget) => void;
  readonly onOpenFileEvidence?: (target: WorkUnitFileEvidenceTarget) => void;
  readonly initialInspectionState?: WorkUnitInspectionState;
  readonly sprintControl?: ReactNode;
}

export interface WorkUnitActivitySessionTarget {
  readonly sessionId: string;
  readonly invocationId: string;
  readonly activityId: string;
}

export interface WorkUnitFileEvidenceTarget {
  readonly reviewId: string;
  readonly changedFileId: string;
}

export interface WorkUnitInspectionState {
  readonly tab: WorkUnitInspectionTab;
  readonly activityId: string;
  readonly sessionId: string;
  readonly invocationId: string;
}

type WorkUnitInspectionTab = 'activity' | 'evidence';

export function WorkUnitDetailWorkspace({
  unit,
  lifecycleEntries,
  workSlicePlanningPointGroupTitle,
  sessions,
  backLabel = 'Back to Work Slice planning point',
  onBack,
  onOpenActivitySession,
  onOpenFileEvidence,
  initialInspectionState,
  sprintControl,
}: WorkUnitDetailWorkspaceProps) {
  const workUnitId = unit.workUnitId;
  const validInitialState =
    initialInspectionState &&
    unit.inspection?.activities.some(
      (activity) =>
        activity.activityId === initialInspectionState.activityId &&
        activity.agentSessionId === initialInspectionState.sessionId &&
        activity.invocationId === initialInspectionState.invocationId,
    )
      ? initialInspectionState
      : undefined;
  const [inspectionTab, setInspectionTab] = useState<WorkUnitInspectionTab>(
    validInitialState?.tab ?? 'activity',
  );
  const [selectedActivityId, setSelectedActivityId] = useState<string | null>(
    validInitialState?.activityId ?? null,
  );
  const [highlightedActivityId, setHighlightedActivityId] = useState<string | null>(null);
  useEffect(() => {
    if (!validInitialState) return;
    setInspectionTab(validInitialState.tab);
    setSelectedActivityId(validInitialState.activityId);
  }, [validInitialState]);
  const attemptHistory = unit.attemptHistory ?? [];
  const retryAttempts = unit.retryAttempts ?? [];
  const activityOrdinals = [
    ...new Set([
      ...attemptHistory.map((attempt) => attempt.ordinal),
      ...retryAttempts.map((retry) => retry.ordinal),
    ]),
  ].sort((left, right) => left - right);

  const navigateToLifecycleTurn = (
    entry: SprintWorkspacePresentationV1['workUnitLifecycle'][number],
  ) => {
    const activity = unit.inspection?.activities.find(
      (candidate) =>
        candidate.agentSessionId === entry.agentSessionId &&
        candidate.invocationId === entry.invocationId,
    );
    const session = sessions.find(({ sessionId }) => sessionId === entry.agentSessionId);
    if (activity && session && roleMatchesLifecycle(activity.role, entry.agentRole)) {
      setInspectionTab('activity');
      setSelectedActivityId(activity.activityId);
    }
  };

  return (
    <DetailWorkspace
      ariaLabel={`Work Unit detail: ${workUnitId}`}
      controlsLabel="Work Unit controls"
      contextLabel="Work Unit context"
      backLabel={backLabel}
      onBack={onBack}
      focusBackOnMount
      hotbarContext={workSlicePlanningPointGroupTitle}
      control={
        <div className="sprint-header-controls">
          {sprintControl}
          <span className={`work-unit-state work-unit-state--${unit.presentationState}`}>
            <small>Current work</small>
            <strong>{workUnitStatusLabel(unit.presentationState)}</strong>
          </span>
        </div>
      }
      context={
        <div className="subdetail-context">
          <p className="eyebrow">Work Unit</p>
          <code>{unit.workUnitId}</code>
          <h1>{unit.title}</h1>
          <p>{unit.summary}</p>
          <p>{unit.details}</p>
          {unit.executionState && (
            <section
              className="work-unit-execution-progress"
              aria-label="Work Unit execution progress"
            >
              <h2>Execution progress</h2>
              <p>{executionStateDetail(unit.executionState.state)}</p>
            </section>
          )}
          {(unit.handlerActivation ||
            unit.actionContinuation ||
            unit.implementerActivation ||
            attemptHistory.length > 0 ||
            retryAttempts.length > 0 ||
            unit.integration ||
            unit.dependencyActivationIntent) && (
            <section className="work-unit-activation" aria-label="Work Unit activation activity">
              <h2>Activation activity</h2>
              {unit.handlerActivation && <p>{handlerActivity(unit.handlerActivation)}</p>}
              {unit.actionContinuation && (
                <p>{actionContinuationActivity(unit.actionContinuation)}</p>
              )}
              {unit.implementerActivation && (
                <p>{implementerActivity(unit.implementerActivation)}</p>
              )}
              {activityOrdinals.map((ordinal) => {
                const attempt = attemptHistory.find((member) => member.ordinal === ordinal);
                return (
                  <section className="work-unit-attempt" key={`attempt-${ordinal}`}>
                    <h3>Attempt ordinal {ordinal}</h3>
                    {attempt?.implementerOutcome && (
                      <ImplementerOutcomeActivity outcome={attempt.implementerOutcome} />
                    )}
                    {attempt?.handlerReview && (
                      <HandlerReviewActivity
                        review={attempt.handlerReview}
                        decision={attempt.handlerDecision}
                      />
                    )}
                    {attempt?.incompleteDisposition && (
                      <IncompleteDispositionActivity disposition={attempt.incompleteDisposition} />
                    )}
                    {retryAttempts
                      .filter((retry) => retry.ordinal === ordinal)
                      .map((retry) => (
                        <RetryAttemptActivity key={retry.retryAttemptId} retryAttempt={retry} />
                      ))}
                  </section>
                );
              })}
              {unit.integration && <IntegrationActivity integration={unit.integration} />}
              {unit.dependencyActivationIntent && (
                <div className="work-unit-dependency-activation">
                  <h3>Dependent activation</h3>
                  <p>
                    {unit.dependencyActivationIntent.eligibilityState === 'blocked'
                      ? 'Dependent activation remains blocked.'
                      : unit.dependencyActivationIntent.activationIntendedAt
                        ? 'Dependencies are eligible and Handler activation intent is durably recorded.'
                        : 'Dependencies are eligible; Handler activation intent is not yet recorded.'}
                  </p>
                  {unit.dependencyActivationIntent.blockedReason && (
                    <p>Reason: {unit.dependencyActivationIntent.blockedReason}.</p>
                  )}
                </div>
              )}
            </section>
          )}
          <section className="work-unit-lifecycle" aria-label="Work Unit lifecycle turn log">
            <h2>Lifecycle</h2>
            {lifecycleEntries.length ? (
              <ol>
                {lifecycleEntries.map((entry) => {
                  const activity = unit.inspection?.activities.find(
                    (candidate) =>
                      candidate.agentSessionId === entry.agentSessionId &&
                      candidate.invocationId === entry.invocationId,
                  );
                  const session = sessions.find(
                    ({ sessionId }) => sessionId === entry.agentSessionId,
                  );
                  const correlated = Boolean(
                    activity && session && roleMatchesLifecycle(activity.role, entry.agentRole),
                  );
                  return (
                    <li
                      key={entry.entryId}
                      className={
                        highlightedActivityId === activity?.activityId
                          ? 'is-highlighted'
                          : undefined
                      }
                    >
                      <button
                        type="button"
                        onClick={() => navigateToLifecycleTurn(entry)}
                        disabled={!correlated}
                      >
                        <span
                          className={`work-unit-lifecycle__identity work-unit-lifecycle__identity--${entry.agentRole}`}
                          aria-hidden="true"
                        >
                          {agentInitial(entry.agentRole)}
                        </span>
                        <span>
                          <strong>{entry.title}</strong>
                          <small>{session?.title ?? 'Recorded Agent Session unavailable'}</small>
                          <span>{entry.summary}</span>
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ol>
            ) : (
              <p>No recorded lifecycle turn links are available for this Work Unit.</p>
            )}
          </section>
        </div>
      }
      primary={
        <section className="work-unit-inspection" aria-label="Work Unit Activity and Evidence">
          <nav
            className="work-unit-inspection__tabs"
            aria-label="Work Unit detail views"
            role="tablist"
          >
            {(['activity', 'evidence'] as const).map((tab) => (
              <button
                key={tab}
                id={`work-unit-${tab}-tab`}
                type="button"
                role="tab"
                aria-selected={inspectionTab === tab}
                aria-controls={`work-unit-${tab}-view`}
                tabIndex={inspectionTab === tab ? 0 : -1}
                onClick={() => setInspectionTab(tab)}
                onKeyDown={(event) => {
                  if (event.key === 'ArrowRight' || event.key === 'ArrowLeft') {
                    event.preventDefault();
                    const nextTab = tab === 'activity' ? 'evidence' : 'activity';
                    setInspectionTab(nextTab);
                    document.getElementById(`work-unit-${nextTab}-tab`)?.focus();
                  }
                }}
              >
                {tab === 'activity' ? 'Activity' : 'Evidence'}
              </button>
            ))}
          </nav>
          {inspectionTab === 'activity' ? (
            <div
              id="work-unit-activity-view"
              role="tabpanel"
              aria-labelledby="work-unit-activity-tab"
            >
              <WorkUnitActivityView
                inspection={unit.inspection}
                selectedActivityId={selectedActivityId}
                onSelectActivity={(activityId) => setSelectedActivityId(activityId)}
                onHighlightActivity={setHighlightedActivityId}
                sessions={sessions}
                onOpenActivitySession={onOpenActivitySession}
              />
            </div>
          ) : (
            <div
              id="work-unit-evidence-view"
              role="tabpanel"
              aria-labelledby="work-unit-evidence-tab"
            >
              <WorkUnitEvidenceView
                inspection={unit.inspection}
                onSelectActivity={(activityId) => {
                  setSelectedActivityId(activityId);
                  setInspectionTab('activity');
                }}
                onOpenFileEvidence={onOpenFileEvidence}
              />
            </div>
          )}
        </section>
      }
    />
  );
}

function WorkUnitActivityView({
  inspection,
  selectedActivityId,
  onSelectActivity,
  onHighlightActivity,
  sessions,
  onOpenActivitySession,
}: {
  readonly inspection?: ProductWorkUnitInspectionV1;
  readonly selectedActivityId: string | null;
  readonly onSelectActivity: (activityId: string) => void;
  readonly onHighlightActivity: (activityId: string | null) => void;
  readonly sessions: readonly WorkUnitAgentSessionPresentation[];
  readonly onOpenActivitySession?: (target: WorkUnitActivitySessionTarget) => void;
}) {
  const selectedActivity = inspection?.activities.find(
    (activity) => activity.activityId === selectedActivityId,
  );

  return (
    <section className="work-unit-activity" aria-label="Work Unit Activity">
      <header className="work-unit-inspection__heading">
        <div>
          <span>Agent-only record</span>
          <h2>Activity</h2>
        </div>
        <p>Application summaries are nested beneath their owning Handler or Implementer turn.</p>
      </header>
      {inspection?.activities.length ? (
        <ol className="work-unit-activity__list">
          {inspection.activities.map((activity) => (
            <li
              key={activity.activityId}
              className={selectedActivityId === activity.activityId ? 'is-selected' : undefined}
              data-activity-id={activity.activityId}
              onMouseEnter={() => onHighlightActivity(activity.activityId)}
              onMouseLeave={() => onHighlightActivity(null)}
            >
              <button
                type="button"
                aria-pressed={selectedActivityId === activity.activityId}
                onClick={() => onSelectActivity(activity.activityId)}
                onFocus={() => onHighlightActivity(activity.activityId)}
                onBlur={() => onHighlightActivity(null)}
              >
                <span className="work-unit-activity__role">{roleLabel(activity.role)}</span>
                <strong>{stageLabel(activity.primaryStage)}</strong>
                <small>{activity.invocationId}</small>
              </button>
              {activity.applicationSummary && (
                <ApplicationActivitySummary
                  activity={activity}
                  activities={inspection.activities}
                  onSelectActivity={onSelectActivity}
                />
              )}
              {selectedActivityId === activity.activityId ? (
                <SelectedActivityTurn
                  activity={activity}
                  session={sessions.find(
                    (session) => session.sessionId === activity.agentSessionId,
                  )}
                  onOpenActivitySession={onOpenActivitySession}
                />
              ) : null}
            </li>
          ))}
        </ol>
      ) : (
        <p className="work-unit-inspection__unavailable">
          No application-owned agent activity is available for this Work Unit.
        </p>
      )}
      {!selectedActivity ? (
        <p className="work-unit-inspection__selection-hint">
          Select an activity to inspect its complete recorded turn.
        </p>
      ) : null}
    </section>
  );
}

function SelectedActivityTurn({
  activity,
  session,
  onOpenActivitySession,
}: {
  readonly activity: ProductWorkUnitInspectionActivityV1;
  readonly session?: WorkUnitAgentSessionPresentation;
  readonly onOpenActivitySession?: (target: WorkUnitActivitySessionTarget) => void;
}) {
  const invocationIndex = session?.transcript?.invocations.findIndex(
    ({ id }) => id === activity.invocationId,
  );
  const previous =
    invocationIndex !== undefined && invocationIndex > 0
      ? session?.transcript?.invocations[invocationIndex - 1]
      : undefined;
  return (
    <section className="work-unit-activity__inspection" aria-label="Selected activity turn">
      <header>
        <div>
          <span>Exact recorded pointer</span>
          <h3>{stageLabel(activity.primaryStage)}</h3>
        </div>
        {onOpenActivitySession ? (
          <button
            type="button"
            onClick={() =>
              onOpenActivitySession({
                sessionId: activity.agentSessionId,
                invocationId: activity.invocationId,
                activityId: activity.activityId,
              })
            }
          >
            Open in Agent Sessions
          </button>
        ) : null}
      </header>
      <AgentSessionTurnInspector
        sessionId={activity.agentSessionId}
        invocationId={activity.invocationId}
        transcript={session?.transcript ?? null}
        precedingInput={
          previous
            ? {
                invocationId: previous.id,
                text: previous.submittedText,
                provenance: previous.inputProvenance,
              }
            : undefined
        }
        ariaLabel={`Agent Session turn: ${activity.invocationId}`}
      />
    </section>
  );
}

function ApplicationActivitySummary({
  activity,
  activities,
  onSelectActivity,
}: {
  readonly activity: ProductWorkUnitInspectionActivityV1;
  readonly activities: readonly ProductWorkUnitInspectionActivityV1[];
  readonly onSelectActivity: (activityId: string) => void;
}) {
  const summary = activity.applicationSummary!;
  return (
    <section className="work-unit-activity__application" aria-label="Application summary">
      <h4>Application summary</h4>
      <ul>
        {summary.applicationEvents.map((event) => (
          <li key={event}>{applicationEventLabel(event)}</li>
        ))}
      </ul>
      {summary.peerEvidenceActivityIds.length ? (
        <div>
          <strong>Related activity</strong>
          {summary.peerEvidenceActivityIds.map((activityId) => {
            const target = activities.find((candidate) => candidate.activityId === activityId);
            return target ? (
              <button key={activityId} type="button" onClick={() => onSelectActivity(activityId)}>
                {stageLabel(target.primaryStage)}
              </button>
            ) : (
              <span key={activityId}>Related activity unavailable ({activityId})</span>
            );
          })}
        </div>
      ) : null}
      <p>
        <strong>MCP calls:</strong> {summary.mcpCallDetail.reason}
      </p>
    </section>
  );
}

function WorkUnitEvidenceView({
  inspection,
  onSelectActivity,
  onOpenFileEvidence,
}: {
  readonly inspection?: ProductWorkUnitInspectionV1;
  readonly onSelectActivity: (activityId: string) => void;
  readonly onOpenFileEvidence?: (target: WorkUnitFileEvidenceTarget) => void;
}) {
  const fileEvidence = inspection?.fileEvidence;
  const sourceActivity =
    fileEvidence?.status === 'available'
      ? inspection?.activities.find(
          (activity) => activity.activityId === fileEvidence.sourceActivityId,
        )
      : undefined;
  return (
    <section className="work-unit-evidence" aria-label="Work Unit Evidence">
      <header className="work-unit-inspection__heading">
        <div>
          <span>Application-owned detail</span>
          <h2>Evidence</h2>
        </div>
        <p>Evidence is shown only where an explicit application owner and source are recorded.</p>
      </header>
      <section className="work-unit-evidence__group" aria-label="File evidence">
        <h3>Files</h3>
        {!fileEvidence ? (
          <p>No application-owned file evidence is available for this Work Unit.</p>
        ) : fileEvidence.status === 'unavailable' ? (
          <p>Unavailable: {fileEvidence.reason}</p>
        ) : (
          <>
            <p>
              Owned by the application. Content fingerprints are recorded; file contents are not
              exposed here.
            </p>
            <ul>
              {fileEvidence.changedFiles.map((file) => (
                <li key={file.evidenceRef}>
                  {isAvailableDiffDestination(file.diffDestination) && onOpenFileEvidence ? (
                    <button
                      type="button"
                      data-evidence-id={file.evidenceRef}
                      data-file-id={file.fileId}
                      onClick={() => openFileEvidence(file.diffDestination, onOpenFileEvidence)}
                    >
                      <strong>{file.displayName}</strong>
                      <span>{file.changeKind}</span>
                    </button>
                  ) : (
                    <span data-evidence-id={file.evidenceRef} data-file-id={file.fileId}>
                      <strong>{file.displayName}</strong>
                      <span>{file.changeKind}</span>
                      <small>
                        Unavailable:{' '}
                        {isAvailableDiffDestination(file.diffDestination)
                          ? 'The application file-review destination is unavailable.'
                          : file.diffDestination.reason}
                      </small>
                    </span>
                  )}
                </li>
              ))}
            </ul>
            {sourceActivity ? (
              <button type="button" onClick={() => onSelectActivity(sourceActivity.activityId)}>
                View owning activity
              </button>
            ) : (
              <p>Owning activity unavailable; navigation is not supported.</p>
            )}
          </>
        )}
      </section>
      <section className="work-unit-evidence__group" aria-label="Test evidence">
        <h3>Tests</h3>
        {!inspection?.testEvidence || !isAvailableTestEvidence(inspection.testEvidence) ? (
          <p>
            Unavailable:{' '}
            {inspection?.testEvidence && !isAvailableTestEvidence(inspection.testEvidence)
              ? inspection.testEvidence.reason
              : 'No application-owned test detail is available for this Work Unit.'}
          </p>
        ) : (
          <div className="work-unit-test-evidence">
            <p>
              <strong>{inspection.testEvidence.whatRan}</strong> · {inspection.testEvidence.result}
            </p>
            <dl>
              <div>
                <dt>Command</dt>
                <dd>{inspection.testEvidence.command}</dd>
              </div>
              <div>
                <dt>Environment</dt>
                <dd>{inspection.testEvidence.environment}</dd>
              </div>
              <div>
                <dt>Run</dt>
                <dd>{inspection.testEvidence.runId}</dd>
              </div>
            </dl>
            {inspection.testEvidence.cases.length ? (
              <ul>
                {inspection.testEvidence.cases.map((item) => (
                  <li key={item.caseId}>
                    {item.label} · {item.result}
                  </li>
                ))}
              </ul>
            ) : null}
            {(() => {
              const testEvidence = inspection.testEvidence;
              if (!isAvailableTestEvidence(testEvidence)) return null;
              const source = inspection.activities.find(
                (activity) => activity.activityId === testEvidence.sourceActivityId,
              );
              return source ? (
                <button type="button" onClick={() => onSelectActivity(source.activityId)}>
                  View owning activity
                </button>
              ) : (
                <p>Owning activity unavailable; navigation is not supported.</p>
              );
            })()}
          </div>
        )}
      </section>
    </section>
  );
}

function isAvailableTestEvidence(
  evidence: NonNullable<ProductWorkUnitInspectionV1['testEvidence']>,
): evidence is Extract<
  NonNullable<ProductWorkUnitInspectionV1['testEvidence']>,
  { readonly status: 'available' }
> {
  return 'status' in evidence && evidence.status === 'available';
}

function isAvailableDiffDestination(destination: unknown): destination is {
  readonly status: 'available';
  readonly owner: 'application';
  readonly reviewId: string;
  readonly changedFileId: string;
} {
  return Boolean(
    destination &&
    typeof destination === 'object' &&
    (destination as { readonly status?: unknown }).status === 'available',
  );
}

function openFileEvidence(
  destination: unknown,
  onOpen: (target: WorkUnitFileEvidenceTarget) => void,
) {
  if (!isAvailableDiffDestination(destination)) return;
  onOpen({ reviewId: destination.reviewId, changedFileId: destination.changedFileId });
}

function roleLabel(role: ProductWorkUnitInspectionActivityV1['role']) {
  return role === 'handler' ? 'Handler' : 'Implementer';
}

function stageLabel(stage: ProductWorkUnitInspectionActivityV1['primaryStage']) {
  return {
    handler_activation: 'Handler activation',
    handler_action: 'Handler action',
    implementer_activation: 'Implementer activation',
    implementer_retry: 'Implementer retry',
    implementer_reporting: 'Implementer reporting',
    handler_review: 'Handler review',
  }[stage];
}

function applicationEventLabel(
  event: NonNullable<
    ProductWorkUnitInspectionActivityV1['applicationSummary']
  >['applicationEvents'][number],
) {
  return {
    submission_recorded: 'Submission recorded',
    file_evidence_recorded: 'File evidence recorded',
    semantic_completion_recorded: 'Semantic completion recorded',
    terminal_lifecycle_observed: 'Terminal lifecycle observed',
    application_acceptance_recorded: 'Application acceptance recorded',
    handler_review_ready: 'Handler review ready',
    review_delivery_persisted: 'Review delivery persisted',
    review_judgment_recorded: 'Review judgment recorded',
    review_lifecycle_observed: 'Review lifecycle observed',
    review_conflict_recorded: 'Review conflict recorded',
  }[event];
}

function agentInitial(
  role: SprintWorkspacePresentationV1['workUnitLifecycle'][number]['agentRole'],
) {
  const detail = {
    epic: 'ER',
    sprint: 'SR',
    work_slice_planner: 'SP',
    work_unit_handler: 'H',
    work_unit_implementer: 'I',
  }[role];
  return detail;
}

function roleMatchesLifecycle(
  role: ProductWorkUnitInspectionActivityV1['role'],
  lifecycleRole: SprintWorkspacePresentationV1['workUnitLifecycle'][number]['agentRole'],
) {
  return (
    (role === 'handler' && lifecycleRole === 'work_unit_handler') ||
    (role === 'implementer' && lifecycleRole === 'work_unit_implementer')
  );
}

function workUnitStatusLabel(
  state: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['presentationState'],
) {
  return {
    not_started: 'Planned',
    waiting_for_dependencies: 'Waiting for dependencies',
    requested: 'Requested',
    launched: 'In progress',
    returned: 'Returned',
    under_review: 'Under review',
    integrated: 'Completed',
    responsibility_accepted: 'Completed',
    deferred: 'Deferred',
  }[state];
}

function handlerActivity(
  activation: NonNullable<
    SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['handlerActivation']
  >,
) {
  if (activation.eligibilityState === 'blocked')
    return `Original Handler is blocked: ${activation.blockedReason}.`;
  const detail = {
    eligible_not_prepared: 'Original Handler is authorized but not yet prepared.',
    invocation_prepared: 'Original Handler invocation is prepared.',
    launch_requested: 'Original Handler launch has been requested.',
    launch_accepted:
      'Original Handler launch was accepted; application readiness is not yet recorded.',
    handler_ready: 'Original Handler is application-ready.',
  }[activation.stage];
  return activation.providerActivityObserved
    ? `${detail} Provider activity is observed separately.`
    : detail;
}

function executionStateDetail(
  state: NonNullable<
    SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['executionState']
  >['state'],
) {
  return {
    waiting_on_prerequisites: 'Waiting on recorded prerequisite work.',
    ready: 'Ready for the recorded next work.',
    active: 'Work is active.',
    retry_authorized: 'A retry is authorized.',
    handed_back: 'Work was handed back; no replacement is implied.',
    settled: 'Work Unit execution is settled.',
    attention: 'This Work Unit needs attention. Other lanes retain their own recorded state.',
  }[state];
}

function actionContinuationActivity(
  continuation: NonNullable<
    SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['actionContinuation']
  >,
) {
  if (continuation.stage === 'blocked')
    return `Handler action continuation is blocked: ${continuation.blockedReason}.`;
  if (continuation.stage === 'failed')
    return `Handler action continuation needs attention: ${continuation.failureReason}.`;
  const detail = {
    requested: 'Handler action continuation was requested.',
    authorized: 'Handler action continuation is authorized.',
    invocation_prepared: 'Handler action continuation invocation is prepared.',
    harness_bound: 'Handler action continuation is bound.',
    launch_requested: 'Handler action continuation launch has been requested.',
    launch_accepted:
      'Handler action continuation launch was accepted; application readiness is not yet recorded.',
    action_ready: 'Handler action continuation is application-ready.',
  }[continuation.stage];
  return continuation.providerActivityObserved
    ? `${detail} Provider activity is observed separately.`
    : detail;
}

function implementerActivity(
  activation: NonNullable<
    SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['implementerActivation']
  >,
) {
  if (activation.stage === 'failed')
    return `Implementer activation needs attention: ${activation.failureReason}.`;
  const detail = {
    requested: 'Implementer activation was requested.',
    authorized: 'Implementer activation is authorized.',
    execution_support_granted: 'Implementer execution support is granted.',
    worktree_ready: 'Implementer isolated worktree is ready.',
    session_created: 'Implementer Session is created.',
    invocation_prepared: 'Implementer invocation is prepared.',
    harness_bound: 'Implementer invocation is bound.',
    launch_requested: 'Implementer launch has been requested.',
    launch_accepted: 'Implementer launch was accepted; application readiness is not yet recorded.',
    implementer_ready: 'Implementer is application-ready.',
  }[activation.stage];
  return activation.providerActivityObserved
    ? `${detail} Provider activity is observed separately.`
    : detail;
}

function ImplementerOutcomeActivity({
  outcome,
}: {
  readonly outcome: ProductWorkUnitImplementerOutcomeV1;
}) {
  const reportingStage = outcome.reportingReadyAt
    ? 'Implementer reporting is application-ready.'
    : outcome.reportingLaunchAcceptedAt
      ? 'Implementer reporting launch was accepted; reporting readiness is not yet recorded.'
      : outcome.reportingLaunchRequestedAt
        ? 'Implementer reporting launch was requested; launch acceptance is not yet recorded.'
        : outcome.reportingHarnessBoundAt
          ? 'Implementer reporting is bound to its immutable Harness revision.'
          : outcome.reportingPreparedAt
            ? 'Implementer reporting invocation is prepared.'
            : 'Implementer reporting was requested.';
  return (
    <div className="work-unit-implementer-outcome">
      <h3>Implementer reporting</h3>
      <p>{reportingStage}</p>
      <dl>
        <RecordedFact label="Attempt" value={outcome.attemptId} />
        <RecordedFact label="Implementer Session" value={outcome.implementerSessionId} />
        <RecordedFact
          label="Original Implementer invocation"
          value={outcome.originalImplementerInvocationId}
        />
        <RecordedFact label="Reporting invocation" value={outcome.reportingInvocationId} />
        <RecordedFact
          label="Reporting Harness revision"
          value={outcome.reportingHarnessRevisionId}
        />
        <RecordedFact label="Requested" value={outcome.reportingRequestedAt} />
        {outcome.reportingPreparedAt && (
          <RecordedFact label="Prepared" value={outcome.reportingPreparedAt} />
        )}
        {outcome.reportingHarnessBoundAt && (
          <RecordedFact label="Harness bound" value={outcome.reportingHarnessBoundAt} />
        )}
        {outcome.reportingLaunchRequestedAt && (
          <RecordedFact label="Launch requested" value={outcome.reportingLaunchRequestedAt} />
        )}
        {outcome.reportingLaunchAcceptedAt && (
          <RecordedFact label="Launch accepted" value={outcome.reportingLaunchAcceptedAt} />
        )}
        {outcome.reportingReadyAt && (
          <RecordedFact label="Reporting ready" value={outcome.reportingReadyAt} />
        )}
      </dl>
      {outcome.failureReason && <p>Reporting needs attention: {outcome.failureReason}.</p>}
      {outcome.submittedOutcome && (
        <div>
          <h3>Submitted outcome claims</h3>
          <dl>
            <RecordedFact label="Outcome variant" value="Review pending" />
            <RecordedFact
              label="Implementer summary claim"
              value={outcome.submittedOutcome.summaryClaim}
            />
            <RecordedFact
              label="Implementer validation claim"
              value={outcome.submittedOutcome.validationStatementClaim}
            />
            <RecordedFact
              label="Claim payload fingerprint"
              value={outcome.submittedOutcome.semanticPayloadFingerprint}
            />
            <RecordedFact label="Submitted" value={outcome.submittedOutcome.submittedAt} />
            <RecordedFact
              label="Claim validation recorded"
              value={`${outcome.submittedOutcome.validationResult} at ${outcome.submittedOutcome.validationAt}`}
            />
          </dl>
          <p>These Implementer statements are claims, not application-owned evidence.</p>
        </div>
      )}
      {outcome.evidence && (
        <div>
          <h3>Application-owned File Review evidence</h3>
          <p>Evidence became ready at {outcome.evidence.readyAt}.</p>
          <p>Comparison fingerprint: {outcome.evidence.comparisonFingerprint}</p>
          <ul>
            {outcome.evidence.changedFiles.map((file) => (
              <li key={file.evidenceRef}>
                <strong>{file.displayName}</strong>
                <span>
                  {file.changeKind}; evidence reference {file.evidenceRef}; content fingerprint{' '}
                  {file.contentFingerprint}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
      {outcome.semanticCompletion && (
        <p>
          Semantic outcome completion was recorded at {outcome.semanticCompletion.completedAt} for
          the exact reporting invocation.
        </p>
      )}
      {outcome.terminalLifecycle && (
        <p>
          Reporting lifecycle was observed as {lifecycleLabel(outcome.terminalLifecycle.status)} at{' '}
          {outcome.terminalLifecycle.observedAt}.
        </p>
      )}
      {outcome.applicationAcceptedAt && (
        <p>
          The reporting outcome was application-accepted at {outcome.applicationAcceptedAt}. This is
          not implementation approval or Work Unit acceptance.
        </p>
      )}
      {outcome.handlerReviewReadyAt && (
        <p>
          <strong>Ready for Handler review</strong> at {outcome.handlerReviewReadyAt}. No Handler
          judgment is recorded here.
        </p>
      )}
    </div>
  );
}

function HandlerReviewActivity({
  review,
  decision,
}: {
  readonly review: ProductWorkUnitHandlerReviewV1;
  readonly decision?: ProductWorkUnitHandlerDecisionV1;
}) {
  const reviewStage = review.reviewReadyAt
    ? 'Handler review is application-ready.'
    : review.launchAcceptedAt
      ? 'Handler review launch was accepted; review readiness is not yet recorded.'
      : review.launchRequestedAt
        ? 'Handler review launch was requested; launch acceptance is not yet recorded.'
        : review.harnessBoundAt
          ? 'Handler review Harness is bound.'
          : review.deliveryPersistedAt
            ? 'Handler review evidence delivery is persisted.'
            : 'Handler review evidence delivery was requested.';
  return (
    <div className="work-unit-handler-review">
      <h3>Handler review</h3>
      <p>{reviewStage}</p>
      <dl>
        <RecordedFact label="Attempt" value={review.attemptId} />
        <RecordedFact label="Handler Session" value={review.handlerSessionId} />
        <RecordedFact
          label="Original Handler invocation"
          value={review.originalHandlerInvocationId}
        />
        <RecordedFact label="Handler action invocation" value={review.actionHandlerInvocationId} />
        <RecordedFact
          label="Implementer reporting invocation"
          value={review.reportingInvocationId}
        />
        <RecordedFact label="Review invocation" value={review.reviewInvocationId} />
        <RecordedFact label="Review Harness revision" value={review.reviewHarnessRevisionId} />
        <RecordedFact label="Delivery requested" value={review.deliveryRequestedAt} />
        {review.deliveryPersistedAt && (
          <RecordedFact label="Delivery persisted" value={review.deliveryPersistedAt} />
        )}
        {review.harnessBoundAt && (
          <RecordedFact label="Harness bound" value={review.harnessBoundAt} />
        )}
        {review.launchRequestedAt && (
          <RecordedFact label="Launch requested" value={review.launchRequestedAt} />
        )}
        {review.launchAcceptedAt && (
          <RecordedFact label="Launch accepted" value={review.launchAcceptedAt} />
        )}
        {review.reviewReadyAt && <RecordedFact label="Review ready" value={review.reviewReadyAt} />}
      </dl>
      <details>
        <summary>Application-bound claims and evidence</summary>
        <dl>
          <RecordedFact label="Summary claim" value={review.delivered.summaryClaim} />
          <RecordedFact
            label="Validation claim"
            value={review.delivered.validationStatementClaim}
          />
          <RecordedFact
            label="Delivered payload fingerprint"
            value={review.delivered.deliveredPayloadFingerprint}
          />
          <RecordedFact
            label="Comparison fingerprint"
            value={review.delivered.comparisonFingerprint}
          />
        </dl>
        <ul>
          {review.delivered.changedFiles.map((file) => (
            <li key={file.evidenceRef}>
              <strong>{file.displayName}</strong>
              <span>
                {file.changeKind}; evidence reference {file.evidenceRef}; content fingerprint{' '}
                {file.contentFingerprint}
              </span>
            </li>
          ))}
        </ul>
      </details>
      {review.semanticJudgment ? (
        <p>
          Handler semantic judgment was recorded as{' '}
          {review.semanticJudgment.variant === 'accept' ? 'accept' : 'return'} at{' '}
          {review.semanticJudgment.recordedAt}.
        </p>
      ) : (
        <p>Handler semantic judgment is pending; the Handler agent owns the review action.</p>
      )}
      {review.semanticJudgment?.reason && (
        <p>
          Structured return reason: {review.semanticJudgment.reason.code} -{' '}
          {review.semanticJudgment.reason.explanation}
        </p>
      )}
      {review.lifecycle && (
        <p>
          Handler review lifecycle was observed as {lifecycleLabel(review.lifecycle.status)} at{' '}
          {review.lifecycle.observedAt}.
        </p>
      )}
      {decision ? (
        <div>
          <p>
            <strong>
              Handler decision: {decision.variant === 'accepted' ? 'accepted' : 'returned'}
            </strong>{' '}
            at {decision.recordedAt}.
          </p>
          {decision.returnReason && (
            <p>
              Structured return reason: {decision.returnReason.code} -{' '}
              {decision.returnReason.explanation}
            </p>
          )}
          {decision.implementationAcceptedAt && (
            <p>
              Implementation accepted by the Handler review at {decision.implementationAcceptedAt}.
            </p>
          )}
          {decision.implementationReturnedAt && (
            <p>
              Implementation returned by the Handler review at {decision.implementationReturnedAt}.
            </p>
          )}
          {decision.retryRequiredAt && (
            <p>
              Retry is required at {decision.retryRequiredAt}. This is a legacy ordinal-1
              compatibility fact only; it is not generalized retry authorization.
            </p>
          )}
        </div>
      ) : review.lifecycle?.status === 'completed' && review.semanticJudgment ? (
        <p>No final Handler decision is recorded yet.</p>
      ) : review.lifecycle ? (
        <p>No final Handler decision is recorded for this lifecycle observation.</p>
      ) : null}
      {review.conflict && (
        <p>
          Review conflict observed at {review.conflict.occurredAt}: {review.conflict.reason}.
        </p>
      )}
      <p>No settlement, dependent activation, or upward continuation is recorded.</p>
    </div>
  );
}

function IntegrationActivity({
  integration,
}: {
  readonly integration: ProductWorkUnitIntegrationV1;
}) {
  return (
    <div className="work-unit-integration">
      <h3>Integration and settlement</h3>
      <p>
        Integration was requested at {integration.requestedAt} and authorized at{' '}
        {integration.authorizedAt}.
      </p>
      {integration.progress && (
        <p>
          Integration progress: {integration.progress.phase} at {integration.progress.recordedAt}.
        </p>
      )}
      {integration.attention && (
        <p>
          Integration needs attention: {integration.attention.safeCode.replaceAll('_', ' ')}. No
          settlement or contribution is recorded.
        </p>
      )}
      {integration.success && (
        <p>Integration success was recorded at {integration.success.recordedAt}.</p>
      )}
      {integration.settlement && (
        <p>Work Unit settlement was recorded at {integration.settlement.settledAt}.</p>
      )}
      {integration.prerequisiteContribution && (
        <p>
          Prerequisite contribution was recorded for{' '}
          {integration.prerequisiteContribution.dependentCount} dependent Work Unit
          {integration.prerequisiteContribution.dependentCount === 1 ? '' : 's'}.
        </p>
      )}
    </div>
  );
}

function IncompleteDispositionActivity({
  disposition,
}: {
  readonly disposition: ProductWorkUnitIncompleteDispositionV1;
}) {
  const classification = {
    refinement_needed: 'Refinement needed',
    functional_objective_not_satisfied: 'Functional objective not satisfied',
    blocked: 'Blocked',
  }[disposition.classification];
  return (
    <div className="work-unit-incomplete-disposition">
      <h3>Incomplete Handler disposition</h3>
      <p>
        {classification}; meaningful progress was{' '}
        {disposition.meaningfulProgress ? 'recorded' : 'not recorded'}.
      </p>
      <dl>
        <RecordedFact label="Classification" value={classification} />
        <RecordedFact
          label="Meaningful progress"
          value={disposition.meaningfulProgress ? 'Recorded' : 'Not recorded'}
        />
        <RecordedFact label="Disposition recorded" value={disposition.recordedAt} />
        {disposition.nextAttemptAuthorizedAt && (
          <RecordedFact
            label="Next attempt authorization"
            value={disposition.nextAttemptAuthorizedAt}
          />
        )}
      </dl>
      {disposition.noProgressHandback ? (
        <>
          <p>
            No meaningful progress was recorded. Work Unit handback persistence and delivery intent
            are recorded separately from Sprint Runner receiver activation or decision.
          </p>
          <dl>
            <RecordedFact label="Handback" value={disposition.noProgressHandback.handbackId} />
            <RecordedFact
              label="Source attempt"
              value={disposition.noProgressHandback.sourceAttemptId}
            />
            <RecordedFact
              label="Source review"
              value={disposition.noProgressHandback.sourceReviewInvocationId}
            />
            <RecordedFact
              label="Handback persisted"
              value={disposition.noProgressHandback.persistedAt}
            />
            <RecordedFact
              label="Delivery intent recorded"
              value={disposition.noProgressHandback.deliveryIntendedAt}
            />
          </dl>
        </>
      ) : (
        <p>
          The authorized next attempt is a recorded application fact; no launch, receiver
          activation, settlement, integration, or dependent activation is recorded here.
        </p>
      )}
      <p>No user acceptance is recorded by this disposition.</p>
    </div>
  );
}

function RetryAttemptActivity({
  retryAttempt,
}: {
  readonly retryAttempt: ProductWorkUnitRetryAttemptV1;
}) {
  const status = retryAttempt.failureReason
    ? 'Retry attempt failed and needs attention. It is not ready; no relaunch or replacement is implied.'
    : retryAttempt.retryReadyAt
      ? 'Retry attempt is application-ready.'
      : retryAttempt.launchAcceptedAt
        ? 'Retry launch was accepted; retry readiness is not yet recorded.'
        : retryAttempt.launchRequestedAt
          ? 'Retry launch was requested; launch acceptance is not yet recorded.'
          : retryAttempt.implementerHarnessBoundAt
            ? 'Retry Implementer Harness is bound.'
            : retryAttempt.implementerInvocationPreparedAt
              ? 'Retry Implementer invocation is prepared.'
              : retryAttempt.implementerSessionCreatedAt
                ? 'Retry Implementer Session is created.'
                : retryAttempt.isolatedWorktreeReadyAt
                  ? 'Retry isolated WorkspaceWrite package is ready.'
                  : retryAttempt.executionSupportGrantedAt
                    ? 'Retry execution support is granted.'
                    : retryAttempt.authorizedAt
                      ? 'Retry attempt is authorized.'
                      : retryAttempt.candidatePinnedAt
                        ? 'Retry candidate is pinned.'
                        : 'Retry capture was requested.';
  return (
    <div className="work-unit-retry-attempt">
      <h3>Returned Work Unit retry</h3>
      <p>{status}</p>
      <dl>
        <RecordedFact label="Ordinal" value={String(retryAttempt.ordinal)} />
        <RecordedFact label="Origin Implementer attempt" value={retryAttempt.originAttemptId} />
        <RecordedFact label="Retry attempt" value={retryAttempt.retryAttemptId} />
        <RecordedFact label="Retry Implementer Session" value={retryAttempt.implementerSessionId} />
        <RecordedFact
          label="Retry Implementer invocation"
          value={retryAttempt.implementerInvocationId}
        />
        <RecordedFact label="Capture requested" value={retryAttempt.captureRequestedAt} />
        {retryAttempt.candidatePinnedAt && (
          <RecordedFact label="Candidate pinned" value={retryAttempt.candidatePinnedAt} />
        )}
        {retryAttempt.authorizedAt && (
          <RecordedFact label="Authorized" value={retryAttempt.authorizedAt} />
        )}
        {retryAttempt.executionSupportGrantedAt && (
          <RecordedFact
            label="Execution support granted"
            value={retryAttempt.executionSupportGrantedAt}
          />
        )}
        {retryAttempt.isolatedWorktreeReadyAt && (
          <RecordedFact
            label="Isolated WorkspaceWrite package ready"
            value={retryAttempt.isolatedWorktreeReadyAt}
          />
        )}
        {retryAttempt.implementerSessionCreatedAt && (
          <RecordedFact
            label="Implementer Session created"
            value={retryAttempt.implementerSessionCreatedAt}
          />
        )}
        {retryAttempt.implementerInvocationPreparedAt && (
          <RecordedFact
            label="Implementer invocation prepared"
            value={retryAttempt.implementerInvocationPreparedAt}
          />
        )}
        {retryAttempt.implementerHarnessBoundAt && (
          <RecordedFact
            label="Implementer Harness bound"
            value={retryAttempt.implementerHarnessBoundAt}
          />
        )}
        {retryAttempt.launchRequestedAt && (
          <RecordedFact label="Launch requested" value={retryAttempt.launchRequestedAt} />
        )}
        {retryAttempt.launchAcceptedAt && (
          <RecordedFact label="Launch accepted" value={retryAttempt.launchAcceptedAt} />
        )}
        {retryAttempt.providerActivationObservedAt && (
          <RecordedFact
            label="Provider activation observed separately"
            value={retryAttempt.providerActivationObservedAt}
          />
        )}
        {retryAttempt.retryReadyAt && (
          <RecordedFact label="Retry ready" value={retryAttempt.retryReadyAt} />
        )}
      </dl>
      {retryAttempt.failureReason && (
        <p>
          Retry failed and needs attention: {retryAttempt.failureReason}. It is not ready.{' '}
          {retryAttempt.providerActivationObservedAt
            ? 'Provider activation was observed separately.'
            : 'No provider activation is recorded.'}
        </p>
      )}
      {!retryAttempt.failureReason && !retryAttempt.retryReadyAt && (
        <p>
          Retry readiness is not yet recorded. This surface does not imply recovery or relaunch.
        </p>
      )}
    </div>
  );
}

function RecordedFact({ label, value }: { readonly label: string; readonly value: string }) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </>
  );
}

function lifecycleLabel(status: 'completed' | 'failed' | 'canceled' | 'interrupted') {
  return {
    completed: 'Completed',
    failed: 'Failed',
    canceled: 'Canceled',
    interrupted: 'Interrupted',
  }[status];
}
