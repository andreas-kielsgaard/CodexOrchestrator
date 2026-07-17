import type { AgentSessionClient } from '../application/agentSessions';
import { StandaloneAgentSessionScreen } from '../features/agentSessions/AgentSessionScreen';
import type {
  ArtifactAccessController,
  SprintAutomaticContinuationPolicyController,
  EpicAutomaticContinuationPolicyController,
  EpicInitiationCapability,
  EpicPlanProposalSource,
  EpicPlanningDraftBinding,
  EpicPlanningDraftLifecycleClient,
  EpicPlanningDraftSummary,
  OrchestrationApplicationClient,
  EpicInitiationConfirmationClient,
} from '../application/orchestrations';
import {
  unavailableEpicPlanProposalSource,
  unavailableEpicInitiationCapability,
  unsupportedArtifactAccessController,
} from '../application/orchestrations';
import { EpicPlanBuilder, OrchestrationSection } from '../features/orchestrations';
import type { EmbeddedAgentSessionComposition } from '../features/agentSessions';
import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  productOrchestrationPresentationAdapter,
  type OrchestrationPresentationAdapter,
} from './orchestrationPresentation';
import { useOrchestrationLoad } from './useOrchestrationLoad';
import type { ManagedPlanBuilderSessionClient } from '../infrastructure/orchestrations/tauriManagedPlanBuilderSessionClient';
import { useEpicInitiationConfirmation } from './useEpicInitiationConfirmation';
import { EpicInitiationConfirmationModal } from './EpicInitiationConfirmationModal';

export interface AppProps {
  readonly agentSessionClient: AgentSessionClient;
  /** Plan Builder alone may use a managed send boundary; ordinary screens keep the generic client. */
  readonly managedPlanBuilderSessionClient?: ManagedPlanBuilderSessionClient;
  readonly orchestrationClient: OrchestrationApplicationClient;
  readonly orchestrationPresentation?: OrchestrationPresentationAdapter;
  readonly orchestrationAgentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly artifactAccessController?: ArtifactAccessController;
  readonly sprintAutomaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly epicAutomaticContinuationPolicyController?: EpicAutomaticContinuationPolicyController;
  readonly epicPlanProposalSource?: EpicPlanProposalSource;
  readonly epicInitiationCapability?: EpicInitiationCapability;
  readonly epicInitiationCapabilityForDraft?: (
    draftId: string,
  ) => Promise<EpicInitiationCapability>;
  readonly epicPlanningDraftLifecycleClient?: EpicPlanningDraftLifecycleClient;
  readonly epicPlanProposalSourceForDraft?: (draftId: string) => EpicPlanProposalSource;
  readonly epicInitiationConfirmationClient?: EpicInitiationConfirmationClient;
}

