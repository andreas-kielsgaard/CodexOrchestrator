import { useState } from 'react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type { WorkUnitAgentSessionPresentation } from '../orchestrationModel';
import { DetailWorkspace } from './DetailWorkspace';
import { ResizableSplitSurface } from './ResizableSplitSurface';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import '../styles/orchestrationSubdetail.css';
import type { ReactNode } from 'react';

export interface WorkUnitDetailWorkspaceProps {
  readonly unit: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];
  readonly lifecycleEntries: SprintWorkspacePresentationV1['workUnitLifecycle'];
  readonly workSlicePlanningPointGroupTitle: string;
  readonly sessions: readonly WorkUnitAgentSessionPresentation[];
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly backLabel?: string;
  readonly onBack: () => void;
  readonly onOpenAgentSession?: (sessionId: string) => void;
  readonly sprintControl?: ReactNode;
}

interface SessionFocusTarget {
  readonly sessionId: string;
  readonly invocationId: string;
  readonly request: number;
}

export function WorkUnitDetailWorkspace({
  unit,
  lifecycleEntries,
  workSlicePlanningPointGroupTitle,
  sessions,
  agentSessionComposition,
  backLabel = 'Back to Work Slice planning point',
  onBack,
  onOpenAgentSession,
  sprintControl,
}: WorkUnitDetailWorkspaceProps) {
  const workUnitId = unit.workUnitId;
  const workSlicePlanner = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'work_slice_planner',
  );
  const handler = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'handler',
  );
  const implementer = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'implementer',
  );
  const [primarySessionId, setPrimarySessionId] = useState(
    handler?.sessionId ?? workSlicePlanner?.sessionId ?? '',
  );
  const [focusTarget, setFocusTarget] = useState<SessionFocusTarget | null>(null);
  const primarySession =
    sessions.find(({ sessionId }) => sessionId === primarySessionId) ?? handler ?? workSlicePlanner;

  const navigateToLifecycleTurn = (
    entry: SprintWorkspacePresentationV1['workUnitLifecycle'][number],
  ) => {
    if ([handler?.sessionId, workSlicePlanner?.sessionId].includes(entry.agentSessionId))
      setPrimarySessionId(entry.agentSessionId);
    setFocusTarget((current) => ({
      sessionId: entry.agentSessionId,
      invocationId: entry.invocationId,
      request: (current?.request ?? 0) + 1,
    }));
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
          {(unit.handlerActivation ||
            unit.actionContinuation ||
            unit.implementerActivation ||
            unit.implementerOutcome) && (
            <section className="work-unit-activation" aria-label="Work Unit activation activity">
              <h2>Activation activity</h2>
              {unit.handlerActivation && <p>{handlerActivity(unit.handlerActivation)}</p>}
              {unit.actionContinuation && (
                <p>{actionContinuationActivity(unit.actionContinuation)}</p>
              )}
              {unit.implementerActivation && (
                <p>{implementerActivity(unit.implementerActivation)}</p>
              )}
              {unit.implementerOutcome && (
                <ImplementerOutcomeActivity outcome={unit.implementerOutcome} />
              )}
            </section>
          )}
          <section className="work-unit-lifecycle" aria-label="Work Unit lifecycle turn log">
            <h2>Lifecycle</h2>
            {lifecycleEntries.length ? (
              <ol>
                {lifecycleEntries.map((entry) => {
                  const session = sessions.find(
                    ({ sessionId }) => sessionId === entry.agentSessionId,
                  );
                  return (
                    <li key={entry.entryId}>
                      <button
                        type="button"
                        onClick={() => navigateToLifecycleTurn(entry)}
                        disabled={!session}
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
        <section className="work-unit-sessions" aria-label="Work Unit Agent Sessions">
          <ResizableSplitSurface
            axis="horizontal"
            primary={
              <div className="work-unit-primary-session">
                {workSlicePlanner && handler ? (
                  <nav aria-label="Planning and handling Agent Session">
                    {[workSlicePlanner, handler].map((session) => (
                      <button
                        key={session.sessionId}
                        type="button"
                        aria-pressed={primarySession?.sessionId === session.sessionId}
                        onClick={() => setPrimarySessionId(session.sessionId)}
                      >
                        {session.role === 'work_slice_planner'
                          ? 'Work Slice Planner'
                          : 'Work Unit Handler'}
                      </button>
                    ))}
                  </nav>
                ) : null}
                <SessionSlot
                  label={
                    primarySession?.role === 'work_slice_planner'
                      ? 'Work Slice Planner'
                      : 'Work Unit Handler'
                  }
                  session={primarySession}
                  agentSessionComposition={agentSessionComposition}
                  focusTarget={focusTarget}
                  onOpenAgentSession={onOpenAgentSession}
                />
              </div>
            }
            secondary={
              <div className="work-unit-execution-session">
                <SessionSlot
                  label="Work Unit Implementer"
                  session={implementer}
                  agentSessionComposition={agentSessionComposition}
                  focusTarget={focusTarget}
                  onOpenAgentSession={onOpenAgentSession}
                />
              </div>
            }
            primaryLabel="Planning and handling conversation"
            secondaryLabel="Work Unit Implementer conversation"
            initialPrimaryPercent={50}
            minimumPrimaryPixels={220}
            minimumSecondaryPixels={220}
          />
        </section>
      }
    />
  );
}

function SessionSlot({
  label,
  session,
  agentSessionComposition,
  onOpenAgentSession,
  focusTarget,
}: {
  readonly label: string;
  readonly session?: WorkUnitAgentSessionPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly onOpenAgentSession?: (sessionId: string) => void;
  readonly focusTarget: SessionFocusTarget | null;
}) {
  return (
    <div className="work-unit-session-slot">
      <h2>{label}</h2>
      {session ? (
        <SharedAgentSessionPanel
          ariaLabel={`${label} Agent Session`}
          conversationAriaLabel={`${label} conversation`}
          session={session}
          composition={agentSessionComposition}
          onOpenStandalone={onOpenAgentSession}
          displayMode="always_open"
          focusInvocationId={
            focusTarget?.sessionId === session.sessionId ? focusTarget.invocationId : undefined
          }
          focusRequest={focusTarget?.request}
        />
      ) : (
        <section className="work-unit-session-empty" aria-label={`${label} unavailable`}>
          <strong>No recorded session</strong>
          <p>This projected Work Unit has no manufactured launch or conversation.</p>
        </section>
      )}
    </div>
  );
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
  readonly outcome: NonNullable<
    SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['implementerOutcome']
  >;
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
        <RecordedFact
          label="Reporting Harness configuration digest"
          value={outcome.reportingHarnessConfigurationDigest}
        />
        <RecordedFact
          label="Reporting Harness repository commit"
          value={outcome.reportingHarnessRepositoryCommitRef}
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
