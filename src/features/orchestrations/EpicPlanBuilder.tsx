import { ArrowLeft, Play, Sparkles, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useState, useSyncExternalStore } from 'react';
import type {
  AgentSessionClient,
  SendAgentSessionMessageCommandDto,
} from '../../application/agentSessions';
import type { ConversationHarnessManagementSource } from '../../application/conversationHarnesses';
import type {
  EpicPlanningDraftBinding,
  EpicPlanningDraftLifecycleClient,
  EpicInitiationCapability,
  EpicPlanProposalSnapshot,
  EpicPlanProposalSource,
} from '../../application/orchestrations';
import {
  epicInitiationErrorMessage,
  managedPlanBuilderSessionConfiguration,
} from '../../application/orchestrations';
import { AgentSessionWorkspace, useAgentSession } from '../agentSessions';
import { AgentIdentityBadge } from '../../components/AgentIdentityBadge';
import { HarnessAwareAgentSessionPane } from '../conversationHarnesses';
import './styles/epicPlanBuilder.css';

export const BUILD_EPIC_PLAN_PROMPT = 'Build the epic plan based on what we have discussed';

export interface EpicPlanBuilderProps {
  readonly agentSessionClient: AgentSessionClient & {
    requestPlan(
      command: Omit<SendAgentSessionMessageCommandDto, 'submittedText'>,
    ): Promise<unknown>;
  };
  readonly proposalSource: EpicPlanProposalSource;
  readonly initiationCapability?: EpicInitiationCapability;
  /** App-owned shared confirmation request; opening the popup is not confirmation. */
  onRequestInitiation?(
    input: Extract<EpicInitiationCapability, { status: 'ready' }>['request'],
  ): Promise<void>;
  /** Reloads per-draft durable authority after a command failure so a retry is meaningful. */
  onInitiationFailure?(): Promise<void>;
  /** The shared controller emits this only after a successful first normal send. */
  onSessionCreated?(sessionId: string, title: string): void;
  onBack(): void;
  readonly draft?: EpicPlanningDraftBinding;
  readonly lifecycleClient?: EpicPlanningDraftLifecycleClient;
  readonly harnessManagementSource?: ConversationHarnessManagementSource;
}