export function App({
  agentSessionClient,
  managedPlanBuilderSessionClient = {
    ...agentSessionClient,
    requestPlan: async () => {
      throw new Error('Plan Builder action is unavailable.');
    },
  },
  orchestrationClient,
  orchestrationPresentation = productOrchestrationPresentationAdapter,
  orchestrationAgentSessionComposition,
  artifactAccessController = unsupportedArtifactAccessController,
  sprintAutomaticContinuationPolicyController,
  epicAutomaticContinuationPolicyController,
  epicPlanProposalSource = unavailableEpicPlanProposalSource,
  epicInitiationCapability = unavailableEpicInitiationCapability,
  epicInitiationCapabilityForDraft,
  epicPlanningDraftLifecycleClient,
  epicPlanProposalSourceForDraft,
  epicInitiationConfirmationClient,
}: AppProps) {
  const [surface, setSurface] = useState<'epics' | 'agent-sessions'>('epics');
  const [orchestrationRoute, setOrchestrationRoute] = useState<'overview' | 'plan-builder'>(
    'overview',
  );
  const orchestrationLoad = useOrchestrationLoad(orchestrationClient);
  const [selectedDraft, setSelectedDraft] = useState<EpicPlanningDraftBinding | null>(null);
  const [planningDrafts, setPlanningDrafts] = useState<readonly EpicPlanningDraftSummary[]>([]);
  const [initiationCapability, setInitiationCapability] = useState<EpicInitiationCapability>(
    unavailableEpicInitiationCapability,
  );
  const refreshDrafts = useCallback(async () => {
    if (!epicPlanningDraftLifecycleClient) return true;
    try {
      setPlanningDrafts(
        (await epicPlanningDraftLifecycleClient.list()).filter(
          (draft) => draft.status === 'active',
        ),
      );
      return true;
    } catch {
      setPlanningDrafts([]);
      return false;
    }
  }, [epicPlanningDraftLifecycleClient]);
  useEffect(() => {
    void refreshDrafts();
  }, [refreshDrafts]);
  const embeddedComposition = useMemo(
    () => orchestrationAgentSessionComposition ?? { client: agentSessionClient },
    [agentSessionClient, orchestrationAgentSessionComposition],
  );
  const planProposalSource = useMemo(
    () =>
      selectedDraft
        ? (epicPlanProposalSourceForDraft?.(selectedDraft.draftId) ?? epicPlanProposalSource)
        : epicPlanProposalSource,
    [epicPlanProposalSource, epicPlanProposalSourceForDraft, selectedDraft],
  );
  const loadInitiationCapability = useCallback(
    async (draftId: string) =>
      epicInitiationCapabilityForDraft
        ? epicInitiationCapabilityForDraft(draftId)
        : epicInitiationCapability,
    [epicInitiationCapability, epicInitiationCapabilityForDraft],
  );
  useEffect(() => {
    let active = true;
    if (!selectedDraft) {
      setInitiationCapability(unavailableEpicInitiationCapability);
      return () => {
        active = false;
      };
    }
    setInitiationCapability({
      status: 'blocked',
      reason: 'Loading the current durable Epic Plan Proposal.',
    });
    void loadInitiationCapability(selectedDraft.draftId).then(
      (capability) => active && setInitiationCapability(capability),
      () =>
        active &&
        setInitiationCapability({
          status: 'blocked',
          reason: 'The current durable Epic Plan Proposal could not be loaded.',
        }),
    );
    return () => {
      active = false;
    };
  }, [loadInitiationCapability, selectedDraft]);
  const confirmInitiation = useCallback(async () => {
    if (selectedDraft) {
      setInitiationCapability({
        status: 'blocked',
        reason: 'Refreshing the confirmed durable Epic initiation.',
      });
    }
    const [orchestrationAvailable, draftsAvailable] = await Promise.all([
      orchestrationLoad.refresh(),
      refreshDrafts(),
    ]);
    let refreshAvailable = orchestrationAvailable && draftsAvailable;
    if (selectedDraft) {
      try {
        setInitiationCapability(await loadInitiationCapability(selectedDraft.draftId));
      } catch {
        refreshAvailable = false;
        setInitiationCapability({
          status: 'blocked',
          reason: 'Current initiation state is unavailable after durable confirmation.',
        });
      }
      await planProposalSource.refresh();
      if (planProposalSource.getSnapshot().kind === 'unavailable') refreshAvailable = false;
    }
    if (!refreshAvailable) throw new Error('post-confirmation application refresh unavailable');
  }, [
    loadInitiationCapability,
    orchestrationLoad,
    planProposalSource,
    refreshDrafts,
    selectedDraft,
  ]);
  const confirmation = useEpicInitiationConfirmation(
    epicInitiationConfirmationClient,
    confirmInitiation,
  );
  const refreshInitiationFailure = useCallback(async () => {
    if (!selectedDraft) return;
    const refreshedCapability = await loadInitiationCapability(selectedDraft.draftId);
    setInitiationCapability(refreshedCapability);
  }, [loadInitiationCapability, selectedDraft]);
  const bindCreatedPlanBuilderSession = useCallback(
    async (sessionId: string, title: string) => {
      if (!epicPlanningDraftLifecycleClient) return;
      try {
        const binding = await epicPlanningDraftLifecycleClient.reconcile(sessionId, title);
        setSelectedDraft(binding);
      } catch {
        // Managed send already owns creation. A catalog read can still recover that exact
        // durable binding without inventing a UI-local draft when reconciliation times out.
        try {
          const draft = (await epicPlanningDraftLifecycleClient.list()).find(
            (item) => item.agentSessionId === sessionId && item.status === 'active',
          );
          if (draft)
            setSelectedDraft({
              draftId: draft.epicPlanningDraftId,
              sessionId: draft.agentSessionId,
              ...(draft.title ? { title: draft.title } : {}),
            });
        } catch {
          // A failed acknowledgement never becomes a UI-local durable claim.
        }
      } finally {
        await refreshDrafts();
      }
    },
    [epicPlanningDraftLifecycleClient, refreshDrafts],
  );

  return (
    <div className="primary-app-shell">
      {confirmation.receiptError && (
        <p className="application-confirmation-error" role="alert">
          {confirmation.receiptError}
        </p>
      )}
      <EpicInitiationConfirmationModal confirmation={confirmation} />
      <nav className="surface-switcher" aria-label="Application surfaces">
        <button
          className={surface === 'epics' ? 'active' : undefined}
          type="button"
          aria-current={surface === 'epics' ? 'page' : undefined}
          onClick={() => setSurface('epics')}
        >
          Orchestration
        </button>
        <button
          className={surface === 'agent-sessions' ? 'active' : undefined}
          type="button"
          aria-current={surface === 'agent-sessions' ? 'page' : undefined}
          onClick={() => setSurface('agent-sessions')}
        >
          Agent Sessions
        </button>
      </nav>
      {surface === 'epics' && orchestrationRoute === 'plan-builder' ? (
        <EpicPlanBuilder
          agentSessionClient={managedPlanBuilderSessionClient}
          proposalSource={planProposalSource}
          initiationCapability={initiationCapability}
          onRequestInitiation={confirmation.requestButton}
          onInitiationFailure={refreshInitiationFailure}
          draft={selectedDraft ?? undefined}
          lifecycleClient={epicPlanningDraftLifecycleClient}
          onSessionCreated={bindCreatedPlanBuilderSession}
          onBack={() => {
            setSelectedDraft(null);
            setOrchestrationRoute('overview');
            void refreshDrafts();
          }}
        />
      ) : surface === 'epics' ? (
        <OrchestrationSurface
          load={orchestrationLoad}
          presentation={orchestrationPresentation}
          agentSessionComposition={embeddedComposition}
          artifactAccessController={artifactAccessController}
          sprintAutomaticContinuationPolicyController={sprintAutomaticContinuationPolicyController}
          epicAutomaticContinuationPolicyController={epicAutomaticContinuationPolicyController}
          planningDrafts={planningDrafts}
          onOpenDraft={(draft) => {
            setSelectedDraft({
              draftId: draft.epicPlanningDraftId,
              sessionId: draft.agentSessionId,
              ...(draft.title ? { title: draft.title } : {}),
            });
            setOrchestrationRoute('plan-builder');
          }}
          onPlanEpic={() => {
            setSelectedDraft(null);
            setOrchestrationRoute('plan-builder');
          }}
        />
      ) : (
        <StandaloneAgentSessionScreen client={agentSessionClient} />
      )}
    </div>
  );
}

