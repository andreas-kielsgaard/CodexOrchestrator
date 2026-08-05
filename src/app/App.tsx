import type { AgentIdentity, AgentSessionClient } from '../application/agentSessions';
import type { ConversationHarnessManagementSource } from '../application/conversationHarnesses';
import { StandaloneAgentSessionScreen } from '../features/agentSessions/AgentSessionScreen';
import type {
  ArtifactAccessController,
  SprintAutomaticContinuationPolicyController,
  EpicAutomaticContinuationPolicyController,
  EpicInitiationCapability,
  EpicPlanProposalSnapshot,
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
import {
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from 'react';
import {
  productOrchestrationPresentationAdapter,
  type OrchestrationPresentationAdapter,
} from './orchestrationPresentation';
import { useOrchestrationLoad } from './useOrchestrationLoad';
import type { ManagedPlanBuilderSessionClient } from '../infrastructure/orchestrations/tauriManagedPlanBuilderSessionClient';
import { useEpicInitiationConfirmation } from './useEpicInitiationConfirmation';
import { EpicInitiationConfirmationModal } from './EpicInitiationConfirmationModal';
import type {
  AgentSessionProductLocation,
  AgentSessionProductOrigin,
} from '../application/agentSessionNavigation';
import type { FileReviewSource } from '../application/fileReview';
import type {
  ContextualFileReviewClient,
  ContextualFileReviewResult,
} from '../application/contextualFileReview';
import { FileReviewScreen } from '../features/fileReview';
import type { WorkUnitActivitySessionTarget } from '../features/orchestrations/components/WorkUnitDetailWorkspace';
import { ProductCommandBar } from './ProductCommandBar';
import {
  canNavigateBack,
  createProductNavigation,
  productNavigationReducer,
  type ProductNavigationDestination,
} from '../application/productNavigation';

export type ApplicationSurface =
  'epics' | 'agent-sessions' | 'harness-inspector' | 'file-review' | 'worktree-review';

export interface AppProps {
  readonly agentSessionClient: AgentSessionClient;
  /** Plan Builder alone may use a managed send boundary; ordinary screens keep the generic client. */
  readonly managedPlanBuilderSessionClient?: ManagedPlanBuilderSessionClient;
  /** Session-owned identity read; assignment and durability remain outside this view. */
  readonly managedPlanBuilderAgentIdentity?: AgentIdentity;
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
  readonly agentSessionHarnessManagementSource?: ConversationHarnessManagementSource;
  readonly agentIdentityForSession?: (sessionId: string) => AgentIdentity | undefined;
  /** Present only in an injected development composition; production boot does not expose it. */
  readonly harnessManagementPreviewSurface?: ReactNode;
  readonly fileReviewSource?: FileReviewSource;
  readonly contextualFileReviewClient?: ContextualFileReviewClient;
  /** Explicit application-owned destinations only; display names never become file-review input. */
  readonly fileReviewSourceForEvidence?: (target: {
    readonly reviewId: string;
    readonly changedFileId: string;
  }) => FileReviewSource | undefined;
  /** Present only in the injected development launcher composition. */
  readonly humanReviewLauncherView?: ReactNode;
  /** Enumerated proof navigation; it cannot activate or focus a native window. */
  readonly humanReviewLauncherNavigation?: () => Promise<'worktree-review' | null>;
  readonly initialSurface?: ApplicationSurface;
}

export function App({
  agentSessionClient,
  managedPlanBuilderSessionClient = {
    ...agentSessionClient,
    requestPlan: async () => {
      throw new Error('Plan Builder action is unavailable.');
    },
  },
  managedPlanBuilderAgentIdentity,
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
  agentSessionHarnessManagementSource,
  agentIdentityForSession,
  harnessManagementPreviewSurface,
  fileReviewSource,
  contextualFileReviewClient,
  fileReviewSourceForEvidence,
  humanReviewLauncherView,
  humanReviewLauncherNavigation,
  initialSurface = 'epics',
}: AppProps) {
  const initialApplicationSurface: ApplicationSurface =
    (initialSurface === 'harness-inspector' && !harnessManagementPreviewSurface) ||
    (initialSurface === 'file-review' && !fileReviewSource) ||
    (initialSurface === 'worktree-review' && !humanReviewLauncherView)
      ? 'epics'
      : initialSurface;
  const [surface, setSurface] = useState<ApplicationSurface>(initialApplicationSurface);
  const productNavigationEpoch = useRef(0);
  const [contextualFileReviewSource, setContextualFileReviewSource] = useState<FileReviewSource>();
  const [fileReviewInitialFileId, setFileReviewInitialFileId] = useState<string>();
  const activeFileReviewSource = fileReviewSource ?? contextualFileReviewSource;
  const supportsProductDestination = useCallback(
    (destination: ProductNavigationDestination) => {
      switch (destination.kind) {
        case 'orchestration':
        case 'plan_builder':
        case 'agent_sessions':
          return true;
        case 'file_review':
          return Boolean(activeFileReviewSource);
        case 'harness_inspector':
          return Boolean(harnessManagementPreviewSurface);
        case 'worktree_review':
          return Boolean(humanReviewLauncherView);
      }
    },
    [activeFileReviewSource, harnessManagementPreviewSurface, humanReviewLauncherView],
  );
  const initialNavigationDestination: ProductNavigationDestination =
    initialApplicationSurface === 'agent-sessions'
      ? { kind: 'agent_sessions', selectedSessionId: null, focusedInvocationId: null }
      : initialApplicationSurface === 'harness-inspector'
        ? { kind: 'harness_inspector' }
        : initialApplicationSurface === 'worktree-review'
          ? { kind: 'worktree_review' }
          : { kind: 'orchestration', location: null };
  const [productNavigation, dispatchProductNavigation] = useReducer(
    (
      state: ReturnType<typeof createProductNavigation>,
      action: Parameters<typeof productNavigationReducer>[1],
    ) => productNavigationReducer(state, action, supportsProductDestination),
    createProductNavigation(initialNavigationDestination),
  );
  const selectedAgentSessionId =
    productNavigation.current.destination.kind === 'agent_sessions'
      ? productNavigation.current.destination.selectedSessionId
      : null;
  const focusedAgentSessionInvocationId =
    productNavigation.current.destination.kind === 'agent_sessions'
      ? (productNavigation.current.destination.focusedInvocationId ?? undefined)
      : undefined;
  const currentProductDestination = productNavigation.current.destination;
  const productReturnOrigin =
    productNavigation.current.destination.kind === 'agent_sessions' &&
    productNavigation.contextualOrigin &&
    supportsProductDestination({
      kind: 'orchestration',
      location: productNavigation.contextualOrigin.location,
    })
      ? productNavigation.contextualOrigin
      : null;
  const [expandedAgentSessionNodes, setExpandedAgentSessionNodes] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [requestedProductLocation, setRequestedProductLocation] =
    useState<AgentSessionProductLocation | null>(null);
  const [orchestrationRoute, setOrchestrationRoute] = useState<'overview' | 'plan-builder'>(
    'overview',
  );
  const orchestrationLoad = useOrchestrationLoad(orchestrationClient);
  const hasSynchronizedProductDestination = useRef(false);

  useEffect(() => {
    if (!hasSynchronizedProductDestination.current) {
      hasSynchronizedProductDestination.current = true;
      return;
    }
    const destination = currentProductDestination;
    if (destination.kind === 'orchestration') {
      setSurface('epics');
      setRequestedProductLocation(destination.location);
      setSelectedDraft(null);
      setOrchestrationRoute('overview');
    } else if (destination.kind === 'plan_builder') {
      setSurface('epics');
      setOrchestrationRoute('plan-builder');
    } else if (destination.kind === 'agent_sessions') {
      setSurface('agent-sessions');
    } else if (destination.kind === 'harness_inspector') {
      setSurface('harness-inspector');
    } else if (destination.kind === 'worktree_review') {
      setSurface('worktree-review');
    }
  }, [currentProductDestination]);

  useEffect(() => {
    if (!humanReviewLauncherView || !humanReviewLauncherNavigation) return;
    let active = true;
    const read = () =>
      void humanReviewLauncherNavigation().then(
        (route) => {
          if (active && route === 'worktree-review') {
            dispatchProductNavigation({
              type: 'navigate',
              intent: 'push',
              destination: { kind: 'worktree_review' },
            });
            setSurface(route);
          }
        },
        () => undefined,
      );
    read();
    const timer = window.setInterval(read, 300);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [humanReviewLauncherNavigation, humanReviewLauncherView]);
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
  const planProposalSnapshot = useSyncExternalStore(
    planProposalSource.subscribe,
    planProposalSource.getSnapshot,
    planProposalSource.getSnapshot,
  );
  const initiationCapabilityLoadSequence = useRef(0);
  const initiationCapabilityLoadTarget = useRef<{
    readonly draftId: string;
    readonly proposalSource: EpicPlanProposalSource;
    readonly proposalSnapshot: EpicPlanProposalSnapshot;
    readonly promise: Promise<'loaded' | 'failed' | 'superseded'>;
  } | null>(null);
  const reconcileInitiationCapability = useCallback(
    (
      draftId: string,
      proposalSource: EpicPlanProposalSource,
      proposalSnapshot: EpicPlanProposalSnapshot,
      options: {
        readonly force?: boolean;
        readonly loadingReason?: string;
        readonly failureReason?: string;
      } = {},
    ) => {
      const current = initiationCapabilityLoadTarget.current;
      if (
        !options.force &&
        current?.draftId === draftId &&
        current.proposalSource === proposalSource &&
        current.proposalSnapshot === proposalSnapshot
      )
        return current.promise;

      const sequence = ++initiationCapabilityLoadSequence.current;
      setInitiationCapability({
        status: 'blocked',
        reason: options.loadingReason ?? 'Loading the current durable Epic Plan Proposal.',
      });
      const promise = loadInitiationCapability(draftId).then(
        (capability) => {
          if (initiationCapabilityLoadSequence.current !== sequence) return 'superseded' as const;
          setInitiationCapability(capability);
          return 'loaded' as const;
        },
        () => {
          if (initiationCapabilityLoadSequence.current !== sequence) return 'superseded' as const;
          setInitiationCapability({
            status: 'blocked',
            reason:
              options.failureReason ??
              'The current durable Epic Plan Proposal could not be loaded.',
          });
          return 'failed' as const;
        },
      );
      initiationCapabilityLoadTarget.current = {
        draftId,
        proposalSource,
        proposalSnapshot,
        promise,
      };
      return promise;
    },
    [loadInitiationCapability],
  );
  useEffect(() => {
    if (!selectedDraft) {
      initiationCapabilityLoadSequence.current += 1;
      initiationCapabilityLoadTarget.current = null;
      setInitiationCapability(unavailableEpicInitiationCapability);
      return;
    }
    void reconcileInitiationCapability(
      selectedDraft.draftId,
      planProposalSource,
      planProposalSnapshot,
    );
  }, [planProposalSnapshot, planProposalSource, reconcileInitiationCapability, selectedDraft]);
  const confirmInitiation = useCallback(async () => {
    if (selectedDraft) {
      initiationCapabilityLoadSequence.current += 1;
      initiationCapabilityLoadTarget.current = null;
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
      const proposalBeforeRefresh = planProposalSource.getSnapshot();
      try {
        await planProposalSource.refresh();
      } catch {
        refreshAvailable = false;
      }
      const proposalAfterRefresh = planProposalSource.getSnapshot();
      const capabilityResult = await reconcileInitiationCapability(
        selectedDraft.draftId,
        planProposalSource,
        proposalAfterRefresh,
        {
          force: proposalBeforeRefresh === proposalAfterRefresh,
          loadingReason: 'Refreshing the confirmed durable Epic initiation.',
          failureReason: 'Current initiation state is unavailable after durable confirmation.',
        },
      );
      if (capabilityResult !== 'loaded') {
        refreshAvailable = false;
        if (capabilityResult === 'failed')
          setInitiationCapability({
            status: 'blocked',
            reason: 'Current initiation state is unavailable after durable confirmation.',
          });
      }
      if (proposalAfterRefresh.kind === 'unavailable') refreshAvailable = false;
    }
    if (!refreshAvailable) throw new Error('post-confirmation application refresh unavailable');
  }, [
    orchestrationLoad,
    planProposalSource,
    reconcileInitiationCapability,
    refreshDrafts,
    selectedDraft,
  ]);
  const confirmation = useEpicInitiationConfirmation(
    epicInitiationConfirmationClient,
    confirmInitiation,
  );
  const refreshInitiationFailure = useCallback(async () => {
    if (!selectedDraft) return;
    await reconcileInitiationCapability(
      selectedDraft.draftId,
      planProposalSource,
      planProposalSource.getSnapshot(),
      { force: true },
    );
  }, [planProposalSource, reconcileInitiationCapability, selectedDraft]);
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
  const openProductAgentSession = useCallback((origin: AgentSessionProductOrigin) => {
    productNavigationEpoch.current += 1;
    dispatchProductNavigation({
      type: 'navigate',
      intent: 'replace',
      destination: { kind: 'orchestration', location: origin.location },
    });
    dispatchProductNavigation({
      type: 'open_contextual_agent_session',
      origin,
      focusedInvocationId: null,
    });
    setSurface('agent-sessions');
  }, []);
  const openWorkUnitActivitySession = useCallback(
    (target: WorkUnitActivitySessionTarget, origin: AgentSessionProductOrigin) => {
      if (
        origin.location.kind !== 'work_unit' ||
        !origin.location.inspectionState ||
        origin.sessionId !== target.sessionId ||
        origin.invocationId !== target.invocationId ||
        origin.location.inspectionState.sessionId !== target.sessionId ||
        origin.location.inspectionState.invocationId !== target.invocationId ||
        origin.location.inspectionState.activityId !== target.activityId
      )
        return;
      productNavigationEpoch.current += 1;
      dispatchProductNavigation({
        type: 'navigate',
        intent: 'replace',
        destination: { kind: 'orchestration', location: origin.location },
      });
      dispatchProductNavigation({
        type: 'open_contextual_agent_session',
        origin,
        focusedInvocationId: target.invocationId,
      });
      setSurface('agent-sessions');
    },
    [],
  );
  const navigateToProductLocation = useCallback(
    (location: AgentSessionProductLocation) => {
      productNavigationEpoch.current += 1;
      if (location.kind === 'epic_planning_draft') {
        const draft = planningDrafts.find(
          ({ epicPlanningDraftId }) => epicPlanningDraftId === location.epicPlanningDraftId,
        );
        if (!draft) return;
        setSelectedDraft({
          draftId: draft.epicPlanningDraftId,
          sessionId: draft.agentSessionId,
          ...(draft.title ? { title: draft.title } : {}),
        });
        dispatchProductNavigation({
          type: 'navigate',
          intent: 'push',
          destination: {
            kind: 'plan_builder',
            epicPlanningDraftId: draft.epicPlanningDraftId,
          },
        });
        setOrchestrationRoute('plan-builder');
      } else {
        dispatchProductNavigation({
          type: 'navigate',
          intent: 'push',
          destination: { kind: 'orchestration', location },
        });
        setOrchestrationRoute('overview');
      }
      setSurface('epics');
    },
    [planningDrafts],
  );
  const requestContextualFileReview = useCallback(
    async (sprintId: string): Promise<ContextualFileReviewResult> => {
      if (!contextualFileReviewClient)
        return {
          status: 'failed',
          reason: 'unavailable',
          message: 'File Review is unavailable right now.',
        };
      let result: ContextualFileReviewResult;
      try {
        result = await contextualFileReviewClient.requestForSprint(sprintId);
      } catch {
        return {
          status: 'failed',
          reason: 'unavailable',
          message: 'File Review is unavailable right now.',
        };
      }
      if (result.status === 'ready') {
        setContextualFileReviewSource(result.source);
        setSurface('file-review');
      }
      return result;
    },
    [contextualFileReviewClient],
  );
  const openFileEvidence = useCallback(
    (target: { readonly reviewId: string; readonly changedFileId: string }) => {
      const source = fileReviewSourceForEvidence?.(target);
      if (!source) return;
      setContextualFileReviewSource(source);
      setFileReviewInitialFileId(target.changedFileId);
      dispatchProductNavigation({
        type: 'navigate',
        intent: 'push',
        destination: { kind: 'file_review', target: { kind: 'file_evidence', ...target } },
      });
      setSurface('file-review');
    },
    [fileReviewSourceForEvidence],
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
          onClick={() => {
            productNavigationEpoch.current += 1;
            dispatchProductNavigation({
              type: 'navigate',
              intent: 'push',
              destination: { kind: 'orchestration', location: null },
            });
            setSurface('epics');
          }}
        >
          Orchestration
        </button>
        <button
          className={surface === 'agent-sessions' ? 'active' : undefined}
          type="button"
          aria-current={surface === 'agent-sessions' ? 'page' : undefined}
          onClick={() => {
            productNavigationEpoch.current += 1;
            dispatchProductNavigation({ type: 'enter_agent_sessions_directly' });
            setSurface('agent-sessions');
          }}
        >
          Agent Sessions
        </button>
        {harnessManagementPreviewSurface && (
          <button
            className={surface === 'harness-inspector' ? 'active' : undefined}
            type="button"
            aria-current={surface === 'harness-inspector' ? 'page' : undefined}
            onClick={() => {
              productNavigationEpoch.current += 1;
              dispatchProductNavigation({
                type: 'navigate',
                intent: 'push',
                destination: { kind: 'harness_inspector' },
              });
              setSurface('harness-inspector');
            }}
          >
            Harness Management
          </button>
        )}
        {humanReviewLauncherView && (
          <button
            className={surface === 'worktree-review' ? 'active' : undefined}
            type="button"
            aria-current={surface === 'worktree-review' ? 'page' : undefined}
            onClick={() => {
              productNavigationEpoch.current += 1;
              dispatchProductNavigation({
                type: 'navigate',
                intent: 'push',
                destination: { kind: 'worktree_review' },
              });
              setSurface('worktree-review');
            }}
          >
            Worktree Review <small>Dev</small>
          </button>
        )}
        {fileReviewSource ? (
          <button
            className={surface === 'file-review' ? 'active' : undefined}
            type="button"
            aria-current={surface === 'file-review' ? 'page' : undefined}
            onClick={() => {
              productNavigationEpoch.current += 1;
              dispatchProductNavigation({ type: 'clear_contextual_origin' });
              setSurface('file-review');
            }}
          >
            Files &amp; diffs
          </button>
        ) : null}
      </nav>
      <ProductCommandBar
        canGoBack={canNavigateBack(productNavigation, supportsProductDestination)}
        onBack={() => {
          productNavigationEpoch.current += 1;
          dispatchProductNavigation({ type: 'back' });
        }}
        returnOrigin={productReturnOrigin}
        onReturn={(origin) => {
          if (origin !== productReturnOrigin) return;
          productNavigationEpoch.current += 1;
          dispatchProductNavigation({ type: 'return_to_contextual_origin', origin });
        }}
      />
      {surface === 'epics' && orchestrationRoute === 'plan-builder' ? (
        <EpicPlanBuilder
          agentSessionClient={managedPlanBuilderSessionClient}
          agentIdentity={managedPlanBuilderAgentIdentity}
          proposalSource={planProposalSource}
          initiationCapability={initiationCapability}
          onRequestInitiation={confirmation.requestButton}
          onInitiationFailure={refreshInitiationFailure}
          draft={selectedDraft ?? undefined}
          lifecycleClient={epicPlanningDraftLifecycleClient}
          harnessManagementSource={agentSessionHarnessManagementSource}
          onSessionCreated={bindCreatedPlanBuilderSession}
          onBack={() => {
            dispatchProductNavigation({ type: 'back' });
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
            dispatchProductNavigation({
              type: 'navigate',
              intent: 'push',
              destination: {
                kind: 'plan_builder',
                epicPlanningDraftId: draft.epicPlanningDraftId,
              },
            });
            setOrchestrationRoute('plan-builder');
          }}
          onPlanEpic={() => {
            setSelectedDraft(null);
            dispatchProductNavigation({
              type: 'navigate',
              intent: 'push',
              destination: { kind: 'plan_builder', epicPlanningDraftId: null },
            });
            setOrchestrationRoute('plan-builder');
          }}
          requestedLocation={requestedProductLocation}
          onOpenAgentSession={openProductAgentSession}
          onOpenWorkUnitActivitySession={openWorkUnitActivitySession}
          onRequestFileReview={contextualFileReviewClient ? requestContextualFileReview : undefined}
          onOpenFileEvidence={fileReviewSourceForEvidence ? openFileEvidence : undefined}
        />
      ) : surface === 'file-review' && activeFileReviewSource ? (
        <FileReviewScreen source={activeFileReviewSource} initialFileId={fileReviewInitialFileId} />
      ) : surface === 'worktree-review' && humanReviewLauncherView ? (
        humanReviewLauncherView
      ) : surface === 'agent-sessions' ? (
        <StandaloneAgentSessionScreen
          client={agentSessionClient}
          harnessManagementSource={agentSessionHarnessManagementSource}
          agentIdentityForSession={agentIdentityForSession}
          orchestrations={
            orchestrationLoad.kind === 'ready' ? orchestrationLoad.readModels : undefined
          }
          planningDrafts={planningDrafts}
          selectedSessionId={selectedAgentSessionId}
          focusInvocationId={focusedAgentSessionInvocationId}
          returnOrigin={productReturnOrigin}
          onSelectedSessionChange={(() => {
            const renderEpoch = productNavigationEpoch.current;
            return (sessionId: string | null) => {
              if (renderEpoch !== productNavigationEpoch.current) return;
              dispatchProductNavigation({
                type: 'navigate',
                intent: 'replace',
                destination: {
                  kind: 'agent_sessions',
                  selectedSessionId: sessionId,
                  focusedInvocationId: null,
                },
              });
            };
          })()}
          expandedNodeIds={expandedAgentSessionNodes}
          onExpandedNodeIdsChange={setExpandedAgentSessionNodes}
          onNavigateToProduct={navigateToProductLocation}
        />
      ) : (
        harnessManagementPreviewSurface
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
  requestedLocation,
  onOpenAgentSession,
  onOpenWorkUnitActivitySession,
  onRequestFileReview,
  onOpenFileEvidence,
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
  readonly requestedLocation: AgentSessionProductLocation | null;
  readonly onOpenAgentSession: (origin: AgentSessionProductOrigin) => void;
  readonly onOpenWorkUnitActivitySession: (
    target: WorkUnitActivitySessionTarget,
    origin: AgentSessionProductOrigin,
  ) => void;
  readonly onRequestFileReview?: (sprintId: string) => Promise<ContextualFileReviewResult>;
  readonly onOpenFileEvidence?: (target: {
    readonly reviewId: string;
    readonly changedFileId: string;
  }) => void;
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
        requestedLocation={requestedLocation}
        onOpenAgentSession={onOpenAgentSession}
        onOpenWorkUnitActivitySession={onOpenWorkUnitActivitySession}
        onRequestFileReview={onRequestFileReview}
        onOpenFileEvidence={onOpenFileEvidence}
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
