import type {
  SprintWorkspacePresentationAdjunct,
  SprintWorkspaceDetailLocation,
  SprintAgentSessionPresentation,
  WorkUnitAgentSessionPresentation,
} from '../orchestrationModel';
import type {
  ArtifactAccessController,
  SprintAutomaticContinuationPolicyController,
  SprintWorkspacePresentationV1,
  ProductSprintRunnerHandbackKnownMovementKindV1,
  ProductSprintRunnerHandbackMovementV1,
} from '../../../application/orchestrations';
import { useEffect, useRef, useState } from 'react';
import { DetailWorkspace } from './DetailWorkspace';
import { SprintContinuationControl } from './SprintContinuationControl';
import { SprintConcernsPanel } from './SprintConcernsPanel';
import { SprintDocumentsPanel } from './SprintDocumentsPanel';
import { SprintFlowMap } from './SprintFlowMap';
import { SprintWorkspaceTabs, type SprintWorkspaceTab } from './SprintWorkspaceTabs';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import {
  type PlanningPointWorkUnitRelationship,
  WorkSlicePlanningPointDetailWorkspace,
} from './WorkSlicePlanningPointDetailWorkspace';
import {
  WorkUnitDetailWorkspace,
  type WorkUnitActivitySessionTarget,
  type WorkUnitFileEvidenceOpenContext,
} from './WorkUnitDetailWorkspace';
import type {
  AgentSessionProductLocation,
  AgentSessionProductOrigin,
} from '../../../application/agentSessionNavigation';
import '../styles/sprintWorkspace.css';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type {
  ContextualFileReviewResult,
  ContextualFileReviewFailureReason,
} from '../../../application/contextualFileReview';

type SprintFileReviewControlState =
  | { readonly kind: 'idle' | 'pending' }
  | {
      readonly kind: 'failed';
      readonly reason: ContextualFileReviewFailureReason;
      readonly message: string;
    };

export interface SprintWorkspaceProps {
  readonly workspace: SprintWorkspacePresentationV1;
  readonly adjunct?: SprintWorkspacePresentationAdjunct;
  readonly artifactAccessController: ArtifactAccessController;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly automaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly selectedRevisionId: string;
  readonly onSelectedRevisionChange: (revisionId: string) => void;
  readonly detailLocation: SprintWorkspaceDetailLocation;
  readonly onDetailLocationChange: (location: SprintWorkspaceDetailLocation) => void;
  readonly onBack: () => void;
  readonly onOpenAgentSession?: (origin: AgentSessionProductOrigin) => void;
  readonly onRequestFileReview?: (
    sprintId: string,
    returnLocation?: AgentSessionProductLocation,
  ) => Promise<ContextualFileReviewResult>;
  readonly onOpenFileEvidence?: (
    target: {
      readonly reviewId: string;
      readonly changedFileId: string;
    },
    returnLocation?: AgentSessionProductLocation,
  ) => void;
  readonly onOpenWorkUnitActivitySession?: (
    target: WorkUnitActivitySessionTarget,
    origin: AgentSessionProductOrigin,
  ) => void;
}

