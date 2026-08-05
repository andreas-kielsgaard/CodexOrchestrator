import { useEffect, useRef, useState } from 'react';
import type {
  SprintPlanItemPresentation,
  SprintWorkspaceDetailLocation,
  EpicPresentation,
} from '../orchestrationModel';
import { ContinuationControl } from './ContinuationControl';
import { DetailWorkspace } from './DetailWorkspace';
import { SprintDetailDialog } from './SprintDetailDialog';
import { SprintPlan } from './SprintPlan';
import { SprintWorkspace } from './SprintWorkspace';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import { sprintStatusLabel } from './presentationLabels';
import '../styles/epicDetail.css';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type {
  ArtifactAccessController,
  SprintAutomaticContinuationPolicyController,
  EpicAutomaticContinuationPolicyController,
} from '../../../application/orchestrations';
import type { ContextualFileReviewResult } from '../../../application/contextualFileReview';

export interface EpicDetailProps {
  readonly epic: EpicPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly artifactAccessController: ArtifactAccessController;
  readonly sprintAutomaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly epicAutomaticContinuationPolicyController?: EpicAutomaticContinuationPolicyController;
  readonly selectedSprintId: string | null;
  readonly selectedRevisionId: string | null;
  readonly detailLocation: SprintWorkspaceDetailLocation;
  readonly onOpenSprint: (sprintId: string, revisionId: string) => void;
  readonly onCloseSprint: () => void;
  readonly onSelectedRevisionChange: (revisionId: string) => void;
  readonly onDetailLocationChange: (location: SprintWorkspaceDetailLocation) => void;
  readonly onBack: () => void;
  readonly onOpenAgentSession?: (sessionId: string) => void;
  readonly onRequestFileReview?: (sprintId: string) => Promise<ContextualFileReviewResult>;
}