/** One normal-app workspace: the shared conversation is primary; the proposal is source-owned. */
export function EpicPlanBuilder({
  agentSessionClient,
  proposalSource,
  initiationCapability = {
    status: 'blocked',
    reason: 'Select an active Epic Planning Draft with a current proposal before initiation.',
  },
  onRequestInitiation,
  onInitiationFailure,
  onSessionCreated,
  onBack,
  draft,
  lifecycleClient,
  harnessManagementSource,
}: EpicPlanBuilderProps) {
  const proposal = useSyncExternalStore(
    proposalSource.subscribe,
    proposalSource.getSnapshot,
    proposalSource.getSnapshot,
  );
  useEffect(() => {
    void proposalSource.refresh();
  }, [proposalSource]);
  const [epicName, setEpicName] = useState(draft?.title ?? '');
  const [hasUserEnteredName, setHasUserEnteredName] = useState(false);
  const [cancelingDraft, setCancelingDraft] = useState(false);
  const [cancelConfirmationOpen, setCancelConfirmationOpen] = useState(false);
  const [cancelDraftError, setCancelDraftError] = useState<string | null>(null);
  const [planRequestPending, setPlanRequestPending] = useState(false);
  const [planRequestError, setPlanRequestError] = useState<string | null>(null);
  const [initiationError, setInitiationError] = useState<string | null>(null);
  const [initiatingEpic, setInitiatingEpic] = useState(false);
  const alreadyInitiated = initiationCapability.status === 'already_initiated';
  const suggestedName = proposal.kind === 'available' ? proposal.suggestedEpicName : undefined;
  useEffect(() => {
    if (!hasUserEnteredName && suggestedName) setEpicName(suggestedName);
  }, [hasUserEnteredName, suggestedName]);
  const session = useAgentSession(agentSessionClient, {
    selectedSessionId: draft?.sessionId ?? null,
    sessionTitle: managedPlanBuilderSessionConfiguration.titleForEpicName(epicName),
    onSessionCreated: (sessionId) =>
      onSessionCreated?.(
        sessionId,
        managedPlanBuilderSessionConfiguration.titleForEpicName(epicName),
      ),
  });
  const agentIdentity = session.details?.session.agentIdentity ?? null;
  // Session loads are a re-query fallback for missed notifications and restart recovery.
  useEffect(() => {
    if (session.details) void proposalSource.refresh();
  }, [proposalSource, session.details]);
  // The terminal update is the persisted Agent Session boundary after managed MCP work has
  // completed. The source is still re-queried rather than projecting from the update payload.
  useEffect(() => {
    let canceled = false;
    let unsubscribe: (() => void) | undefined;
    void agentSessionClient
      .subscribeUpdates((update) => {
        if (update.kind === 'invocation_terminal') void proposalSource.refresh();
      })
      .then((nextUnsubscribe) => {
        if (canceled) nextUnsubscribe();
        else unsubscribe = nextUnsubscribe;
      })
      .catch(() => undefined);
    return () => {
      canceled = true;
      unsubscribe?.();
    };
  }, [agentSessionClient, proposalSource]);
  const userTurns = useMemo(
    () => session.transcript?.invocations.filter((turn) => turn.inputProvenance === 'user') ?? [],
    [session.transcript?.invocations],
  );
  const planAction = useMemo(() => {
    const proposalRecordedAt = proposal.kind === 'available' ? proposal.revision?.recordedAt : null;
    const hasConversationAfterProposal =
      proposal.kind === 'unavailable'
        ? userTurns.length > 0
        : Boolean(
            proposalRecordedAt && userTurns.some((turn) => turn.createdAt > proposalRecordedAt),
          );
    return {
      label: proposal.kind === 'available' ? 'Rebuild plan' : 'Plan Epic',
      enabled:
        userTurns.length > 0 &&
        hasConversationAfterProposal &&
        !planRequestPending &&
        !session.sending &&
        !session.transcript?.activeInvocationId,
    };
  }, [planRequestPending, proposal, session.sending, session.transcript, userTurns]);

  const requestPlan = async () => {
    if (!planAction.enabled || planRequestPending) return;
    setPlanRequestPending(true);
    setPlanRequestError(null);
    try {
      await agentSessionClient.requestPlan({
        sessionId: draft?.sessionId ?? session.selectedSessionId ?? undefined,
        title: managedPlanBuilderSessionConfiguration.titleForEpicName(epicName),
      });
      await session.reload();
    } catch {
      setPlanRequestError('The Plan Builder request could not be started.');
    } finally {
      // A command acknowledgement is not durable proposal evidence. Re-query even on failure so
      // an authoritative unavailable result clears a stale proposal projection.
      await proposalSource.refresh();
      setPlanRequestPending(false);
    }
  };

  const initiateEpic = async () => {
    if (initiationCapability.status !== 'ready' || initiatingEpic) return;
    setInitiatingEpic(true);
    setInitiationError(null);
    try {
      if (!onRequestInitiation) throw new Error('Confirmation is unavailable.');
      await onRequestInitiation(initiationCapability.request);
    } catch (error) {
      await proposalSource.refresh();
      await onInitiationFailure?.().catch(() => undefined);
      setInitiationError(epicInitiationErrorMessage(error));
    } finally {
      setInitiatingEpic(false);
    }
  };

  const cancelDraft = async () => {
    if (!draft || !lifecycleClient || cancelingDraft) return;
    setCancelDraftError(null);
    setCancelingDraft(true);
    try {
      await lifecycleClient.cancel(draft);
      setCancelConfirmationOpen(false);
      onBack();
    } catch {
      setCancelDraftError('The planning draft could not be canceled. It remains active.');
      setCancelConfirmationOpen(false);
    } finally {
      setCancelingDraft(false);
    }
  };
  const conversation = (
    <AgentSessionWorkspace
      controller={session}
      presentation={{
        showHeader: false,
        ariaLabel: 'Epic Plan Builder conversation',
        emptyState: {
          heading: 'Let’s build a plan',
          guidance: 'Paste a prepared Epic description or begin discussing what you want to build.',
        },
        composer: {
          messageLabel: 'Describe what we are working on',
          messagePlaceholder: 'Describe what we are working on',
          keyboardHint: 'tooltip',
        },
      }}
    />
  );

  return (
    <main className="epic-plan-builder" aria-label="Plan an Epic">
      <div className="epic-plan-builder__toolbar" aria-label="Epic planning draft actions">
        <button className="epic-plan-builder__back" type="button" onClick={onBack}>
          <ArrowLeft size={16} aria-hidden="true" />
          Back to orchestration overview
        </button>
        {agentIdentity && <AgentIdentityBadge identity={agentIdentity} compact />}
        <span className="epic-plan-builder__toolbar-context">
          {agentIdentity
            ? `${agentIdentity.name}: ${agentIdentity.harnessRole}`
            : alreadyInitiated
              ? 'Epic initiation confirmed'
              : 'Pre-initiation Epic planning draft'}
        </span>
        {draft && lifecycleClient && !alreadyInitiated && (
          <button
            className="epic-plan-builder__cancel-draft"
            type="button"
            disabled={cancelingDraft}
            onClick={() => setCancelConfirmationOpen(true)}
          >
            <Trash2 size={15} aria-hidden="true" />
            {cancelingDraft ? 'Canceling draft…' : 'Cancel draft'}
          </button>
        )}
      </div>
      {cancelConfirmationOpen && (
        <div className="epic-initiation-confirmation" role="presentation">
          <section
            className="epic-initiation-confirmation__dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="cancel-draft-title"
          >
            <h2 id="cancel-draft-title">Cancel this planning draft?</h2>
            <p>
              Its Agent Session history will be kept, but the draft will no longer appear as active.
            </p>
            {cancelDraftError && <p role="alert">{cancelDraftError}</p>}
            <div className="epic-initiation-confirmation__actions">
              <button
                type="button"
                disabled={cancelingDraft}
                onClick={() => setCancelConfirmationOpen(false)}
              >
                Keep draft
              </button>
              <button type="button" disabled={cancelingDraft} onClick={() => void cancelDraft()}>
                {cancelingDraft ? 'Canceling draft…' : 'Cancel draft'}
              </button>
            </div>
          </section>
        </div>
      )}
      <div className="epic-plan-builder__body">
        {cancelDraftError && (
          <p className="epic-plan-builder__cancel-error" role="alert">
            {cancelDraftError}
          </p>
        )}
        <div className="epic-plan-builder__layout">
          <aside className="epic-plan-builder__controls" aria-label="Epic planning controls">
            <div className="epic-plan-builder__controls-primary">
              <p className="eyebrow">Epic planning draft</p>
              <h1>Plan an Epic</h1>
              <label htmlFor="epic-plan-builder-name">Epic name</label>
              <input
                id="epic-plan-builder-name"
                className="epic-plan-builder__name"
                aria-label="Epic name"
                value={epicName}
                onChange={(event) => {
                  setHasUserEnteredName(true);
                  setEpicName(event.target.value);
                }}
                onBlur={() => {
                  if (draft && lifecycleClient) void lifecycleClient.updateTitle(draft, epicName);
                }}
                placeholder="Name this Epic"
              />
            </div>
            <div className="epic-plan-builder__controls-actions">
              <button
                className="epic-plan-builder__plan-action"
                type="button"
                disabled={!planAction.enabled}
                onClick={() => void requestPlan()}
              >
                <Sparkles size={16} aria-hidden="true" />
                {planAction.label}
              </button>
              <button
                className="epic-plan-builder__initiate-action"
                type="button"
                disabled={initiationCapability.status !== 'ready' || initiatingEpic}
                aria-describedby={
                  initiationCapability.status !== 'ready'
                    ? 'epic-initiation-unavailable'
                    : undefined
                }
                onClick={() => void initiateEpic()}
              >
                <Play size={16} aria-hidden="true" />
                {initiatingEpic
                  ? 'Requesting confirmation…'
                  : initiationCapability.status === 'already_initiated'
                    ? 'Epic already initiated'
                    : 'Initiate Epic'}
              </button>
              {initiatingEpic && <p role="status">Opening Epic initiation confirmation…</p>}
              {initiationCapability.status !== 'ready' && (
                <p id="epic-initiation-unavailable">{initiationCapability.reason}</p>
              )}
              {planRequestError && <p role="alert">{planRequestError}</p>}
              {initiationError && <p role="alert">{initiationError}</p>}
            </div>
          </aside>
          <div className="epic-plan-builder__conversation">
            {harnessManagementSource && session.selectedSessionId ? (
              <HarnessAwareAgentSessionPane
                sessionId={session.selectedSessionId}
                source={harnessManagementSource}
              >
                {conversation}
              </HarnessAwareAgentSessionPane>
            ) : (
              conversation
            )}
          </div>
          <aside
            className="epic-plan-builder__proposal"
            aria-labelledby="proposed-epic-plan-heading"
          >
            <header className="epic-plan-builder__proposal-header">
              <h2 id="proposed-epic-plan-heading">
                {agentIdentity ? `${agentIdentity.name}’s Proposed Plan:` : 'Proposed Epic plan'}
              </h2>
            </header>
            <div className="epic-plan-builder__proposal-body">
              {proposal.kind === 'available' ? (
                <ol>
                  {proposal.sprints.map((sprint, index) => (
                    <ProposedSprintCard key={sprint.title} sprint={sprint} number={index + 1} />
                  ))}
                </ol>
              ) : (
                <p className="epic-plan-builder__empty" role="status">
                  The Epic Plan Builder will organize the emerging plan into proposed Sprints with
                  bounded objectives and concerns.
                </p>
              )}
            </div>
          </aside>
        </div>
      </div>
    </main>
  );
}

function ProposedSprintCard({
  sprint,
  number,
}: {
  readonly sprint: Extract<EpicPlanProposalSnapshot, { kind: 'available' }>['sprints'][number];
  readonly number: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const detailsId = `proposed-sprint-${number}`;
  const toggle = () => setExpanded((current) => !current);
  return (
    <li className="epic-plan-builder__sprint" onClick={toggle}>
      <button
        className="epic-plan-builder__sprint-toggle"
        type="button"
        aria-expanded={expanded}
        aria-controls={detailsId}
        aria-label={`Sprint ${number} ${sprint.title}`}
        onClick={(event) => {
          event.stopPropagation();
          toggle();
        }}
      />
      <div className="epic-plan-builder__sprint-card">
        <div className="epic-plan-builder__sprint-heading">
          <span>Sprint {number}</span>
          <strong>{sprint.title}</strong>
        </div>
        <div className="epic-plan-builder__sprint-details" id={detailsId} hidden={!expanded}>
          <p>{sprint.intendedMovement}</p>
          <ul aria-label={`Concerns addressed by ${sprint.title}`}>
            {sprint.concernSummaries.map((concern) => (
              <li key={concern}>{concern}</li>
            ))}
          </ul>
        </div>
      </div>
    </li>
  );
}