export function SprintWorkspace({
  workspace,
  adjunct,
  artifactAccessController,
  agentSessionComposition,
  automaticContinuationPolicyController,
  selectedRevisionId,
  onSelectedRevisionChange,
  detailLocation,
  onDetailLocationChange,
  onBack,
  onOpenAgentSession,
  onRequestFileReview,
  onOpenFileEvidence,
  onOpenWorkUnitActivitySession,
}: SprintWorkspaceProps) {
  const [selectedTab, setSelectedTab] = useState<SprintWorkspaceTab>('flow');
  const [selectedConcernId, setSelectedConcernId] = useState<string | null>(null);
  const [highlightedSprintRunnerConcernId, setHighlightedSprintRunnerConcernId] = useState<
    string | null
  >(null);
  const [hoveredGraphElement, setHoveredGraphElement] = useState<{
    readonly kind: 'work_slice_planning_point' | 'work_unit' | 'gate';
    readonly id: string;
  } | null>(null);
  const sprintRunnerConcernFocusIndexRef = useRef(new Map<string, number>());
  const sprintRestoreRef = useRef<{
    kind: 'work_slice_planning_point' | 'work_unit';
    id: string;
  } | null>(null);
  const concernRestoreWorkUnitRef = useRef<string | null>(null);
  const fileReviewRequestSequence = useRef(0);
  const [fileReviewState, setFileReviewState] = useState<SprintFileReviewControlState>({
    kind: 'idle',
  });
  const planningValue =
    workspace.sprint.planningState.source.status === 'available'
      ? workspace.sprint.planningState.value
      : undefined;
  const hasStartedPlan = planningValue?.kind === 'started_plan';
  const hasPreStartForecast = planningValue?.kind === 'pre_start_forecast';
  const planningUnavailableReason =
    workspace.sprint.planningState.source.status === 'available'
      ? 'The planning state is not available.'
      : workspace.sprint.planningState.source.reason;

  useEffect(() => {
    if (detailLocation.kind !== 'sprint' || !sprintRestoreRef.current) return;
    const restore = sprintRestoreRef.current;
    sprintRestoreRef.current = null;
    document
      .querySelector<HTMLButtonElement>(
        restore.kind === 'work_slice_planning_point'
          ? `[data-work-slice-planning-point-id="${restore.id}"]`
          : `[data-work-unit-id="${restore.id}"]`,
      )
      ?.focus();
  }, [detailLocation]);

  useEffect(() => {
    fileReviewRequestSequence.current += 1;
    setFileReviewState({ kind: 'idle' });
  }, [workspace.sprint.sprintId]);

  const requestFileReview = async () => {
    if (!onRequestFileReview || fileReviewState.kind === 'pending') return;
    const sequence = ++fileReviewRequestSequence.current;
    setFileReviewState({ kind: 'pending' });
    const result = await onRequestFileReview(
      workspace.sprint.sprintId,
      fileReviewReturnLocation(workspace, detailLocation),
    );
    if (fileReviewRequestSequence.current !== sequence) return;
    setFileReviewState(
      result.status === 'failed'
        ? { kind: 'failed', reason: result.reason, message: result.message }
        : { kind: 'idle' },
    );
  };
  const fileReviewControl =
    hasStartedPlan && onRequestFileReview ? (
      <SprintFileReviewControl state={fileReviewState} onRequest={requestFileReview} />
    ) : undefined;

  useEffect(() => {
    if (detailLocation.kind !== 'sprint' || !concernRestoreWorkUnitRef.current) return;
    const id = concernRestoreWorkUnitRef.current;
    concernRestoreWorkUnitRef.current = null;
    document.querySelector<HTMLButtonElement>(`[data-concern-work-unit-id="${id}"]`)?.focus();
  }, [detailLocation]);

  const selectedView = workspace.revisionViews.find(
    ({ sprintPlanRevisionId }) => sprintPlanRevisionId === selectedRevisionId,
  )!;
  const activeView = workspace.revisionViews.find(
    ({ sprintPlanRevisionId }) => sprintPlanRevisionId === workspace.activeSprintPlanRevisionId,
  )!;
  const ownerOf = (workUnitId: string, view: (typeof workspace.revisionViews)[number]) =>
    view.workSlicePlanningPointGroups.find(({ workUnitScopeIds }) =>
      workUnitScopeIds.includes(
        view.workUnits.find((unit) => unit.workUnitId === workUnitId)?.workUnitScopeId ?? '',
      ),
    );

  if (hasStartedPlan && detailLocation.kind === 'work_unit') {
    const view = workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === detailLocation.revisionId,
    )!;
    const workSlicePlanningPointGroup = view.workSlicePlanningPointGroups.find(
      ({ workSlicePlanningPointId }) =>
        workSlicePlanningPointId === detailLocation.workSlicePlanningPointId,
    )!;
    const unit = view.workUnits.find(({ workUnitId }) => workUnitId === detailLocation.workUnitId)!;
    return (
      <WorkUnitDetailWorkspace
        unit={unit}
        lifecycleEntries={workspace.workUnitLifecycle.filter(
          ({ workUnitId }) => workUnitId === unit.workUnitId,
        )}
        workSlicePlanningPointGroupTitle={workSlicePlanningPointGroup.title}
        sessions={workUnitSessions(workspace, unit, adjunct)}
        agentSessionComposition={agentSessionComposition}
        backLabel={
          detailLocation.origin === 'concern'
            ? 'Back to Concern'
            : 'Back to Work Slice planning point'
        }
        onBack={() => {
          if (detailLocation.origin === 'concern') {
            concernRestoreWorkUnitRef.current = detailLocation.workUnitId;
            onDetailLocationChange({ kind: 'sprint' });
            return;
          }
          onDetailLocationChange({
            kind: 'work_slice_planning_point',
            revisionId: detailLocation.revisionId,
            workSlicePlanningPointId: detailLocation.workSlicePlanningPointId,
          });
        }}
        onOpenActivitySession={(target) =>
          onOpenWorkUnitActivitySession?.(target, {
            sessionId: target.sessionId,
            invocationId: target.invocationId,
            location: {
              kind: 'work_unit',
              epicId: workspace.sprint.epicId,
              sprintId: workspace.sprint.sprintId,
              revisionId: detailLocation.revisionId,
              workSlicePlanningPointId: detailLocation.workSlicePlanningPointId,
              workUnitId: detailLocation.workUnitId,
              label: unit.title,
              inspectionState: {
                tab: 'activity',
                activityId: target.activityId,
                sessionId: target.sessionId,
                invocationId: target.invocationId,
              },
            },
          })
        }
        onOpenFileEvidence={(target, context) =>
          onOpenFileEvidence?.(target, fileReviewReturnLocation(workspace, detailLocation, context))
        }
        initialInspectionState={detailLocation.inspectionState}
        sprintControl={fileReviewControl}
      />
    );
  }

  if (hasStartedPlan && detailLocation.kind === 'work_slice_planning_point') {
    const view = workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === detailLocation.revisionId,
    )!;
    const workSlicePlanningPointGroup = view.workSlicePlanningPointGroups.find(
      ({ workSlicePlanningPointId }) =>
        workSlicePlanningPointId === detailLocation.workSlicePlanningPointId,
    )!;
    return (
      <WorkSlicePlanningPointDetailWorkspace
        workSlicePlanningPointGroup={workSlicePlanningPointGroup}
        currentWorkState={workSlicePlanningPointState(workSlicePlanningPointGroup, view)}
        workUnitRelationships={planningPointWorkUnitRelationships(
          workspace,
          view,
          workSlicePlanningPointGroup,
          adjunct,
        )}
        plannerSession={workSlicePlanningPointSession(
          workspace,
          workSlicePlanningPointGroup.workSlicePlanningPointId,
          adjunct,
        )}
        agentSessionComposition={agentSessionComposition}
        workflow={adjunct?.workSlicePlanningPointWorkflows.find(
          ({ workSlicePlanningPointId }) =>
            workSlicePlanningPointId === workSlicePlanningPointGroup.workSlicePlanningPointId,
        )}
        onBack={() => onDetailLocationChange({ kind: 'sprint' })}
        onOpenWorkUnit={(workUnitId) => {
          onDetailLocationChange({
            kind: 'work_unit',
            revisionId: view.sprintPlanRevisionId,
            workSlicePlanningPointId: workSlicePlanningPointGroup.workSlicePlanningPointId,
            workUnitId,
            origin: 'work_slice_planning_point',
          });
        }}
        onOpenAgentSession={(sessionId) =>
          onOpenAgentSession?.({
            sessionId,
            location: {
              kind: 'work_slice_planning_point',
              epicId: workspace.sprint.epicId,
              sprintId: workspace.sprint.sprintId,
              revisionId: detailLocation.revisionId,
              workSlicePlanningPointId: workSlicePlanningPointGroup.workSlicePlanningPointId,
              label: workSlicePlanningPointGroup.title,
            },
          })
        }
        sprintControl={fileReviewControl}
      />
    );
  }

  return (
    <DetailWorkspace
      ariaLabel="Sprint detail"
      controlsLabel="Sprint controls"
      contextLabel="Sprint context"
      backLabel="Back to Epic"
      onBack={onBack}
      focusBackOnMount
      hotbarNavigation={
        hasStartedPlan ? (
          <SprintWorkspaceTabs selected={selectedTab} onSelect={setSelectedTab} />
        ) : undefined
      }
      control={
        <div className="sprint-header-controls">
          {fileReviewControl}
          <SprintContinuationControl
            automaticEnabled={workspace.continuation.policy?.automaticEnabled ?? false}
            controller={automaticContinuationPolicyController}
            policyUpdateIntent={
              workspace.continuation.policy
                ? {
                    level: 'sprint',
                    sprintId: workspace.sprint.sprintId,
                    policyId: workspace.continuation.policy.policyId,
                    automaticEnabled: workspace.continuation.policy.automaticEnabled,
                  }
                : undefined
            }
          />
        </div>
      }
      context={
        <div className="sprint-context">
          <p className="eyebrow">Sprint</p>
          <h1>{workspace.sprint.title}</h1>
          <span
            className={`sprint-context__state sprint-context__state--${
              workspace.sprint.lifecycle?.value ??
              workspace.sprint.lifecycle?.source.status ??
              'unavailable'
            }`}
          >
            {sprintLifecycleLabel(workspace.sprint.lifecycle)}
          </span>
          <p>{workspace.sprint.summary}</p>
          {workspace.sprint.sprintRunnerTransition ? (
            <section
              className="sprint-context__runner-transition"
              aria-label="Sprint Runner activation"
            >
              <h2>Sprint Runner activation</h2>
              <p>{workspace.sprint.sprintRunnerTransition.label}</p>
              <ul>
                <li>Requested and authorized</li>
                {workspace.sprint.sprintRunnerTransition.sessionCreatedAt ? (
                  <li>Session created</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.harnessAppliedAt ? (
                  <li>Harness applied</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.launchAcceptedAt ? (
                  <li>Launch accepted; pre-start ready</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.preStartSemanticOutcomeRecordedAt ? (
                  <li>Pre-start outcome recorded</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.preStartLifecycleObservedAt ? (
                  <li>Matching pre-start lifecycle observed</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.preStartOutcomeAcceptedAt ? (
                  <li>Pre-start outcome accepted</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.parentContinuationDeliveryRequestedAt ? (
                  <li>Epic continuation delivery requested</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.parentContinuationDeliveryPersistedAt ? (
                  <li>Epic continuation invocation persisted</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.epicContinuationLaunchAcceptedAt ? (
                  <li>Epic continuation launch accepted; awaiting Epic authorization</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.sprintStartPersistedAt ? (
                  <li>Sprint start authorized and persisted</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.sprintContinuationLaunchAcceptedAt ? (
                  <li>Started Sprint continuation launch accepted</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.repositoryBranchReevaluationRecordedAt ? (
                  <li>Repository and branch reevaluation recorded</li>
                ) : null}
                {workspace.sprint.sprintRunnerTransition.planningReadyAt ? (
                  <li>Planning-ready; downstream has not started</li>
                ) : null}
              </ul>
              <SprintRunnerActivationObservation
                transition={workspace.sprint.sprintRunnerTransition}
                hasCreatedWorkUnits={hasCreatedWorkUnits(workspace.sprint)}
              />
            </section>
          ) : null}
          <WorkSlicePlannerBoundary sprint={workspace.sprint} workUnits={activeView.workUnits} />
          <SprintRunnerHandbackActivity workUnits={activeView.workUnits} />
          <section className="sprint-context__objectives" aria-label="Epic Runner objectives">
            <h2>Epic Runner objectives</h2>
            {workspace.epicRunnerObjectives.length > 0 ? (
              <ul>
                {workspace.epicRunnerObjectives.map((objective) => (
                  <li key={objective.objectiveId}>{objective.title}</li>
                ))}
              </ul>
            ) : (
              <p>No recorded Epic Runner Sprint objectives.</p>
            )}
          </section>
          {hasStartedPlan && workspace.sprintRunnerConcerns.length > 0 ? (
            <section
              className="sprint-context__runner-concerns"
              aria-label="Sprint Runner concerns"
            >
              <h2>Sprint Runner concerns</h2>
              <ul>
                {workspace.sprintRunnerConcerns.map((sprintRunnerConcern) => {
                  const relatedToHover = hoveredGraphElement
                    ? sprintRunnerConcern.graphElementRefs.some(
                        (reference) =>
                          reference.kind === hoveredGraphElement.kind &&
                          reference.id === hoveredGraphElement.id,
                      )
                    : false;
                  return (
                    <li key={sprintRunnerConcern.sprintRunnerConcernId}>
                      <button
                        type="button"
                        className={
                          highlightedSprintRunnerConcernId ===
                            sprintRunnerConcern.sprintRunnerConcernId || relatedToHover
                            ? 'is-highlighted'
                            : undefined
                        }
                        aria-pressed={
                          highlightedSprintRunnerConcernId ===
                          sprintRunnerConcern.sprintRunnerConcernId
                        }
                        onPointerEnter={() =>
                          setHighlightedSprintRunnerConcernId(
                            sprintRunnerConcern.sprintRunnerConcernId,
                          )
                        }
                        onPointerLeave={() => setHighlightedSprintRunnerConcernId(null)}
                        onFocus={() =>
                          setHighlightedSprintRunnerConcernId(
                            sprintRunnerConcern.sprintRunnerConcernId,
                          )
                        }
                        onBlur={() => setHighlightedSprintRunnerConcernId(null)}
                        onClick={() => {
                          setSelectedTab('flow');
                          focusNextSprintRunnerConcernGraphElement(
                            sprintRunnerConcern,
                            selectedView,
                            sprintRunnerConcernFocusIndexRef.current,
                          );
                        }}
                      >
                        {sprintRunnerConcern.title}
                      </button>
                    </li>
                  );
                })}
              </ul>
            </section>
          ) : null}
        </div>
      }
      primary={
        <>
          {(workspace.epicEscalationReceivers ?? []).length > 0 && (
            <section
              aria-label="Unresolved Epic reassessment"
              className="orchestration-reassessment"
            >
              <p className="eyebrow">Unresolved Sprint concern</p>
              <p>
                Epic reassessment context returned to this Sprint. The concern remains unresolved.
              </p>
              {(workspace.epicEscalationReceivers ?? []).map((receiver) => (
                <div
                  key={`${receiver.epicId}:${receiver.sprintId}:${receiver.deliveryRequestedAt}`}
                >
                  {receiver.disposition?.downstreamRequest && (
                    <p>
                      Downstream request recorded only:{' '}
                      {receiver.disposition.downstreamRequest.request}. It is not delivery or
                      activation.
                    </p>
                  )}
                  {receiver.disposition?.humanExternalAttention && (
                    <p>
                      Attention requested: {receiver.disposition.humanExternalAttention.reason}.
                      Authority needed:{' '}
                      {receiver.disposition.humanExternalAttention.authorityNeeded}.
                    </p>
                  )}
                  {receiver.disposition?.consideredIntent && (
                    <p>
                      Other Epic work remains intent only: {receiver.disposition.consideredIntent}.
                    </p>
                  )}
                  <p>
                    Context return, dependency request, alternate work, or attention has not cleared
                    this Sprint concern.
                  </p>
                </div>
              ))}
            </section>
          )}
          {hasPreStartForecast ? (
            <section className="sprint-forecast" aria-label="Sprint Runner pre-start forecast">
              <p className="eyebrow">Sprint Runner forecast</p>
              <h2>Concerns before Sprint start</h2>
              <p>
                This forecast stays intentionally low resolution until the Sprint starts and the
                current branch and repository state can be reevaluated.
              </p>
              {workspace.concerns.length > 0 ? (
                <ul>
                  {workspace.concerns.map((concern) => (
                    <li key={concern.concernId}>
                      <strong>{concern.title}</strong>
                      <span>{concern.summary}</span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p>No sourced pre-start concerns are available.</p>
              )}
            </section>
          ) : !hasStartedPlan ? (
            <section className="sprint-forecast" aria-label="Sprint planning unavailable">
              <p className="eyebrow">Sprint Runner plan</p>
              <h2>Planning state unavailable</h2>
              <p>{planningUnavailableReason}</p>
            </section>
          ) : null}
          {hasStartedPlan && selectedTab === 'flow' && (
            <section
              className="sprint-tab-panel"
              id="sprint-flow-panel"
              role="tabpanel"
              aria-labelledby="sprint-flow-tab"
            >
              <section className="sprint-surface-host" aria-label="Sprint Runner plan">
                <header className="sprint-start-assessment">
                  <span>Started plan</span>
                  <strong>{planningValue.repositoryAssessmentSummary}</strong>
                  <time dateTime={planningValue.reevaluatedAt}>
                    Reevaluated {new Date(planningValue.reevaluatedAt).toLocaleString()}
                  </time>
                </header>
                <SprintFlowMap
                  workspace={workspace}
                  selectedRevisionId={selectedRevisionId}
                  onSelectedRevisionChange={onSelectedRevisionChange}
                  highlightedSprintRunnerConcernId={highlightedSprintRunnerConcernId}
                  hoveredGraphElement={hoveredGraphElement}
                  onHoveredGraphElementChange={setHoveredGraphElement}
                  onOpenWorkSlicePlanningPointGroup={(workSlicePlanningPointId) => {
                    sprintRestoreRef.current = {
                      kind: 'work_slice_planning_point',
                      id: workSlicePlanningPointId,
                    };
                    onDetailLocationChange({
                      kind: 'work_slice_planning_point',
                      revisionId: selectedRevisionId,
                      workSlicePlanningPointId,
                    });
                  }}
                  onOpenWorkUnit={(workUnitId) => {
                    const owner = ownerOf(workUnitId, selectedView);
                    if (!owner) return;
                    sprintRestoreRef.current = { kind: 'work_unit', id: workUnitId };
                    onDetailLocationChange({
                      kind: 'work_unit',
                      revisionId: selectedRevisionId,
                      workSlicePlanningPointId: owner.workSlicePlanningPointId,
                      workUnitId,
                      origin: 'work_slice_planning_point',
                    });
                  }}
                />
              </section>
            </section>
          )}
          {hasStartedPlan && selectedTab === 'concerns' && (
            <section
              className="sprint-tab-panel"
              id="sprint-concerns-panel"
              role="tabpanel"
              aria-labelledby="sprint-concerns-tab"
            >
              <SprintConcernsPanel
                workspace={workspace}
                selectedConcernId={selectedConcernId}
                onSelectConcern={setSelectedConcernId}
                onOpenWorkUnit={(workUnitId) => {
                  const owner = ownerOf(workUnitId, activeView);
                  if (!owner) return;
                  onDetailLocationChange({
                    kind: 'work_unit',
                    revisionId: activeView.sprintPlanRevisionId,
                    workSlicePlanningPointId: owner.workSlicePlanningPointId,
                    workUnitId,
                    origin: 'concern',
                  });
                }}
              />
            </section>
          )}
          {hasStartedPlan && selectedTab === 'documents' && (
            <section
              className="sprint-tab-panel"
              id="sprint-documents-panel"
              role="tabpanel"
              aria-labelledby="sprint-documents-tab"
            >
              <SprintDocumentsPanel
                documents={workspace.documents}
                artifactAccess={artifactAccessController}
              />
            </section>
          )}
        </>
      }
      agentSession={
        adjunct?.agentSession ? (
          <SharedAgentSessionPanel
            ariaLabel="Sprint Agent Session"
            conversationAriaLabel="Sprint Agent Session conversation"
            session={adjunct.agentSession}
            composition={agentSessionComposition}
            onOpenStandalone={(sessionId) =>
              onOpenAgentSession?.({
                sessionId,
                location: {
                  kind: 'sprint',
                  epicId: workspace.sprint.epicId,
                  sprintId: workspace.sprint.sprintId,
                  label: workspace.sprint.title,
                },
              })
            }
            displayMode="always_open"
          />
        ) : undefined
      }
    />
  );
}

export function SprintRunnerHandbackActivity({
  workUnits,
}: {
  readonly workUnits: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'];
}) {
  const entries = workUnits.flatMap((workUnit) =>
    (workUnit.attemptHistory ?? []).flatMap((attempt) =>
      attempt.incompleteDisposition?.noProgressHandback
        ? [{ workUnit, handback: attempt.incompleteDisposition.noProgressHandback }]
        : [],
    ),
  );
  if (entries.length === 0) return null;
  return (
    <section
      className="sprint-context__runner-transition"
      aria-label="Sprint Runner Handback reassessment"
    >
      <h2>Sprint Runner Handback</h2>
      <p>
        The handed-back concern remains unresolved. Only recorded Handback and Sprint Runner stages
        are shown. Any local exhaustion record is an upward request, not final Sprint or Epic
        blockage; no Epic response is recorded here.
      </p>
      <ul>
        {entries.map(({ workUnit, handback }) => (
          <li key={`${workUnit.workUnitId}-${handback.handbackId}`}>
            <strong>{workUnit.title}</strong>: {handbackActivityDetail(handback)}
          </li>
        ))}
      </ul>
    </section>
  );
}

function handbackActivityDetail(
  handback: NonNullable<
    NonNullable<
      SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['attemptHistory'][number]['incompleteDisposition']
    >['noProgressHandback']
  >,
) {
  const stages = [
    `Handback persisted at ${handback.persistedAt}`,
    `Delivery intent recorded at ${handback.deliveryIntendedAt}`,
  ];
  const delivery = handback.sprintRunnerDelivery;
  if (!delivery) return `${stages.join('; ')}. Sprint Runner delivery is not recorded.`;
  stages.push(`Delivery requested at ${delivery.deliveryRequestedAt}`);
  if (delivery.deliveryPersistedAt) stages.push('Delivery persisted');
  if (delivery.harnessBoundAt) stages.push('Reassessment Harness binding recorded');
  if (delivery.launchRequestedAt) stages.push('Sprint Runner launch requested');
  if (delivery.launchAcceptedAt) stages.push('Sprint Runner launch accepted');
  if (delivery.providerActivationObservedAt) stages.push('Provider activity observed separately');
  if (delivery.semanticReassessmentRecordedAt) stages.push('Semantic reassessment recorded');
  if (delivery.selectedMovement) stages.push(handbackMovementDetail(delivery.selectedMovement));
  if (delivery.escalationIntentRecordedAt) stages.push('Escalation intent recorded upward');
  if (delivery.escalationDeliveryRequestedAt)
    stages.push('Escalation delivery request recorded upward');
  return `${stages.join('; ')}.`;
}

function handbackMovementDetail(
  movement: NonNullable<
    NonNullable<
      NonNullable<
        NonNullable<
          SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['attemptHistory'][number]['incompleteDisposition']
        >['noProgressHandback']
      >['sprintRunnerDelivery']
    >['selectedMovement']
  >,
) {
  if (isKnownMovementKind(movement, 'continue_eligible_work'))
    return `Alternate eligible work recorded: ${movement.eligibleWorkSummary}`;
  if (isKnownMovementKind(movement, 'wait_for_agent_dependency'))
    return `Agent-achievable dependency wait (${dependencyOwnerLabel(movement.dependencyOwnerClassification)}; owner: ${movement.dependencyOwner}; enabling result: ${movement.enablingResult}; resumption path: ${movement.resumptionPath})`;
  if (isKnownMovementKind(movement, 'local_exhaustion_escalate'))
    return `Local exhaustion recorded: ${movement.localExhaustionSummary}`;
  return `Bounded movement recorded: ${movement.rationale}${movement.boundedDetails?.length ? ` (${movement.boundedDetails.map(({ value }) => `Additional bounded detail recorded: ${value}`).join('; ')})` : ''}; no settlement or blockage is implied`;
}

function isKnownMovementKind<K extends ProductSprintRunnerHandbackKnownMovementKindV1>(
  movement: ProductSprintRunnerHandbackMovementV1,
  kind: K,
): movement is Extract<ProductSprintRunnerHandbackMovementV1, { readonly movementKind: K }> {
  return movement.movementKind === kind;
}

function dependencyOwnerLabel(
  classification:
    'work_unit_handler' | 'work_unit_implementer' | 'work_slice_planner' | 'sprint_runner',
) {
  return {
    work_unit_handler: 'Work Unit Handler',
    work_unit_implementer: 'Work Unit Implementer',
    work_slice_planner: 'Work Slice Planner',
    sprint_runner: 'Sprint Runner',
  }[classification];
}

export function WorkSlicePlannerBoundary({
  sprint,
  workUnits = [],
}: {
  readonly sprint: SprintWorkspacePresentationV1['sprint'];
  readonly workUnits?: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'];
}) {
  const transition = sprint.sprintRunnerTransition;
  const materializations = sprint.workUnitMaterializations ?? [];
  const handlerActivityWorkUnits = workUnits.filter(
    ({ handlerActivation }) => handlerActivation !== undefined,
  );
  const dependencyActivityWorkUnits = workUnits.filter(
    ({ dependencyActivationIntent }) => dependencyActivationIntent !== undefined,
  );
  const createdWorkUnits = hasCreatedWorkUnits(sprint);
  if (!transition?.workSlicePlannerRequestId) return null;
  const stage = (label: string, recorded: boolean) => (
    <li key={label}>{recorded ? label : `${label} (not recorded)`}</li>
  );
  return (
    <section className="sprint-context__runner-transition" aria-label="Work Slice Planner boundary">
      <h2>Work Slice Planner boundary</h2>
      <p>
        {createdWorkUnits
          ? 'This Sprint has durable planned responsibilities. The recorded Planner boundary remains historical.'
          : 'This Sprint currently stops at the application-owned Work Slice Planner boundary.'}
      </p>
      <ul>
        {stage('Planner request', Boolean(transition.workSlicePlannerRequestedAt))}
        {stage('Planner authorization', Boolean(transition.workSlicePlannerAuthorizedAt))}
        {stage('Work Slice planning point', Boolean(transition.workSlicePlanningPointId))}
        {stage('Planner Session', Boolean(transition.workSlicePlannerSessionCreatedAt))}
        {stage('Planner invocation', Boolean(transition.workSlicePlannerInvocationCreatedAt))}
        {stage('Harness application', Boolean(transition.workSlicePlannerHarnessAppliedAt))}
        {stage('Launch requested', Boolean(transition.workSlicePlannerLaunchRequestedAt))}
        {stage('Runtime launch accepted', Boolean(transition.workSlicePlannerLaunchAcceptedAt))}
        {stage('Planner readiness', Boolean(transition.workSlicePlannerReadyAt))}
        {stage(
          'Provider activation observed',
          Boolean(transition.workSlicePlannerProviderActivationObservedAt),
        )}
        {stage('Lifecycle observed', Boolean(transition.workSlicePlannerLifecycleObservedAt))}
        {stage('Proposal submitted', Boolean(transition.workSliceProposalSubmittedAt))}
        {stage(
          transition.workSliceProposalValidationResult === 'invalid'
            ? 'Validation rejected'
            : 'Validation accepted',
          Boolean(transition.workSliceProposalValidationResult),
        )}
        {stage('Refinement requested', Boolean(transition.workSliceRefinementRequestedAt))}
        {stage('Semantic completion', Boolean(transition.workSliceSemanticCompletedAt))}
        {stage(
          'Terminal lifecycle observed',
          Boolean(transition.workSliceTerminalLifecycleObservedAt),
        )}
        {stage('Application acceptance', Boolean(transition.workSliceApplicationAcceptedAt))}
        {stage('Materialization readiness', Boolean(transition.workSliceMaterializationReadyAt))}
      </ul>
      <p>Proposal facts remain distinct from every later Work Unit or downstream action.</p>
      {materializations.length ? (
        <section aria-label="Durable Work Unit materialization">
          <h3>Durable planned responsibilities</h3>
          <ul>
            {materializations.map((materialization) => (
              <li key={materialization.materializationId}>
                Accepted revision {materialization.acceptedRevisionId}:{' '}
                {materializationLabel(materialization.stage)}.{' '}
                {executionSummary(materialization.execution)}
              </li>
            ))}
          </ul>
          {handlerActivityWorkUnits.length || dependencyActivityWorkUnits.length ? (
            <section aria-label="Handler activation activity">
              <h3>Dependency and Handler activity</h3>
              <ul>
                {dependencyActivityWorkUnits.map((workUnit) => (
                  <li key={`${workUnit.workUnitId}-dependency`}>
                    {workUnit.title}:{' '}
                    {dependencyActivationActivityDetail(workUnit.dependencyActivationIntent!)}
                  </li>
                ))}
                {handlerActivityWorkUnits.map((workUnit) => (
                  <li key={workUnit.workUnitId}>
                    {workUnit.title}: {handlerActivationActivityDetail(workUnit.handlerActivation!)}
                  </li>
                ))}
              </ul>
              <p>These records stop at Handler activation.</p>
            </section>
          ) : (
            <p>These are planned responsibilities only. No Handler activation is recorded.</p>
          )}
        </section>
      ) : null}
      <small>{plannerObservationSummary(transition)}</small>
    </section>
  );
}

function handlerActivationActivityDetail(
  activation: NonNullable<
    SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['handlerActivation']
  >,
) {
  if (activation.eligibilityState === 'blocked')
    return `Handler activation blocked: ${activation.blockedReason}.`;
  const providerObservation = activation.providerActivityObserved
    ? ' Provider activity observed separately; no provider lifecycle, outcome, or acceptance is implied.'
    : ' Provider activity is unobserved.';
  return {
    eligible_not_prepared: `Handler activation is eligible but not yet prepared.${providerObservation}`,
    invocation_prepared: `Handler invocation prepared; launch is not yet recorded.${providerObservation}`,
    launch_requested: `Handler launch requested; acceptance is not yet recorded.${providerObservation}`,
    launch_accepted: `Handler launch accepted; application Handler readiness is not yet recorded.${providerObservation}`,
    handler_ready: `Handler launch accepted and application Handler readiness recorded.${providerObservation}`,
  }[activation.stage];
}

function dependencyActivationActivityDetail(
  intent: NonNullable<
    SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['dependencyActivationIntent']
  >,
) {
  return intent.eligibilityState === 'blocked'
    ? `Dependency activation blocked: ${intent.blockedReason}.`
    : intent.activationIntendedAt
      ? 'Dependencies eligible; Handler activation intent recorded.'
      : 'Dependencies eligible; Handler activation intent not recorded.';
}

export function SprintRunnerActivationObservation({
  transition,
  hasCreatedWorkUnits,
}: {
  readonly transition: NonNullable<
    SprintWorkspacePresentationV1['sprint']['sprintRunnerTransition']
  >;
  readonly hasCreatedWorkUnits: boolean;
}) {
  return (
    <small>
      {transition.providerReceiverActivationObservedAt
        ? 'Provider/receiver activation has been observed.'
        : 'Provider/receiver activation has not been observed.'}{' '}
      {transition.downstreamNotStarted
        ? hasCreatedWorkUnits
          ? 'The pre-materialization downstream-not-started record remains historical.'
          : 'No Work Slice or Work Unit has been created.'
        : ''}
    </small>
  );
}

function hasCreatedWorkUnits(sprint: SprintWorkspacePresentationV1['sprint']) {
  return (sprint.workUnitMaterializations ?? []).some((materialization) =>
    ['work_units_created', 'relationships_complete', 'settled'].includes(materialization.stage),
  );
}

function materializationLabel(
  stage: NonNullable<
    SprintWorkspacePresentationV1['sprint']['workUnitMaterializations']
  >[number]['stage'],
) {
  return {
    authorized: 'authorized',
    attempt_recorded: 'attempt recorded',
    work_units_created: 'Work Units created; relationships not complete',
    relationships_complete: 'relationships complete; settlement not recorded',
    settled: 'Work Units and relationships settled',
  }[stage];
}

function executionSummary(
  execution:
    | NonNullable<
        NonNullable<
          SprintWorkspacePresentationV1['sprint']['workUnitMaterializations']
        >[number]['execution']
      >
    | undefined,
) {
  if (!execution) return 'Execution progress is not recorded.';
  if (execution.attention)
    return 'Execution needs attention; no Work Slice settlement is recorded.';
  if (execution.planningPointSettlement) return 'Planning-point execution settlement is recorded.';
  if (execution.settlement) return 'Work Slice execution settlement is recorded.';
  if (execution.graphCompletion)
    return 'Graph completion is recorded; Work Slice execution settlement is not recorded.';
  return 'Execution progress is not recorded.';
}

function plannerObservationSummary(
  transition: NonNullable<SprintWorkspacePresentationV1['sprint']['sprintRunnerTransition']>,
) {
  const provider = Boolean(transition.workSlicePlannerProviderActivationObservedAt);
  const lifecycle = Boolean(transition.workSlicePlannerLifecycleObservedAt);
  if (provider && lifecycle) return 'Provider activation and lifecycle observations are recorded.';
  if (provider) return 'Provider activation observation is recorded; lifecycle remains unobserved.';
  if (lifecycle)
    return 'Lifecycle observation is recorded; provider activation remains unobserved.';
  return 'Provider activation and lifecycle remain unobserved unless durable source facts are recorded.';
}

function SprintFileReviewControl({
  state,
  onRequest,
}: {
  readonly state: SprintFileReviewControlState;
  readonly onRequest: () => Promise<void>;
}) {
  return (
    <div className="sprint-file-review-control">
      <button type="button" disabled={state.kind === 'pending'} onClick={() => void onRequest()}>
        Review files
      </button>
      {state.kind === 'pending' ? (
        <small role="status">Preparing File Review…</small>
      ) : state.kind === 'failed' ? (
        <small role="alert" data-reason={state.reason}>
          {state.message}
        </small>
      ) : null}
    </div>
  );
}

function sprintLifecycleLabel(lifecycle: SprintWorkspacePresentationV1['sprint']['lifecycle']) {
  if (!lifecycle) return 'State unavailable';
  if (lifecycle.source.status !== 'available') return `State ${lifecycle.source.status}`;
  const value = lifecycle.value;
  if (!value) return 'State unavailable';
  return {
    not_started: 'Planned',
    in_progress: 'Processing',
    completed: 'Completed',
  }[value];
}

function focusNextSprintRunnerConcernGraphElement(
  sprintRunnerConcern: SprintWorkspacePresentationV1['sprintRunnerConcerns'][number],
  view: SprintWorkspacePresentationV1['revisionViews'][number],
  focusIndexes: Map<string, number>,
) {
  const priority = (reference: (typeof sprintRunnerConcern.graphElementRefs)[number]) => {
    if (reference.kind === 'work_unit') {
      const state = view.workUnits.find(
        ({ workUnitId }) => workUnitId === reference.id,
      )?.presentationState;
      if (['requested', 'launched', 'returned', 'under_review'].includes(state ?? '')) return 0;
      if (['integrated', 'responsibility_accepted'].includes(state ?? '')) return 1;
      return 2;
    }
    if (reference.kind === 'work_slice_planning_point') {
      const group = view.workSlicePlanningPointGroups.find(
        ({ workSlicePlanningPointId }) => workSlicePlanningPointId === reference.id,
      );
      const states = view.workUnits
        .filter(({ workUnitScopeId }) => group?.workUnitScopeIds.includes(workUnitScopeId))
        .map(({ presentationState }) => presentationState);
      if (
        states.some((state) =>
          ['requested', 'launched', 'returned', 'under_review'].includes(state),
        )
      )
        return 0;
      if (
        states.length &&
        states.every((state) => ['integrated', 'responsibility_accepted'].includes(state))
      )
        return 1;
    }
    return 2;
  };
  const ordered = [...sprintRunnerConcern.graphElementRefs].sort(
    (left, right) =>
      priority(left) - priority(right) ||
      `${left.kind}:${left.id}`.localeCompare(`${right.kind}:${right.id}`),
  );
  if (!ordered.length) return;
  const index = focusIndexes.get(sprintRunnerConcern.sprintRunnerConcernId) ?? 0;
  const next = ordered[index % ordered.length];
  focusIndexes.set(sprintRunnerConcern.sprintRunnerConcernId, (index + 1) % ordered.length);
  requestAnimationFrame(() => {
    const element = Array.from(
      document.querySelectorAll<HTMLElement>('[data-flow-element-kind][data-flow-element-id]'),
    ).find(
      (candidate) =>
        candidate.dataset.flowElementKind === next.kind &&
        candidate.dataset.flowElementId === next.id,
    );
    element?.focus();
    element?.scrollIntoView?.({ block: 'center', inline: 'center' });
  });
}

function workSlicePlanningPointSession(
  workspace: SprintWorkspacePresentationV1,
  workSlicePlanningPointId: string,
  adjunct?: SprintWorkspacePresentationAdjunct,
): SprintAgentSessionPresentation | undefined {
  const adjunctById = new Map(
    (adjunct?.workSlicePlanningPointSessions ?? []).map((session) => [session.sessionId, session]),
  );
  const sessions = workspace.agentSessionReferences
    .filter(
      (reference) =>
        reference.targetKind === 'work_slice_planning_point' &&
        reference.targetId === workSlicePlanningPointId &&
        reference.semanticRole === 'work_slice_planner',
    )
    .map((reference) => ({
      sessionId: reference.agentSessionId,
      title: reference.title,
      transcript: adjunctById.get(reference.agentSessionId)?.transcript,
    }));
  return sessions.length === 1 ? sessions[0] : undefined;
}

function workSlicePlanningPointState(
  group: SprintWorkspacePresentationV1['revisionViews'][number]['workSlicePlanningPointGroups'][number],
  view: SprintWorkspacePresentationV1['revisionViews'][number],
) {
  const states = view.workUnits
    .filter(({ workUnitScopeId }) => group.workUnitScopeIds.includes(workUnitScopeId))
    .map(({ presentationState }) => presentationState);
  if (states.length === 0) return 'No scoped Work Units';
  if (states.some((state) => ['requested', 'launched', 'returned', 'under_review'].includes(state)))
    return 'Processing';
  if (states.every((state) => ['integrated', 'responsibility_accepted'].includes(state)))
    return 'Completed';
  if (states.every((state) => state === 'deferred')) return 'Deferred';
  if (states.every((state) => ['not_started', 'waiting_for_dependencies'].includes(state)))
    return 'Planned';
  return 'Mixed';
}

function planningPointWorkUnitRelationships(
  workspace: SprintWorkspacePresentationV1,
  view: SprintWorkspacePresentationV1['revisionViews'][number],
  group: SprintWorkspacePresentationV1['revisionViews'][number]['workSlicePlanningPointGroups'][number],
  adjunct?: SprintWorkspacePresentationAdjunct,
): readonly PlanningPointWorkUnitRelationship[] {
  return group.workUnitScopeIds.map((workUnitScopeId) => {
    const workUnit = view.workUnits.find(
      (candidate) => candidate.workUnitScopeId === workUnitScopeId,
    );
    if (!workUnit)
      throw new Error(
        `Work Slice planning point ${group.workSlicePlanningPointId} references missing scope ${workUnitScopeId}`,
      );
    const sessions = workUnitSessions(workspace, workUnit, adjunct);
    return {
      workUnit,
      handlers: sessions.filter(({ role }) => role === 'handler'),
      implementers: sessions.filter(({ role }) => role === 'implementer'),
    };
  });
}

function workUnitSessions(
  workspace: SprintWorkspacePresentationV1,
  unit: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number],
  adjunct?: SprintWorkspacePresentationAdjunct,
): readonly WorkUnitAgentSessionPresentation[] {
  const adjunctSessions = (adjunct?.workUnitSessions ?? []).filter(
    (session) => session.workUnitId === unit.workUnitId,
  );
  const executionIds = new Set(unit.attempts.map((attempt) => attempt.workUnitExecutionId));
  const adjunctById = new Map(
    [...adjunctSessions, ...(adjunct?.workSlicePlanningPointSessions ?? [])].map((session) => [
      session.sessionId,
      session,
    ]),
  );
  const referenced: WorkUnitAgentSessionPresentation[] = workspace.agentSessionReferences
    .filter(
      (reference) =>
        reference.targetKind === 'work_unit_execution' &&
        executionIds.has(reference.targetId) &&
        ['work_unit_handler', 'work_unit_implementer'].includes(reference.semanticRole),
    )
    .map((reference) => ({
      sessionId: reference.agentSessionId,
      title: reference.title,
      workUnitId: unit.workUnitId,
      role: (
        {
          work_unit_handler: 'handler',
          work_unit_implementer: 'implementer',
        } as const
      )[reference.semanticRole as 'work_unit_handler' | 'work_unit_implementer'],
      transcript: adjunctById.get(reference.agentSessionId)?.transcript,
    }));
  const view = workspace.revisionViews.find(
    ({ sprintPlanRevisionId }) => sprintPlanRevisionId === unit.sprintPlanRevisionId,
  );
  const owner = view?.workSlicePlanningPointGroups.find(({ workUnitScopeIds }) =>
    workUnitScopeIds.includes(unit.workUnitScopeId),
  );
  const planners: WorkUnitAgentSessionPresentation[] = owner
    ? workspace.agentSessionReferences
        .filter(
          (reference) =>
            reference.targetKind === 'work_slice_planning_point' &&
            reference.targetId === owner.workSlicePlanningPointId &&
            reference.semanticRole === 'work_slice_planner',
        )
        .map((reference) => ({
          sessionId: reference.agentSessionId,
          title: reference.title,
          workUnitId: unit.workUnitId,
          role: 'work_slice_planner',
          transcript: adjunctById.get(reference.agentSessionId)?.transcript,
        }))
    : [];
  return [
    ...new Map(
      [...planners, ...referenced].map((session) => [session.sessionId, session]),
    ).values(),
  ];
}

function fileReviewReturnLocation(
  workspace: SprintWorkspacePresentationV1,
  detailLocation: SprintWorkspaceDetailLocation,
  context?: WorkUnitFileEvidenceOpenContext,
): AgentSessionProductLocation {
  const sprint = workspace.sprint;
  if (detailLocation.kind === 'sprint')
    return {
      kind: 'sprint',
      epicId: sprint.epicId,
      sprintId: sprint.sprintId,
      label: sprint.title,
    };

  const view = workspace.revisionViews.find(
    ({ sprintPlanRevisionId }) => sprintPlanRevisionId === detailLocation.revisionId,
  );
  const workSlicePlanningPointGroup = view?.workSlicePlanningPointGroups.find(
    ({ workSlicePlanningPointId }) =>
      workSlicePlanningPointId === detailLocation.workSlicePlanningPointId,
  );
  if (detailLocation.kind === 'work_slice_planning_point')
    return {
      kind: 'work_slice_planning_point',
      epicId: sprint.epicId,
      sprintId: sprint.sprintId,
      revisionId: detailLocation.revisionId,
      workSlicePlanningPointId: detailLocation.workSlicePlanningPointId,
      label: workSlicePlanningPointGroup?.title ?? 'Planning point',
    };

  const unit = view?.workUnits.find(({ workUnitId }) => workUnitId === detailLocation.workUnitId);
  return {
    kind: 'work_unit',
    epicId: sprint.epicId,
    sprintId: sprint.sprintId,
    revisionId: detailLocation.revisionId,
    workSlicePlanningPointId: detailLocation.workSlicePlanningPointId,
    workUnitId: detailLocation.workUnitId,
    label: unit?.title ?? 'Work Unit',
    ...(context?.inspectionState || detailLocation.inspectionState
      ? { inspectionState: context?.inspectionState ?? detailLocation.inspectionState }
      : {}),
  };
}