export function EpicDetail({
  epic,
  agentSessionComposition,
  artifactAccessController,
  sprintAutomaticContinuationPolicyController,
  epicAutomaticContinuationPolicyController,
  selectedSprintId,
  selectedRevisionId,
  detailLocation,
  onOpenSprint,
  onCloseSprint,
  onSelectedRevisionChange,
  onDetailLocationChange,
  onBack,
  onOpenAgentSession,
  onRequestFileReview,
}: EpicDetailProps) {
  const restoreSprintIdRef = useRef<string | null>(null);
  const [selectedSprintOpener, setSelectedSprintOpener] = useState<{
    readonly sprint: SprintPlanItemPresentation;
    readonly opener: HTMLButtonElement;
  } | null>(null);
  const selectedSprint = epic.plan.items.find(({ id }) => id === selectedSprintId);
  const activeSprint =
    epic.plan.items.find(({ status }) => status === 'in_progress') ??
    [...epic.plan.items].reverse().find(({ status }) => status === 'completed');

  useEffect(() => {
    if (selectedSprint || !restoreSprintIdRef.current) return;
    const sprintId = restoreSprintIdRef.current;
    restoreSprintIdRef.current = null;
    Array.from(document.querySelectorAll<HTMLButtonElement>('[data-sprint-id]'))
      .find((button) => button.dataset.sprintId === sprintId)
      ?.focus();
  }, [selectedSprint]);

  if (selectedSprint?.workspace) {
    return (
      <SprintWorkspace
        workspace={selectedSprint.workspace}
        adjunct={selectedSprint.workspaceAdjunct}
        artifactAccessController={artifactAccessController}
        agentSessionComposition={agentSessionComposition}
        automaticContinuationPolicyController={sprintAutomaticContinuationPolicyController}
        selectedRevisionId={
          selectedRevisionId ?? selectedSprint.workspace.selectedSprintPlanRevisionId
        }
        onSelectedRevisionChange={onSelectedRevisionChange}
        detailLocation={detailLocation}
        onDetailLocationChange={onDetailLocationChange}
        onBack={() => {
          restoreSprintIdRef.current = selectedSprint.id;
          onCloseSprint();
        }}
        onOpenAgentSession={onOpenAgentSession}
        onRequestFileReview={onRequestFileReview}
      />
    );
  }

  return (
    <>
      <DetailWorkspace
        ariaLabel="Epic detail"
        controlsLabel="Epic controls"
        contextLabel="Epic context"
        backLabel="Back to Epics"
        onBack={onBack}
        control={
          epic.continuation ? (
            <ContinuationControl
              continuation={epic.continuation}
              controller={epicAutomaticContinuationPolicyController}
            />
          ) : null
        }
        context={
          <>
            <div className="orchestration-context-rail__epic">
              <p className="eyebrow">Epic</p>
              <h1>{epic.name}</h1>
            </div>
            {activeSprint && (
              <div className="orchestration-context-rail__sprint">
                <p className="eyebrow">Active Sprint</p>
                <strong>{activeSprint.name}</strong>
                <span
                  className={`sprint-context-status sprint-context-status--${
                    typeof activeSprint.status === 'string'
                      ? activeSprint.status
                      : activeSprint.status.kind
                  }`}
                >
                  {sprintStatusLabel(activeSprint.status)}
                </span>
              </div>
            )}
          </>
        }
        primary={
          <>
            {(epic.epicEscalationReceivers ?? []).length > 0 && (
              <section aria-label="Epic reassessment" className="orchestration-reassessment">
                <p className="eyebrow">Epic reassessment</p>
                <p>The Epic received the exact Sprint concern. The concern remains unresolved.</p>
                {(epic.epicEscalationReceivers ?? []).map((receiver) => (
                  <div key={`${receiver.epicId}:${receiver.sprintId}:${receiver.deliveryRequestedAt}`}>
                    <strong>Receiver delivery and reassessment</strong>
                    <p>Delivery requested: {receiver.deliveryRequestedAt}</p>
                    {receiver.semanticReassessmentRecordedAt && <p>Semantic reassessment recorded: {receiver.semanticReassessmentRecordedAt}</p>}
                    {receiver.disposition && <p>Disposition: {receiver.disposition.movementKind}. This is not Sprint selection, start, settlement, completion, or acceptance.</p>}
                  </div>
                ))}
              </section>
            )}
            {(epic.sprintResultProjections ?? []).length > 0 && (
              <section aria-label="Sprint result receipt" className="orchestration-reassessment">
                <p className="eyebrow">Sprint result receipt</p>
                <p>
                  These are separate local Sprint results and Epic receipt stages. They do not settle
                  the Epic, complete later Work Slices, record provider outcomes, or establish user
                  acceptance.
                </p>
                {(epic.sprintResultProjections ?? []).map((result) => {
                  const successorId = result.realization?.successorSprintId;
                  const successor = epic.plan.items.find((item) => item.id === successorId);
                  const successorTransition = successor?.workspace?.sprint.sprintRunnerTransition;
                  return (
                    <div key={result.resultId}>
                      <strong>Local Sprint result: {result.resultKind}</strong>
                      <p>Recorded: {result.recordedAt}</p>
                      {result.receiver ? (
                        <ul>
                          <li>Epic receipt requested: {result.receiver.deliveryRequestedAt}</li>
                          {result.receiver.deliveryPersistedAt && <li>Receipt delivery persisted: {result.receiver.deliveryPersistedAt}</li>}
                          {result.receiver.harnessBoundAt && <li>Application binding recorded: {result.receiver.harnessBoundAt}</li>}
                          {result.receiver.launchAcceptedAt && <li>Receiver launch accepted: {result.receiver.launchAcceptedAt}</li>}
                          {result.receiver.providerActivationObservedAt && <li>Provider activation observed separately: {result.receiver.providerActivationObservedAt}</li>}
                          {result.receiver.semanticReassessmentRecordedAt && <li>Semantic consideration recorded: {result.receiver.semanticReassessmentRecordedAt}</li>}
                        </ul>
                      ) : <p>Epic receipt has not been recorded.</p>}
                      {result.disposition && <p>Disposition recorded: {result.disposition.movementKind}. This preserves the result and grants no Sprint selection, start, settlement, completion, or acceptance authority.</p>}
                      {result.realization?.outcomeKind === 'retained_attention' && <p>Retained concern/attention recorded: {result.realization.retainedAttentionCode}. The concern remains open.</p>}
                      {result.realization?.outcomeKind === 'terminal_readiness' && <p>Terminal readiness recorded: {result.realization.terminalReadinessRecordedAt}. This is not Epic settlement, completion, or acceptance.</p>}
                      {result.realization?.outcomeKind === 'successor_request' && (
                        <>
                          {result.realization.successorRequestRecordedAt ? <p>Exact successor request recorded for {successor?.name ?? 'the next Sprint'}: {result.realization.successorRequestRecordedAt}.</p> : <p>Successor realization recorded; the exact successor request is not yet recorded.</p>}
                          {successorTransition?.launchAcceptedAt && <p>Successor pre-start launch accepted: {successorTransition.launchAcceptedAt}.</p>}
                          {successorTransition?.sprintStartPersistedAt && <p>Successor start persisted: {successorTransition.sprintStartPersistedAt}.</p>}
                          {successorTransition?.repositoryBranchReevaluationRecordedAt && <p>Successor repository/branch reevaluation recorded: {successorTransition.repositoryBranchReevaluationRecordedAt}.</p>}
                        </>
                      )}
                    </div>
                  );
                })}
              </section>
            )}
            <SprintPlan
              items={epic.plan.items}
              onOpen={(sprint, opener) => {
                setSelectedSprintOpener({ sprint, opener });
                onOpenSprint(sprint.id, sprint.workspace?.selectedSprintPlanRevisionId ?? '');
              }}
            />
          </>
        }
        agentSession={
          epic.epicRunnerSession ? (
            <SharedAgentSessionPanel
              ariaLabel="Epic Runner Agent Session"
              conversationAriaLabel="Epic Runner Agent Session conversation"
              session={epic.epicRunnerSession}
              composition={agentSessionComposition}
              onOpenStandalone={onOpenAgentSession}
            />
          ) : undefined
        }
      />

      {selectedSprint && !selectedSprint.workspace && selectedSprintOpener && (
        <SprintDetailDialog
          sprint={selectedSprint}
          restoreFocusTo={selectedSprintOpener.opener}
          onClose={() => onCloseSprint()}
        />
      )}
    </>
  );
}
