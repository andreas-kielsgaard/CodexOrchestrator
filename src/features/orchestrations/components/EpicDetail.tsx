import { ArrowLeft } from 'lucide-react';
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
import type {
  AgentSessionProductLocation,
  AgentSessionProductOrigin,
} from '../../../application/agentSessionNavigation';
import type { WorkUnitActivitySessionTarget } from './WorkUnitDetailWorkspace';
import type {
  EpicProductDecisionSource,
  ProductDecisionClient,
  ProductDecisionCorrectionClient,
  ProductDecisionEvidenceDestination,
  ProductDecisionEvidenceNavigationRequest,
  ProductDecisionPublishTarget,
} from '../../../application/productDecisions';
import { EpicProductDecisionsPanel } from '../../productDecisions';

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
  readonly globalBackAvailable?: boolean;
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
  readonly epicProductDecisionSource?: EpicProductDecisionSource;
  readonly productDecisionClient?: ProductDecisionClient;
  readonly productDecisionCorrectionClient?: ProductDecisionCorrectionClient;
  readonly requestedProductDecisions?: boolean;
  readonly onOpenProductDecisionEvidence?: (
    request: ProductDecisionEvidenceNavigationRequest,
    origin: AgentSessionProductOrigin,
  ) => void;
  readonly onOpenProductiveDecisionEvidence?: (
    destination: ProductDecisionEvidenceDestination,
    origin: AgentSessionProductOrigin,
  ) => void;
  readonly onPublishProductDecision?: (target: ProductDecisionPublishTarget) => void;
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
  globalBackAvailable = false,
  onOpenAgentSession,
  onRequestFileReview,
  onOpenFileEvidence,
  onOpenWorkUnitActivitySession,
  epicProductDecisionSource,
  productDecisionClient,
  productDecisionCorrectionClient,
  requestedProductDecisions = false,
  onOpenProductDecisionEvidence,
  onOpenProductiveDecisionEvidence,
  onPublishProductDecision,
}: EpicDetailProps) {
  const restoreSprintIdRef = useRef<string | null>(null);
  const [selectedSprintOpener, setSelectedSprintOpener] = useState<{
    readonly sprint: SprintPlanItemPresentation;
    readonly opener: HTMLButtonElement;
  } | null>(null);
  const [epicSection, setEpicSection] = useState<'plan' | 'product-decisions'>(
    requestedProductDecisions ? 'product-decisions' : 'plan',
  );
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

  useEffect(() => {
    setEpicSection(requestedProductDecisions ? 'product-decisions' : 'plan');
  }, [requestedProductDecisions]);

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
        globalBackAvailable={globalBackAvailable}
        onOpenAgentSession={onOpenAgentSession}
        onRequestFileReview={onRequestFileReview}
        onOpenFileEvidence={onOpenFileEvidence}
        onOpenWorkUnitActivitySession={onOpenWorkUnitActivitySession}
      />
    );
  }

  if (epicSection === 'product-decisions' && (epicProductDecisionSource || productDecisionClient)) {
    return (
      <main
        className="epic-product-decisions-view"
        aria-label="Epic Product Decisions"
        data-viewport-contained="true"
        data-view-layout="single-column"
      >
        <div className="epic-product-decisions-view__menu" aria-label="Epic controls">
          {!globalBackAvailable && (
            <button className="epic-product-decisions-view__back" type="button" onClick={onBack}>
              <ArrowLeft size={16} aria-hidden="true" />
              Back to Epics
            </button>
          )}
          <EpicIdentity epicName={epic.name} />
          <EpicViewNavigation current="product-decisions" onChange={setEpicSection} />
        </div>
        <div className="epic-product-decisions-view__content">
          <EpicProductDecisionsPanel
            epicId={epic.id}
            source={epicProductDecisionSource}
            productiveClient={productDecisionClient}
            correctionClient={productDecisionCorrectionClient}
            agentSessionClient={agentSessionComposition?.client}
            onOpenEvidence={(request) => {
              if (!epicProductDecisionSource) return;
              const resolution = epicProductDecisionSource.resolveEvidenceNavigation(request);
              if (resolution.kind !== 'available') return;
              onOpenProductDecisionEvidence?.(request, {
                sessionId: resolution.destination.sessionId,
                invocationId: resolution.destination.invocationId,
                location: { kind: 'epic_product_decisions', epicId: epic.id, label: epic.name },
              });
            }}
            onOpenProductiveEvidence={(destination) => {
              onOpenProductiveDecisionEvidence?.(destination, {
                sessionId: destination.sessionId,
                invocationId: destination.invocationId,
                location: { kind: 'epic_product_decisions', epicId: epic.id, label: epic.name },
              });
            }}
            onPublish={onPublishProductDecision}
          />
        </div>
      </main>
    );
  }

  return (
    <>
      <DetailWorkspace
        ariaLabel="Epic detail"
        controlsLabel="Epic controls"
        contextLabel="Epic context"
        backLabel={globalBackAvailable ? undefined : 'Back to Epics'}
        onBack={onBack}
        showBack={!globalBackAvailable}
        hotbarContext={<EpicIdentity epicName={epic.name} />}
        hotbarNavigation={
          epicProductDecisionSource || productDecisionClient ? (
            <EpicViewNavigation current="plan" onChange={setEpicSection} />
          ) : undefined
        }
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
                  <div
                    key={`${receiver.epicId}:${receiver.sprintId}:${receiver.deliveryRequestedAt}`}
                  >
                    <strong>Receiver delivery and reassessment</strong>
                    <p>Delivery requested: {receiver.deliveryRequestedAt}</p>
                    {receiver.semanticReassessmentRecordedAt && (
                      <p>
                        Semantic reassessment recorded: {receiver.semanticReassessmentRecordedAt}
                      </p>
                    )}
                    {receiver.disposition && (
                      <p>
                        Disposition: {receiver.disposition.movementKind}. This is not Sprint
                        selection, start, settlement, completion, or acceptance.
                      </p>
                    )}
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
            {epic.epicSettlement && (
              <section aria-label="Epic settlement" className="orchestration-reassessment">
                <p className="eyebrow">Epic settlement</p>
                {epic.epicSettlement.kind === 'settled' ? (
                  <p>
                    Settlement was recorded at {epic.epicSettlement.persistedAt}. This records the
                    Epic settlement only; other downstream outcomes are not established here.
                  </p>
                ) : (
                  <>
                    <p>Epic settlement remains unresolved.</p>
                    <p>Reason recorded: {epic.epicSettlement.reasonCode}.</p>
                    <p>Resume by: {epic.epicSettlement.resumptionFact}</p>
                    <p>Unresolved state recorded at {epic.epicSettlement.recordedAt}.</p>
                  </>
                )}
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
              onOpenStandalone={(sessionId) =>
                onOpenAgentSession?.({
                  sessionId,
                  location: {
                    kind: 'epic',
                    epicId: epic.id,
                    label: epic.name,
                  },
                })
              }
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

function EpicIdentity({ epicName }: { readonly epicName: string }) {
  return (
    <span className="epic-detail__identity" aria-label={`Current Epic: ${epicName}`}>
      <small>Epic</small>
      <strong>{epicName}</strong>
    </span>
  );
}

function EpicViewNavigation({
  current,
  onChange,
}: {
  readonly current: 'plan' | 'product-decisions';
  readonly onChange: (view: 'plan' | 'product-decisions') => void;
}) {
  return (
    <div className="epic-detail__section-switch" aria-label="Epic views">
      <button
        type="button"
        aria-current={current === 'plan' ? 'page' : undefined}
        onClick={() => onChange('plan')}
      >
        Plan
      </button>
      <button
        type="button"
        aria-current={current === 'product-decisions' ? 'page' : undefined}
        onClick={() => onChange('product-decisions')}
      >
        Product decisions
      </button>
    </div>
  );
}