function OrchestrationSurface({
  load,
  presentation,
  agentSessionComposition,
  artifactAccessController,
  sprintAutomaticContinuationPolicyController,
  epicAutomaticContinuationPolicyController,
  onPlanEpic,
  planningDrafts,
  onOpenDraft,
}: {
  readonly load: ReturnType<typeof useOrchestrationLoad>;
  readonly presentation: OrchestrationPresentationAdapter;
  readonly agentSessionComposition: EmbeddedAgentSessionComposition;
  readonly artifactAccessController: ArtifactAccessController;
  readonly sprintAutomaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly epicAutomaticContinuationPolicyController?: EpicAutomaticContinuationPolicyController;
  readonly onPlanEpic: () => void;
  readonly planningDrafts: readonly EpicPlanningDraftSummary[];
  readonly onOpenDraft: (draft: EpicPlanningDraftSummary) => void;
}) {
  if (load.kind === 'ready')
    return (
      <OrchestrationSection
        view={presentation.present(load.readModels)}
        agentSessionComposition={agentSessionComposition}
        artifactAccessController={artifactAccessController}
        sprintAutomaticContinuationPolicyController={sprintAutomaticContinuationPolicyController}
        epicAutomaticContinuationPolicyController={epicAutomaticContinuationPolicyController}
        onPlanEpic={onPlanEpic}
        planningDrafts={planningDrafts}
        onOpenPlanningDraft={onOpenDraft}
      />
    );
  const copy =
    load.kind === 'loading'
      ? 'Loading orchestration data…'
      : load.kind === 'failed'
        ? load.message
        : load.reason;
  return (
    <main
      className="orchestration-section"
      aria-label="Orchestration"
      aria-busy={load.kind === 'loading'}
    >
      <header className="orchestration-page-header">
        <p className="eyebrow">Orchestration</p>
        <h1>
          {load.kind === 'unavailable'
            ? 'Orchestration data unavailable'
            : 'Orchestration overview'}
        </h1>
        <p role={load.kind === 'loading' ? 'status' : 'alert'}>{copy}</p>
        <button className="orchestration-page-header__plan" type="button" onClick={onPlanEpic}>
          Plan an Epic
        </button>
      </header>
      {planningDrafts.length > 0 && (
        <section className="orchestration-list" aria-label="Active Epic planning drafts">
          <table>
            <tbody>
              {planningDrafts.map((draft) => (
                <tr key={draft.epicPlanningDraftId}>
                  <td>
                    <button
                      className="orchestration-list__open"
                      type="button"
                      onClick={() => onOpenDraft(draft)}
                    >
                      <strong>{draft.title ?? 'Untitled Epic draft'}</strong>
                      <small>Pre-initiation planning draft</small>
                    </button>
                  </td>
                  <td>Planning</td>
                  <td>Draft</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </main>
  );
}
