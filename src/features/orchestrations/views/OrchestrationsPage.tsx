import {
  AlertCircle,
  Check,
  Edit3,
  GitBranch,
  LoaderCircle,
  Play,
  Plus,
  ScrollText,
  Upload,
  X,
} from 'lucide-react';
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type DragEvent,
  type FormEvent,
  type ReactNode,
} from 'react';
import type { EntityId } from '../../../domain/model';
import {
  getOrchestrationStatusDescription,
  getOrchestrationStatusLabel,
  isMockOrUnsupported,
  type OrchestrationTruthState,
} from '../../../domain/orchestrationState';
import {
  integrationPendingTruthState,
  localDraftTruthState,
  mapBuildPackageToAgentConversation,
  type BlockerConclusion,
  type OrchestrationAgentWindow,
  type OrchestrationBlocker,
  type OrchestrationBlockerState,
  type OrchestrationBuildPackage,
  type OrchestrationBuildStage,
  type OrchestrationBuildStageId,
  type OrchestrationClient,
  type OrchestrationConversationMessage,
  type OrchestrationPlanNode,
  type OrchestrationPlannerTurn,
  type OrchestrationSnapshot,
  type OrchestrationStep,
  type OrchestrationWorkSlice,
  type UploadedConversationFile,
} from '../../../application/orchestrationClient';
import {
  AgentConversationView,
  ConversationThread,
  CurrentAction,
  FileList,
  StageList,
  StatusPill,
  type ConversationMessageItem,
  type OrchestrationFileItem,
  type OrchestrationStageItem,
} from '../../../ui';
import { capitalize, errorMessage, formatDateTime } from '../../../app/viewModels/formatting';

export interface OrchestrationsPageProps {
  orchestrationClient: OrchestrationClient;
}

type OrchestrationView = 'live' | 'plan' | 'history' | 'blockers';

const defaultOrchestrationFolder = 'C:\\Users\\user\\.codex\\orchestrations';
const internalIntakeDraftTitle = 'Internal plan-builder intake draft';
export function OrchestrationsPage({ orchestrationClient }: OrchestrationsPageProps) {
  const [orchestrations, setOrchestrations] = useState<OrchestrationSnapshot[]>([]);
  const [buildPackages, setBuildPackages] = useState<OrchestrationBuildPackage[]>([]);
  const [clientError, setClientError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedOrchestrationId, setSelectedOrchestrationId] = useState<EntityId | null>(null);
  const [selectedBuildId, setSelectedBuildId] = useState<EntityId | null>(null);
  const [mode, setMode] = useState<'overview' | 'add' | 'build' | 'detail'>('overview');
  const selectedOrchestration =
    orchestrations.find((orchestration) => orchestration.id === selectedOrchestrationId) ?? null;
  const selectedBuild = buildPackages.find((build) => build.id === selectedBuildId) ?? null;

  const openOverview = () => {
    setMode('overview');
    setSelectedOrchestrationId(null);
    setSelectedBuildId(null);
  };

  const applyRegistrySnapshot = useCallback(
    (registry: {
      orchestrations: OrchestrationSnapshot[];
      buildPackages: OrchestrationBuildPackage[];
    }) => {
      setOrchestrations(registry.orchestrations);
      setBuildPackages(registry.buildPackages);
    },
    [],
  );

  useEffect(() => {
    let cancelled = false;

    void (async () => {
      setIsLoading(true);
      setClientError(null);

      try {
        const registry = await orchestrationClient.loadOrchestrations();

        if (!cancelled) {
          applyRegistrySnapshot(registry);
        }
      } catch (caught) {
        if (!cancelled) {
          setClientError(errorMessage(caught));
        }
      } finally {
        if (!cancelled) {
          setIsLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [applyRegistrySnapshot, orchestrationClient]);

  if (mode === 'add') {
    return (
      <AddOrchestrationFlow
        orchestrationClient={orchestrationClient}
        onBack={openOverview}
        onCreated={(buildPackage) => {
          setBuildPackages((current) => [
            buildPackage,
            ...current.filter((build) => build.id !== buildPackage.id),
          ]);
          setSelectedBuildId(buildPackage.id);
        }}
      />
    );
  }

  if (mode === 'build' && selectedBuild) {
    return (
      <OrchestrationBuildWorkspace
        orchestrationClient={orchestrationClient}
        buildPackage={selectedBuild}
        onBackToOverview={openOverview}
        onBuildUpdate={(updatedBuild) => {
          setBuildPackages((current) =>
            current.map((build) => (build.id === updatedBuild.id ? updatedBuild : build)),
          );
        }}
        onStart={(orchestration) => {
          setOrchestrations((current) => [orchestration, ...current]);
          setBuildPackages((current) => current.filter((build) => build.id !== selectedBuild.id));
          setSelectedBuildId(null);
          setSelectedOrchestrationId(orchestration.id);
          setMode('detail');
        }}
        onBuildStillPending={(updatedBuild) => {
          setBuildPackages((current) =>
            current.map((build) => (build.id === updatedBuild.id ? updatedBuild : build)),
          );
        }}
        onCancel={() => {
          void (async () => {
            try {
              applyRegistrySnapshot(await orchestrationClient.cancelDraft(selectedBuild.id));
              openOverview();
            } catch (caught) {
              setClientError(errorMessage(caught));
            }
          })();
        }}
      />
    );
  }

  if (mode === 'detail' && selectedOrchestration) {
    return (
      <OrchestrationWorkspace
        orchestration={selectedOrchestration}
        onBackToOverview={openOverview}
      />
    );
  }

  return (
    <OrchestrationRegistryOverview
      orchestrations={orchestrations}
      buildPackages={buildPackages}
      error={clientError}
      loading={isLoading}
      onAdd={() => setMode('add')}
      onOpenBuild={(buildId) => {
        setSelectedBuildId(buildId);
        setMode('build');
      }}
      onOpen={(orchestrationId) => {
        setSelectedOrchestrationId(orchestrationId);
        setMode('detail');
      }}
    />
  );
}

interface OrchestrationRegistryOverviewProps {
  orchestrations: OrchestrationSnapshot[];
  buildPackages: OrchestrationBuildPackage[];
  error: string | null;
  loading: boolean;
  onAdd(): void;
  onOpenBuild(buildId: EntityId): void;
  onOpen(orchestrationId: EntityId): void;
}

function OrchestrationRegistryOverview({
  orchestrations,
  buildPackages,
  error,
  loading,
  onAdd,
  onOpenBuild,
  onOpen,
}: OrchestrationRegistryOverviewProps) {
  const activeCount = orchestrations.filter((orchestration) =>
    [
      'planning',
      'delegated',
      'delegating',
      'working',
      'reviewing',
      'merging',
      'reporting',
    ].includes(orchestration.state),
  ).length;

  return (
    <section className="workspace orchestration-workspace" id="orchestrations">
      <header className="topbar">
        <div>
          <p className="eyebrow">Product-owned control plane</p>
          <h1>Orchestrations</h1>
        </div>
        <div className="status-strip" aria-label="Orchestration registry status">
          <span>{orchestrations.length} registered</span>
          <span>{buildPackages.length} drafts</span>
          <span>{activeCount} active</span>
          <button className="primary-action" type="button" onClick={onAdd}>
            <Plus size={17} aria-hidden="true" />
            Add Orchestration
          </button>
        </div>
      </header>

      {loading && (
        <section className="notice" role="status">
          <LoaderCircle size={18} aria-hidden="true" />
          <span>Loading orchestration registry</span>
        </section>
      )}

      {error && (
        <section className="notice error" role="status">
          <AlertCircle size={18} aria-hidden="true" />
          <span>{error}</span>
        </section>
      )}

      <section className="orchestration-registry-board" aria-label="Registered orchestrations">
        <header>
          <div>
            <p className="eyebrow">Overview</p>
            <h2>Registered Orchestrations</h2>
          </div>
          <button className="primary-action" type="button" onClick={onAdd}>
            <Plus size={17} aria-hidden="true" />
            Add Orchestration
          </button>
        </header>

        {orchestrations.length === 0 && buildPackages.length === 0 ? (
          <div className="empty-orchestration-registry">
            <p>No orchestrations are registered.</p>
            <button className="primary-action" type="button" onClick={onAdd}>
              <Plus size={17} aria-hidden="true" />
              Add Orchestration
            </button>
          </div>
        ) : (
          <div className="orchestration-registry-grid">
            {buildPackages.map((buildPackage) => {
              const stage = currentBuildStage(buildPackage);

              return (
                <button
                  className="orchestration-registry-card build"
                  key={buildPackage.id}
                  type="button"
                  aria-label={`Open build package ${displayBuildPackageTitle(buildPackage)}`}
                  onClick={() => onOpenBuild(buildPackage.id)}
                >
                  <header>
                    <div>
                      <strong>{displayBuildPackageTitle(buildPackage)}</strong>
                      <small>{buildPackageTitleDetail(buildPackage)}</small>
                    </div>
                    <span className={statePillClass(stage.state)}>
                      {getOrchestrationStatusLabel(stage.state)}
                    </span>
                  </header>
                  <p>{getOrchestrationStatusDescription(stage.state)}</p>
                  <div className="metric-row">
                    <span>Plan Builder</span>
                    <span>No runtime thread</span>
                    <span>{buildPackage.files.length} uploads</span>
                  </div>
                </button>
              );
            })}
            {orchestrations.map((orchestration) => (
              <button
                className="orchestration-registry-card"
                key={orchestration.id}
                type="button"
                onClick={() => onOpen(orchestration.id)}
              >
                <header>
                  <div>
                    <strong>{orchestration.title}</strong>
                    <small>{orchestration.anchor}</small>
                  </div>
                  <span className={`state-pill ${orchestration.state}`}>{orchestration.state}</span>
                </header>
                <p>{orchestration.currentPosition}</p>
                <div className="metric-row">
                  <span>{orchestration.planners.length} planners</span>
                  <span>{countWorkSlices(orchestration)} slices</span>
                  <span>{orchestration.blockers.length} blockers</span>
                </div>
              </button>
            ))}
          </div>
        )}
      </section>
    </section>
  );
}

interface AddOrchestrationFlowProps {
  orchestrationClient: OrchestrationClient;
  onBack(): void;
  onCreated(buildPackage: OrchestrationBuildPackage): void;
}

function AddOrchestrationFlow({
  orchestrationClient,
  onBack,
  onCreated,
}: AddOrchestrationFlowProps) {
  const conversationFileInputRef = useRef<HTMLInputElement | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [promptDraft, setPromptDraft] = useState('');
  const [uploadedFiles, setUploadedFiles] = useState<UploadedConversationFile[]>([]);
  const [submittedBuildPackage, setSubmittedBuildPackage] =
    useState<OrchestrationBuildPackage | null>(null);
  const [messages, setMessages] = useState<OrchestrationConversationMessage[]>([
    {
      id: 'plan-builder-ready',
      role: 'system',
      body: 'Plan Builder has not started. Add source material for a local intake draft; no Codex runtime thread exists yet.',
      createdAt: new Date().toISOString(),
      truth: localDraftTruthState,
    },
  ]);
  const trimmedPrompt = promptDraft.trim();
  const hasSourceMaterial = trimmedPrompt.length > 0 || uploadedFiles.length > 0;
  const addFlowState = getAddOrchestrationCurrentAction({
    sourceMaterialReady: hasSourceMaterial,
    promptStarted: promptDraft.trim().length > 0 || uploadedFiles.length > 0,
    submittedBuildPackage,
    submitting,
    submitError,
  });
  const planBuilderStageState = submittedBuildPackage
    ? currentBuildStage(submittedBuildPackage).state
    : localDraftTruthState;
  const submittedCurrentStage = submittedBuildPackage
    ? currentBuildStage(submittedBuildPackage)
    : null;
  const canApproveSubmittedPlan =
    submittedBuildPackage !== null &&
    submittedCurrentStage?.id === 'plan-review' &&
    hasPlanBuilderOutputEvidence(submittedBuildPackage) &&
    !hasInstantiatorOutputEvidence(submittedBuildPackage) &&
    !submitting;

  const updateSubmittedBuildPackage = (buildPackage: OrchestrationBuildPackage) => {
    setSubmittedBuildPackage(buildPackage);
    setMessages(buildPackage.messages);
    onCreated(buildPackage);
  };

  const submitPrompt = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!hasSourceMaterial) {
      return;
    }

    void (async () => {
      const submittedAt = new Date().toISOString();
      const submittedPrompt =
        trimmedPrompt || 'Source material attached as files; no pasted text was supplied.';
      setSubmitting(true);
      setSubmitError(null);
      setMessages((current) => [
        ...current,
        {
          id: `local-submit-${submittedAt}`,
          role: 'user',
          body: submittedPrompt,
          createdAt: submittedAt,
          state: 'completed',
          truth: { status: 'ready', provenance: 'user_input' },
        },
        {
          id: `local-submitting-${submittedAt}`,
          role: 'system',
          body: 'Saving source material as a local intake draft before requesting Plan Builder runtime start.',
          createdAt: submittedAt,
          state: 'processing',
          truth: { status: 'starting', provenance: 'local_draft' },
        },
      ]);

      try {
        const buildPackage = await orchestrationClient.createDraft({
          title: internalIntakeDraftTitle,
          folderPath: defaultOrchestrationFolder,
          prompt: submittedPrompt,
          files: uploadedFiles,
        });
        const savedDraft: OrchestrationBuildPackage = {
          ...buildPackage,
          clientState: {
            ...buildPackage.clientState,
            status: 'starting',
            provenance: 'local_draft',
            currentAction:
              'Draft saved. Sending Plan Builder runtime request; waiting for backend acknowledgement.',
            runtimeSupported: false,
          },
          messages: [
            ...buildPackage.messages,
            {
              id: `runtime-request-${submittedAt}`,
              role: 'system',
              body: 'Draft saved. Sending Plan Builder runtime request; waiting for backend acknowledgement.',
              createdAt: new Date().toISOString(),
              state: 'processing',
              truth: { status: 'starting', provenance: 'local_draft' },
            },
          ],
          stages: buildPackage.stages.map((stage) =>
            stage.id === 'plan-builder'
              ? {
                  ...stage,
                  state: { status: 'starting', provenance: 'local_draft' },
                  summary: 'Draft saved; sending Plan Builder runtime request.',
                  detail:
                    'The frontend has saved the draft and is awaiting backend acknowledgement for the runtime request.',
                }
              : stage,
          ),
        };
        updateSubmittedBuildPackage(savedDraft);

        try {
          const runtimeBuild = await orchestrationClient.startPlanBuilderRun({
            buildPackageId: buildPackage.id,
          });
          updateSubmittedBuildPackage(runtimeBuild);
          setPromptDraft('');
        } catch (startCaught) {
          const runtimeFailedBuild = buildRuntimeStartFailurePackage(
            savedDraft,
            errorMessage(startCaught),
          );
          updateSubmittedBuildPackage(runtimeFailedBuild);
          setPromptDraft('');
        }
      } catch (caught) {
        const message = errorMessage(caught);

        setSubmitError(message);
        setMessages((current) =>
          current.map((conversationMessage) =>
            conversationMessage.id === `local-submitting-${submittedAt}`
              ? {
                  ...conversationMessage,
                  body: `Draft creation failed. ${message}`,
                  state: 'completed',
                  truth: { status: 'failed', provenance: 'unsupported' },
                }
              : conversationMessage,
          ),
        );
      } finally {
        setSubmitting(false);
      }
    })();
  };

  const addConversationFiles = (fileList: FileList | null) => {
    const nextFiles = uploadedFilesFromFileList(fileList);

    if (nextFiles.length === 0) {
      return;
    }

    setUploadedFiles((current) => mergeUploadedFiles(current, nextFiles));
  };

  const submitPlanFeedback = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmedFeedback = promptDraft.trim();

    if (!submittedBuildPackage || !trimmedFeedback) {
      return;
    }

    void (async () => {
      setSubmitting(true);
      setSubmitError(null);

      try {
        updateSubmittedBuildPackage(
          await orchestrationClient.addDraftNote({
            buildPackageId: submittedBuildPackage.id,
            body: trimmedFeedback,
          }),
        );
        setPromptDraft('');
      } catch (caught) {
        setSubmitError(errorMessage(caught));
      } finally {
        setSubmitting(false);
      }
    })();
  };

  const approveSubmittedPlan = () => {
    if (!submittedBuildPackage || !canApproveSubmittedPlan) {
      return;
    }

    void (async () => {
      setSubmitting(true);
      setSubmitError(null);

      try {
        updateSubmittedBuildPackage(
          await orchestrationClient.requestBuildStage({
            buildPackageId: submittedBuildPackage.id,
            stageId: 'instantiator',
          }),
        );
      } catch (caught) {
        setSubmitError(errorMessage(caught));
      } finally {
        setSubmitting(false);
      }
    })();
  };

  return (
    <section className="workspace orchestration-workspace" id="add-orchestration">
      <header className="topbar">
        <div>
          <p className="eyebrow">New orchestration</p>
          <h1>Add Orchestration</h1>
        </div>
        <button className="text-button" type="button" onClick={onBack}>
          Back to Overview
        </button>
      </header>

      <section className="add-orchestration-layout">
        <aside className="add-orchestration-settings" aria-label="Plan Builder intake state">
          <header>
            <p className="eyebrow">Stage</p>
            <h2>Plan Builder</h2>
          </header>
          <StageList
            ariaLabel="Plan Builder intake stage"
            stages={[
              {
                description: submittedBuildPackage
                  ? 'Source material was saved as an intake draft; runtime start is unsupported in this path.'
                  : 'Not started. Source material is still local to this intake screen.',
                evidenceLabel: submittedBuildPackage
                  ? 'Unsupported integration'
                  : 'Local draft session',
                id: 'plan-builder-intake',
                isCurrent: true,
                state: planBuilderStageState,
                title: 'Plan Builder',
              },
            ]}
          />
          <p className="intake-internal-note">
            The storage layer receives an internal draft title and default folder only after submit.
          </p>
        </aside>

        <section className="add-orchestration-conversation" aria-label="Plan builder conversation">
          <header>
            <div>
              <p className="eyebrow">Plan Builder</p>
              <h2>Intake</h2>
            </div>
            <StatusPill state={addFlowState.state} />
          </header>

          <CurrentAction
            actionLabel={
              canApproveSubmittedPlan ? 'Confirm build plan and start instantiating' : undefined
            }
            aria-label="Add orchestration current action"
            busy={submitting}
            description={addFlowState.description}
            onAction={canApproveSubmittedPlan ? approveSubmittedPlan : undefined}
            state={addFlowState.state}
            title={addFlowState.title}
          />

          {submittedBuildPackage ? (
            <>
              <AgentConversationView
                aria-label="Plan Builder intake draft conversation"
                conversation={toIntakeAgentConversation(submittedBuildPackage)}
              />

              <form className="agent-prompt-form" onSubmit={submitPlanFeedback}>
                <label htmlFor="plan-builder-feedback">Plan Builder feedback</label>
                <div className="agent-prompt-row build-prompt-row">
                  <textarea
                    id="plan-builder-feedback"
                    value={promptDraft}
                    onChange={(event) => setPromptDraft(event.target.value)}
                    placeholder="Preserve feedback locally. Runtime continuation is unsupported unless the backend reports a continuation route."
                  />
                  <button
                    className="primary-action"
                    type="submit"
                    aria-label="Preserve Plan Builder feedback"
                    disabled={submitting || !promptDraft.trim()}
                  >
                    <Edit3 size={17} aria-hidden="true" />
                  </button>
                </div>
                {submitError && (
                  <p className="form-error" role="status">
                    {submitError}
                  </p>
                )}
              </form>
            </>
          ) : (
            <>
              <ConversationThreadView
                messages={messages}
                files={uploadedFiles}
                onFilesAdded={setUploadedFiles}
              />

              <form className="agent-prompt-form" onSubmit={submitPrompt}>
                <label htmlFor="plan-builder-prompt">Source material</label>
                <div className="agent-prompt-row build-prompt-row">
                  <textarea
                    id="plan-builder-prompt"
                    value={promptDraft}
                    onChange={(event) => setPromptDraft(event.target.value)}
                    placeholder="Paste the source handoff, rough objective, constraints, and relevant context."
                  />
                  <button
                    className="icon-button"
                    type="button"
                    onClick={() => conversationFileInputRef.current?.click()}
                    title="Attach files"
                    aria-label="Attach files"
                  >
                    <Upload size={16} aria-hidden="true" />
                  </button>
                  <button
                    className="primary-action"
                    type="submit"
                    aria-label="Start Plan Builder"
                    disabled={submitting || !hasSourceMaterial}
                  >
                    <Play size={17} aria-hidden="true" />
                    <span>{submitting ? 'Starting' : 'Start Plan Builder'}</span>
                  </button>
                  <input
                    ref={conversationFileInputRef}
                    className="visually-hidden"
                    type="file"
                    multiple
                    aria-label="Choose conversation files"
                    onChange={(event) => addConversationFiles(event.target.files)}
                  />
                </div>
                {submitError && (
                  <p className="form-error" role="status">
                    {submitError}
                  </p>
                )}
              </form>
            </>
          )}
        </section>
      </section>
    </section>
  );
}

interface OrchestrationBuildWorkspaceProps {
  orchestrationClient: OrchestrationClient;
  buildPackage: OrchestrationBuildPackage;
  onBackToOverview(): void;
  onBuildUpdate(buildPackage: OrchestrationBuildPackage): void;
  onStart(orchestration: OrchestrationSnapshot): void;
  onBuildStillPending(buildPackage: OrchestrationBuildPackage): void;
  onCancel(): void;
}

interface AddOrchestrationCurrentActionInput {
  sourceMaterialReady: boolean;
  promptStarted: boolean;
  submittedBuildPackage: OrchestrationBuildPackage | null;
  submitting: boolean;
  submitError: string | null;
}

interface AddOrchestrationCurrentAction {
  description: string;
  state: OrchestrationTruthState;
  title: string;
}

function getAddOrchestrationCurrentAction({
  sourceMaterialReady,
  promptStarted,
  submittedBuildPackage,
  submitting,
  submitError,
}: AddOrchestrationCurrentActionInput): AddOrchestrationCurrentAction {
  if (submitError) {
    return {
      description: `The draft was not created. ${submitError}`,
      state: { status: 'failed', provenance: 'unsupported' },
      title: 'Draft creation failed',
    };
  }

  if (submittedBuildPackage) {
    const submittedState = {
      status: submittedBuildPackage.clientState.status,
      provenance: submittedBuildPackage.clientState.provenance,
    };
    const localRuntimeRequestInFlight =
      submittedState.status === 'starting' && submittedState.provenance === 'local_draft';
    const runtimeEvidenceAvailable =
      submittedBuildPackage.clientState.runtimeSupported ||
      submittedState.provenance === 'runtime_event' ||
      submittedState.provenance === 'backend_response';

    if (localRuntimeRequestInFlight) {
      return {
        description: submittedBuildPackage.clientState.currentAction,
        state: submittedState,
        title: 'Sending runtime request',
      };
    }

    return {
      description: runtimeEvidenceAvailable
        ? submittedBuildPackage.clientState.currentAction
        : 'Source material was saved as an intake draft. Plan Builder runtime start is unsupported, so no Codex thread was created.',
      state:
        submittedBuildPackage.clientState.status === 'integration_pending'
          ? integrationPendingTruthState
          : submittedState,
      title: runtimeEvidenceAvailable
        ? getOrchestrationStatusLabel(submittedState)
        : 'Runtime unsupported',
    };
  }

  if (submitting) {
    return {
      description:
        'Saving source material as an intake draft before requesting Plan Builder runtime start.',
      state: { status: 'starting', provenance: 'local_draft' },
      title: 'Saving intake draft',
    };
  }

  if (sourceMaterialReady) {
    return {
      description: 'Source material is ready to save as a local intake draft.',
      state: { status: 'ready', provenance: 'user_input' },
      title: 'Ready to save intake',
    };
  }

  if (promptStarted) {
    return {
      description: 'Source material is being prepared locally; Plan Builder has not started.',
      state: localDraftTruthState,
      title: 'Intake in progress',
    };
  }

  return {
    description: 'Paste source material or attach files. Plan Builder has not started.',
    state: localDraftTruthState,
    title: 'Waiting for source material',
  };
}

function buildRuntimeStartFailurePackage(
  buildPackage: OrchestrationBuildPackage,
  message: string,
): OrchestrationBuildPackage {
  const failedState = { status: 'failed', provenance: 'backend_response' } as const;
  const failedAt = new Date().toISOString();
  const notice = {
    id: 'plan-builder-runtime-start-failed',
    kind: 'error' as const,
    title: 'Plan Builder runtime start failed',
    message: `The draft was saved, but the Plan Builder runtime command failed before returning stage evidence. ${message}`,
    truth: failedState,
  };

  return {
    ...buildPackage,
    updatedAt: failedAt,
    clientState: {
      ...buildPackage.clientState,
      status: failedState.status,
      provenance: failedState.provenance,
      currentAction: notice.message,
      updatedAt: failedAt,
      runtimeSupported: false,
      notices: [notice],
      primaryAction: undefined,
    },
    messages: [
      ...buildPackage.messages,
      {
        id: `runtime-start-failed-${failedAt}`,
        role: 'system',
        body: notice.message,
        createdAt: failedAt,
        state: 'completed',
        truth: failedState,
      },
    ],
    stages: buildPackage.stages.map((stage) =>
      stage.id === 'plan-builder'
        ? {
            ...stage,
            state: failedState,
            summary: 'The draft was preserved, but Plan Builder runtime start failed.',
            detail: notice.message,
          }
        : stage,
    ),
  };
}

function OrchestrationBuildWorkspace({
  orchestrationClient,
  buildPackage,
  onBackToOverview,
  onBuildUpdate,
  onStart,
  onBuildStillPending,
  onCancel,
}: OrchestrationBuildWorkspaceProps) {
  const [promptDraft, setPromptDraft] = useState('');
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionBusy, setActionBusy] = useState(false);
  const [planBuilderView, setPlanBuilderView] = useState<'conversation' | 'plan-review'>(
    'conversation',
  );
  const buildFileInputRef = useRef<HTMLInputElement | null>(null);
  const currentStage = currentBuildStage(buildPackage);
  const hasPlanBuilderOutput = hasPlanBuilderOutputEvidence(buildPackage);
  const hasInstantiatorOutput = hasInstantiatorOutputEvidence(buildPackage);
  const completedStageCount = buildPackage.stages.filter(
    (stage) => stage.state.status === 'completed',
  ).length;
  const rootStartupReady =
    currentStage.id === 'root-startup' && currentStage.state.status === 'ready';
  const primaryAction = buildPackage.clientState.primaryAction;
  const canRequestBuildStage =
    primaryAction?.id === 'request-build-stage' &&
    primaryAction.enabled &&
    !isMockOrUnsupported(currentStage.state);
  const canStartOrchestration =
    rootStartupReady &&
    primaryAction?.id === 'start-orchestration' &&
    primaryAction.enabled &&
    !isMockOrUnsupported(currentStage.state);
  const canStartInstantiation =
    hasPlanBuilderOutput &&
    !hasInstantiatorOutput &&
    currentStage.id === 'plan-review' &&
    !actionBusy &&
    currentStage.state.status !== 'running' &&
    currentStage.state.status !== 'starting' &&
    currentStage.state.status !== 'waiting_for_event';

  const requestBuildStage = (stageId: OrchestrationBuildStageId) => {
    if (actionBusy) {
      return;
    }

    void (async () => {
      setActionBusy(true);
      setActionError(null);

      try {
        onBuildUpdate(
          await orchestrationClient.requestBuildStage({
            buildPackageId: buildPackage.id,
            stageId,
          }),
        );
      } catch (caught) {
        setActionError(errorMessage(caught));
      } finally {
        setActionBusy(false);
      }
    })();
  };

  const advanceBuild = () => {
    if (!canRequestBuildStage) {
      return;
    }

    requestBuildStage(currentStage.id);
  };

  const startInstantiation = () => {
    if (!canStartInstantiation) {
      return;
    }

    requestBuildStage('instantiator');
  };

  const submitBuildPrompt = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmedPrompt = promptDraft.trim();

    if (!trimmedPrompt) {
      return;
    }

    void (async () => {
      setActionBusy(true);
      setActionError(null);

      try {
        onBuildUpdate(
          await orchestrationClient.addDraftNote({
            buildPackageId: buildPackage.id,
            body: trimmedPrompt,
          }),
        );
        setPromptDraft('');
      } catch (caught) {
        setActionError(errorMessage(caught));
      } finally {
        setActionBusy(false);
      }
    })();
  };

  const addBuildFiles = (fileList: FileList | null) => {
    const nextFiles = uploadedFilesFromFileList(fileList);

    if (nextFiles.length === 0) {
      return;
    }

    void (async () => {
      setActionBusy(true);
      setActionError(null);

      try {
        onBuildUpdate(
          await orchestrationClient.attachDraftFiles({
            buildPackageId: buildPackage.id,
            files: nextFiles,
          }),
        );
      } catch (caught) {
        setActionError(errorMessage(caught));
      } finally {
        setActionBusy(false);
      }
    })();
  };

  const startOrchestration = () => {
    void (async () => {
      setActionBusy(true);
      setActionError(null);

      try {
        const result = await orchestrationClient.startOrchestration({
          buildPackageId: buildPackage.id,
        });

        if (result.orchestration) {
          onStart(result.orchestration);
          return;
        }

        if (result.buildPackage) {
          onBuildStillPending(result.buildPackage);
        }
      } catch (caught) {
        setActionError(errorMessage(caught));
      } finally {
        setActionBusy(false);
      }
    })();
  };

  const currentActionLabel = canStartOrchestration
    ? (primaryAction?.label ?? 'Start Orchestration')
    : canStartInstantiation
      ? 'Confirm build plan and start instantiating'
      : canRequestBuildStage
        ? primaryAction.label
        : undefined;
  const currentActionHandler = canStartOrchestration
    ? startOrchestration
    : canStartInstantiation
      ? startInstantiation
      : canRequestBuildStage
        ? advanceBuild
        : undefined;

  return (
    <section className="workspace orchestration-workspace" id="orchestration-build">
      <header className="topbar">
        <div>
          <p className="eyebrow">Build and initiation</p>
          <h1>{buildPackage.title}</h1>
        </div>
        <div className="status-strip" aria-label="Build initiation status">
          <button className="text-button" type="button" onClick={onBackToOverview}>
            Overview
          </button>
          <button
            className="text-button danger"
            type="button"
            onClick={onCancel}
            aria-label={`Cancel build ${buildPackage.title}`}
          >
            <X size={16} aria-hidden="true" />
            Cancel Build
          </button>
          <span>
            {completedStageCount} of {buildPackage.stages.length} stages complete
          </span>
          <span>{buildPackage.files.length} uploads</span>
        </div>
      </header>

      <section className="build-initiation-hero" aria-label="Build initiation current state">
        <CurrentAction
          actionLabel={currentActionLabel}
          aria-label="Build initiation current action"
          busy={actionBusy}
          description={`${buildPackage.clientState.currentAction} ${currentStage.detail}`}
          onAction={currentActionHandler}
          state={currentStage.state}
          title={currentStage.title}
        />
      </section>

      <section className="build-initiation-layout">
        <section className="build-stage-board" aria-label="Build initiation stages">
          <header>
            <div>
              <p className="eyebrow">Airlock</p>
              <h2>Build Package</h2>
            </div>
            <span>{buildPackage.folderPath}</span>
          </header>

          <StageList
            ariaLabel="Build initiation stages"
            stages={toStageItems(buildPackage, currentStage.id)}
          />

          {hasInstantiatorOutput && (
            <section className="generated-package-panel" aria-label="Expected package outputs">
              <header>
                <p className="eyebrow">Instantiator Evidence</p>
                <h3>Generated Package</h3>
              </header>
              <FileList
                aria-label="Expected package output slots"
                emptyLabel="No instantiator-backed outputs were reported."
                files={toExpectedOutputItems(buildPackage)}
              />
            </section>
          )}
        </section>

        <section className="plan-builder-workspace" aria-label="Plan Builder UI">
          <header>
            <div>
              <p className="eyebrow">Plan Builder</p>
              <h2>Build Conversation</h2>
            </div>
            <span className={statePillClass(currentStage.state)}>
              {getOrchestrationStatusLabel(currentStage.state)}
            </span>
          </header>

          <div className="view-tabs" role="tablist" aria-label="Plan Builder views">
            <button
              className={planBuilderView === 'conversation' ? 'active' : ''}
              type="button"
              onClick={() => setPlanBuilderView('conversation')}
            >
              Conversation
            </button>
            {hasInstantiatorOutput && (
              <button
                className={planBuilderView === 'plan-review' ? 'active' : ''}
                type="button"
                onClick={() => setPlanBuilderView('plan-review')}
              >
                Expected Shape
              </button>
            )}
          </div>

          {planBuilderView === 'conversation' ? (
            <section className="build-conversation-panel" aria-label="Build conversation">
              <header>
                <div>
                  <p className="eyebrow">Conversation</p>
                  <h3>Turns</h3>
                </div>
                <CurrentTurnIndicator messages={buildPackage.messages} />
              </header>

              <ConversationThreadView
                messages={buildPackage.messages}
                files={buildPackage.files}
                onFilesAdded={(updater) => {
                  const nextFiles = updater(buildPackage.files);

                  void (async () => {
                    setActionBusy(true);
                    setActionError(null);

                    try {
                      onBuildUpdate(
                        await orchestrationClient.attachDraftFiles({
                          buildPackageId: buildPackage.id,
                          files: nextFiles,
                        }),
                      );
                    } catch (caught) {
                      setActionError(errorMessage(caught));
                    } finally {
                      setActionBusy(false);
                    }
                  })();
                }}
              />

              <CurrentTurnSummary messages={buildPackage.messages} />

              <form className="agent-prompt-form" onSubmit={submitBuildPrompt}>
                <label htmlFor="build-prompt">Plan Builder feedback</label>
                <div className="agent-prompt-row build-prompt-row">
                  <textarea
                    id="build-prompt"
                    value={promptDraft}
                    onChange={(event) => setPromptDraft(event.target.value)}
                    placeholder="Preserve feedback locally. Runtime continuation is unsupported unless the backend reports a continuation route."
                  />
                  <button
                    className="icon-button"
                    type="button"
                    onClick={() => buildFileInputRef.current?.click()}
                    title="Attach files"
                    aria-label="Attach files"
                  >
                    <Upload size={16} aria-hidden="true" />
                  </button>
                  <button
                    className="primary-action"
                    type="submit"
                    aria-label="Preserve Plan Builder feedback"
                    disabled={actionBusy || !promptDraft.trim()}
                  >
                    <Edit3 size={17} aria-hidden="true" />
                  </button>
                  <input
                    ref={buildFileInputRef}
                    className="visually-hidden"
                    type="file"
                    multiple
                    aria-label="Choose conversation files"
                    onChange={(event) => addBuildFiles(event.target.files)}
                  />
                </div>
              </form>
            </section>
          ) : (
            <section className="plan-preview-panel" aria-label="Expected local plan shape">
              <header>
                <p className="eyebrow">Instantiator Evidence</p>
                <h3>Expected Shape</h3>
              </header>
              <p>
                Shown from instantiator stage-run evidence. Missing files are not shown as pending.
              </p>
              <FileList
                aria-label="Instantiator-backed expected shape"
                emptyLabel="No instantiator-backed outputs were reported."
                files={toExpectedOutputItems(buildPackage)}
              />
            </section>
          )}
        </section>
      </section>
      {actionError && (
        <section className="notice error" role="status">
          <AlertCircle size={18} aria-hidden="true" />
          <span>{actionError}</span>
        </section>
      )}
    </section>
  );
}

interface ConversationThreadViewProps {
  messages: OrchestrationConversationMessage[];
  files: UploadedConversationFile[];
  onFilesAdded(updater: (current: UploadedConversationFile[]) => UploadedConversationFile[]): void;
}

function ConversationThreadView({ messages, files, onFilesAdded }: ConversationThreadViewProps) {
  return (
    <ConversationFileDropZone files={files} onFilesAdded={onFilesAdded}>
      <ConversationThread
        aria-label="Conversation messages"
        messages={toConversationItems(messages)}
      />
    </ConversationFileDropZone>
  );
}

interface CurrentTurnIndicatorProps {
  messages: OrchestrationConversationMessage[];
}

function CurrentTurnIndicator({ messages }: CurrentTurnIndicatorProps) {
  const currentTurn = currentConversationTurn(messages);

  if (!currentTurn) {
    return <span className="state-pill idle">idle</span>;
  }

  const stateClass =
    currentTurn.state === 'processing'
      ? 'state-pill planning'
      : currentTurn.truth
        ? statePillClass(currentTurn.truth)
        : 'state-pill idle';

  return (
    <span className={stateClass}>
      {currentTurn.state === 'processing'
        ? `${currentTurn.role} processing`
        : currentTurn.truth
          ? getOrchestrationStatusLabel(currentTurn.truth)
          : 'idle'}
    </span>
  );
}

function CurrentTurnSummary({ messages }: CurrentTurnIndicatorProps) {
  const currentTurn = currentConversationTurn(messages);

  if (!currentTurn || currentTurn.state !== 'processing') {
    return null;
  }

  return (
    <section className="current-processing-turn" aria-label="Current processing turn">
      <span>Currently processing</span>
      <p>{currentTurn.body}</p>
    </section>
  );
}

function currentConversationTurn(
  messages: OrchestrationConversationMessage[],
): OrchestrationConversationMessage | undefined {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (messages[index].state === 'processing') {
      return messages[index];
    }
  }

  return messages[messages.length - 1];
}

function turnLabel(message: OrchestrationConversationMessage, index: number): string {
  if (message.role === 'system') {
    return 'System';
  }

  return `Turn ${index + 1} - ${capitalize(message.role)}`;
}

interface ConversationFileDropZoneProps {
  files: UploadedConversationFile[];
  onFilesAdded(updater: (current: UploadedConversationFile[]) => UploadedConversationFile[]): void;
  children?: ReactNode;
}

function ConversationFileDropZone({
  files,
  onFilesAdded,
  children,
}: ConversationFileDropZoneProps) {
  const addFiles = (fileList: FileList | null) => {
    const nextFiles = uploadedFilesFromFileList(fileList);

    onFilesAdded((current) => mergeUploadedFiles(current, nextFiles));
  };

  const handleDrop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    addFiles(event.dataTransfer.files);
  };

  return (
    <div
      className="conversation-file-dropzone"
      onDragOver={(event) => event.preventDefault()}
      onDrop={handleDrop}
      aria-label="Conversation file uploads"
    >
      {children}
      {files.length > 0 && (
        <FileList aria-label="Uploaded files" files={toUploadedFileItems(files)} />
      )}
    </div>
  );
}

interface OrchestrationWorkspaceProps {
  orchestration: OrchestrationSnapshot;
  onBackToOverview?(): void;
}

function OrchestrationWorkspace({ orchestration, onBackToOverview }: OrchestrationWorkspaceProps) {
  const [view, setView] = useState<OrchestrationView>('live');
  const [selectedPlanNodeId, setSelectedPlanNodeId] = useState<EntityId>(
    findActivePlanNode(orchestration.plan)?.id ?? orchestration.plan.id,
  );
  const selectedPlanNode =
    findPlanNode(orchestration.plan, selectedPlanNodeId) ?? orchestration.plan;
  const [selectedBlockerId, setSelectedBlockerId] = useState<EntityId>(
    orchestration.blockers[0]?.id ?? '',
  );
  const [blockerConclusions, setBlockerConclusions] = useState<Record<EntityId, BlockerConclusion>>(
    {},
  );
  const selectedBlocker = orchestration.blockers.find(
    (blocker) => blocker.id === selectedBlockerId,
  );
  const [selectedPlannerId, setSelectedPlannerId] = useState<EntityId>(
    orchestration.planners[0]?.id ?? '',
  );
  const selectedPlanner =
    orchestration.planners.find((planner) => planner.id === selectedPlannerId) ??
    orchestration.planners[0];
  const [selectedSliceId, setSelectedSliceId] = useState<EntityId>(
    selectedPlanner?.workSlices[0]?.id ?? '',
  );
  const selectedSlice =
    selectedPlanner?.workSlices.find((slice) => slice.id === selectedSliceId) ??
    selectedPlanner?.workSlices[0];
  const [selectedStepId, setSelectedStepId] = useState<EntityId>(selectedSlice?.steps[0]?.id ?? '');
  const selectedStep =
    selectedSlice?.steps.find((step) => step.id === selectedStepId) ?? selectedSlice?.steps[0];
  const openBlockerReview = (blockerId: EntityId) => {
    setSelectedBlockerId(blockerId);
    setView('blockers');
  };
  const selectPlanner = (plannerId: EntityId) => {
    const planner = orchestration.planners.find((candidate) => candidate.id === plannerId);
    const nextSlice = planner?.workSlices[0];

    setSelectedPlannerId(plannerId);
    setSelectedSliceId(nextSlice?.id ?? '');
    setSelectedStepId(nextSlice?.steps[0]?.id ?? '');
  };

  useEffect(() => {
    const nextSlice = selectedPlanner?.workSlices[0];
    setSelectedSliceId(nextSlice?.id ?? '');
    setSelectedStepId(nextSlice?.steps[0]?.id ?? '');
  }, [selectedPlanner?.id, selectedPlanner?.workSlices]);

  return (
    <section className="workspace orchestration-workspace" id="orchestrations">
      <header className="topbar">
        <div>
          <p className="eyebrow">{orchestration.anchor}</p>
          <h1>{orchestration.title}</h1>
        </div>
        <div className="status-strip" aria-label="Orchestration status">
          {onBackToOverview && (
            <button className="text-button" type="button" onClick={onBackToOverview}>
              Overview
            </button>
          )}
          <span>{orchestration.state}</span>
          <span>{orchestration.planners.length} planners</span>
          <span>{countWorkSlices(orchestration)} work slices</span>
        </div>
      </header>

      <section className="orchestration-hero" aria-label="Orchestration objective">
        <div>
          <p className="eyebrow">Objective</p>
          <h2>{orchestration.objective}</h2>
        </div>
        <p>{orchestration.currentPosition}</p>
      </section>

      <div className="view-tabs" role="tablist" aria-label="Orchestration views">
        <button
          className={view === 'live' ? 'active' : ''}
          type="button"
          onClick={() => setView('live')}
        >
          Live State
        </button>
        <button
          className={view === 'plan' ? 'active' : ''}
          type="button"
          onClick={() => setView('plan')}
        >
          Plan Map
        </button>
        <button
          className={view === 'history' ? 'active' : ''}
          type="button"
          onClick={() => setView('history')}
        >
          History
        </button>
        <button
          className={view === 'blockers' ? 'active' : ''}
          type="button"
          onClick={() => setView('blockers')}
        >
          Blockers
        </button>
      </div>

      {view === 'live' ? (
        <OrchestrationLiveStateV2
          orchestration={orchestration}
          selectedPlanner={selectedPlanner}
          selectedSlice={selectedSlice}
          blockerConclusions={blockerConclusions}
          onPlannerSelect={selectPlanner}
          onSliceSelect={setSelectedSliceId}
          onBlockerReview={openBlockerReview}
        />
      ) : view === 'plan' ? (
        <OrchestrationPlanMapV2
          orchestration={orchestration}
          selectedPlanNode={selectedPlanNode}
          blockerConclusions={blockerConclusions}
          onPlanNodeSelect={setSelectedPlanNodeId}
          onBlockerReview={openBlockerReview}
        />
      ) : view === 'history' ? (
        <OrchestrationHistory
          orchestration={orchestration}
          selectedPlanner={selectedPlanner}
          selectedSlice={selectedSlice}
          selectedStep={selectedStep}
          onPlannerSelect={setSelectedPlannerId}
          onSliceSelect={(sliceId) => {
            const slice = selectedPlanner?.workSlices.find((candidate) => candidate.id === sliceId);
            setSelectedSliceId(sliceId);
            setSelectedStepId(slice?.steps[0]?.id ?? '');
          }}
          onStepSelect={setSelectedStepId}
        />
      ) : (
        <OrchestrationBlockersView
          orchestration={orchestration}
          selectedBlocker={selectedBlocker}
          blockerConclusions={blockerConclusions}
          onBlockerSelect={setSelectedBlockerId}
          onBlockerConclusion={(blockerId, conclusion) =>
            setBlockerConclusions((current) => ({ ...current, [blockerId]: conclusion }))
          }
        />
      )}
    </section>
  );
}

interface OrchestrationLiveStateV2Props {
  orchestration: OrchestrationSnapshot;
  selectedPlanner?: OrchestrationPlannerTurn;
  selectedSlice?: OrchestrationWorkSlice;
  blockerConclusions: Record<EntityId, BlockerConclusion>;
  onPlannerSelect(plannerId: EntityId): void;
  onSliceSelect(sliceId: EntityId): void;
  onBlockerReview(blockerId: EntityId): void;
}

function OrchestrationLiveStateV2({
  orchestration,
  selectedPlanner,
  selectedSlice,
  blockerConclusions,
  onPlannerSelect,
  onSliceSelect,
  onBlockerReview,
}: OrchestrationLiveStateV2Props) {
  const [promptDrafts, setPromptDrafts] = useState<Record<EntityId, string>>({});
  const [queuedPrompts, setQueuedPrompts] = useState<Record<EntityId, string[]>>({});
  const latestRootTurn = orchestration.rootTurns[0];
  const livePlanners = orchestration.planners.filter((planner) => isLivePlanner(planner));
  const visiblePlanners =
    livePlanners.length > 0 ? livePlanners : orchestration.planners.slice(0, 1);
  const selectedPlannerBlockers = selectedPlanner
    ? getBlockersByIds(orchestration, selectedPlanner.blockerIds)
    : [];
  const selectedSliceBlockers = selectedSlice
    ? getBlockersByIds(orchestration, selectedSlice.blockerIds)
    : [];

  const queuePrompt = (threadId: EntityId) => {
    const prompt = promptDrafts[threadId]?.trim();

    if (!prompt) {
      return;
    }

    setQueuedPrompts((current) => ({
      ...current,
      [threadId]: [...(current[threadId] ?? []), prompt],
    }));
    setPromptDrafts((current) => ({ ...current, [threadId]: '' }));
  };

  return (
    <div className="orchestration-live-state redesigned-live-state">
      <section className="current-step-board" aria-label="Current orchestration steps">
        <header>
          <div>
            <p className="eyebrow">Current Work</p>
            <h2>Orchestration Steps</h2>
          </div>
          <div className="metric-row">
            <span>{visiblePlanners.length} active planner turns</span>
            <span>
              {visiblePlanners.reduce((total, planner) => total + planner.workSlices.length, 0)}{' '}
              slices
            </span>
          </div>
        </header>

        {latestRootTurn && (
          <article className="root-turn-card" aria-label="Latest orchestration root turn">
            <header>
              <div>
                <p className="eyebrow">{formatDateTime(latestRootTurn.lastUpdatedAt)}</p>
                <h3>{latestRootTurn.title}</h3>
              </div>
              <span className={`state-pill ${latestRootTurn.state}`}>{latestRootTurn.state}</span>
            </header>
            <p>{latestRootTurn.currentAction}</p>
            <div className="agent-output-window">
              <span>Last Root Output</span>
              <p>{latestRootTurn.lastOutput}</p>
            </div>
            <RelevantBlockerNotice
              blockers={getBlockersByIds(orchestration, latestRootTurn.blockerIds)}
              blockerConclusions={blockerConclusions}
              onBlockerReview={onBlockerReview}
            />
          </article>
        )}

        <div className="live-planner-strip" aria-label="Planner turns with ongoing work">
          {visiblePlanners.map((planner) => (
            <button
              className={`planner-turn-card${planner.id === selectedPlanner?.id ? ' selected' : ''}`}
              key={planner.id}
              type="button"
              onClick={() => onPlannerSelect(planner.id)}
            >
              <header>
                <div>
                  <strong>{planner.title}</strong>
                  <small>{formatDateTime(planner.startedAt)}</small>
                </div>
                <span className={`state-pill ${planner.state}`}>{planner.state}</span>
              </header>
              <p>{getPlannerStatusSummary(planner)}</p>
              <RelevantBlockerNotice
                blockers={getBlockersByIds(orchestration, planner.blockerIds)}
                blockerConclusions={blockerConclusions}
                onBlockerReview={onBlockerReview}
                compact
              />
            </button>
          ))}
        </div>
      </section>

      {selectedPlanner && (
        <section className="planner-live-detail" aria-label="Selected planner live detail">
          <header>
            <div>
              <p className="eyebrow">Planner Detail</p>
              <h2>{selectedPlanner.title}</h2>
            </div>
            <span className={`state-pill ${selectedPlanner.state}`}>{selectedPlanner.state}</span>
          </header>
          <p>{selectedPlanner.reasoningSummary}</p>
          <RelevantBlockerNotice
            blockers={selectedPlannerBlockers}
            blockerConclusions={blockerConclusions}
            onBlockerReview={onBlockerReview}
          />

          <section className="explicit-plan-panel" aria-label="Planner explicit plan">
            <h3>Explicit Plan</h3>
            <ol>
              {selectedPlanner.explicitPlan.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ol>
          </section>

          <section className="planner-slice-overview" aria-label="Planner work slices">
            <header>
              <h3>Work Slices</h3>
              <span>{selectedPlanner.workSlices.length} total</span>
            </header>
            <div className="work-slice-window-grid">
              {selectedPlanner.workSlices.map((slice) => (
                <button
                  className={`work-slice-window${slice.id === selectedSlice?.id ? ' selected' : ''}`}
                  key={slice.id}
                  type="button"
                  onClick={() => onSliceSelect(slice.id)}
                >
                  <header>
                    <div>
                      <strong>{slice.title}</strong>
                      <small>{slice.repo}</small>
                    </div>
                    <span className={`state-pill ${slice.state}`}>{slice.state}</span>
                  </header>
                  <p className="slice-stage">{slice.lifecycleStage}</p>
                  <p>{slice.currentTurn}</p>
                  <RelevantBlockerNotice
                    blockers={getBlockersByIds(orchestration, slice.blockerIds)}
                    blockerConclusions={blockerConclusions}
                    onBlockerReview={onBlockerReview}
                    compact
                  />
                </button>
              ))}
            </div>
          </section>
        </section>
      )}

      {selectedSlice && (
        <section className="work-slice-live-detail" aria-label="Work slice conversation detail">
          <header>
            <div>
              <p className="eyebrow">{selectedSlice.repo}</p>
              <h2>{selectedSlice.title}</h2>
            </div>
            <span className={`state-pill ${selectedSlice.state}`}>
              {selectedSlice.lifecycleStage}
            </span>
          </header>
          <p>{selectedSlice.summary}</p>
          <RelevantBlockerNotice
            blockers={selectedSliceBlockers}
            blockerConclusions={blockerConclusions}
            onBlockerReview={onBlockerReview}
          />
          <div className="thread-pair-grid">
            <ThreadDetailPanel
              thread={selectedSlice.delegationThread}
              label="Delegation Thread"
              promptDraft={promptDrafts[selectedSlice.delegationThread.id] ?? ''}
              queuedPrompts={queuedPrompts[selectedSlice.delegationThread.id] ?? []}
              onPromptChange={(value) =>
                setPromptDrafts((current) => ({
                  ...current,
                  [selectedSlice.delegationThread.id]: value,
                }))
              }
              onQueuePrompt={() => queuePrompt(selectedSlice.delegationThread.id)}
            />
            <ThreadDetailPanel
              thread={selectedSlice.workerThread}
              label="Worker Thread"
              promptDraft={promptDrafts[selectedSlice.workerThread.id] ?? ''}
              queuedPrompts={queuedPrompts[selectedSlice.workerThread.id] ?? []}
              onPromptChange={(value) =>
                setPromptDrafts((current) => ({
                  ...current,
                  [selectedSlice.workerThread.id]: value,
                }))
              }
              onQueuePrompt={() => queuePrompt(selectedSlice.workerThread.id)}
            />
          </div>
        </section>
      )}
    </div>
  );
}

interface ThreadDetailPanelProps {
  thread: OrchestrationAgentWindow;
  label: string;
  promptDraft: string;
  queuedPrompts: string[];
  onPromptChange(value: string): void;
  onQueuePrompt(): void;
}

function ThreadDetailPanel({
  thread,
  label,
  promptDraft,
  queuedPrompts,
  onPromptChange,
  onQueuePrompt,
}: ThreadDetailPanelProps) {
  const [uploadedFiles, setUploadedFiles] = useState<UploadedConversationFile[]>([]);

  return (
    <article className="thread-detail-panel" aria-label={label}>
      <header>
        <div>
          <p className="eyebrow">{thread.threadId}</p>
          <h3>{label}</h3>
        </div>
        <span className={`state-pill ${thread.state}`}>{thread.state}</span>
      </header>
      <p>{thread.currentAction}</p>
      <div className="conversation-window">
        <article>
          <span>{thread.role}</span>
          <p>{thread.lastOutput}</p>
          <time dateTime={thread.lastUpdatedAt}>{formatDateTime(thread.lastUpdatedAt)}</time>
        </article>
        {queuedPrompts.map((prompt, index) => (
          <article className="queued" key={`${thread.id}-${index}`}>
            <span>queued prompt</span>
            <p>{prompt}</p>
          </article>
        ))}
      </div>
      <ConversationFileDropZone files={uploadedFiles} onFilesAdded={setUploadedFiles} />
      <form
        className="agent-prompt-form"
        onSubmit={(event) => {
          event.preventDefault();
          onQueuePrompt();
        }}
      >
        <label htmlFor={`thread-prompt-${thread.id}`}>Prompt {label}</label>
        <div className="agent-prompt-row">
          <textarea
            id={`thread-prompt-${thread.id}`}
            value={promptDraft}
            onChange={(event) => onPromptChange(event.target.value)}
            placeholder="Write a prompt for this conversation"
          />
          <button className="primary-action" type="submit" aria-label={`Queue prompt for ${label}`}>
            <Play size={17} aria-hidden="true" />
          </button>
        </div>
      </form>
    </article>
  );
}

interface RelevantBlockerNoticeProps {
  blockers: OrchestrationBlocker[];
  blockerConclusions: Record<EntityId, BlockerConclusion>;
  onBlockerReview(blockerId: EntityId): void;
  compact?: boolean;
}

function RelevantBlockerNotice({
  blockers,
  blockerConclusions,
  onBlockerReview,
  compact = false,
}: RelevantBlockerNoticeProps) {
  if (blockers.length === 0) {
    return null;
  }

  return (
    <div className={`relevant-blockers${compact ? ' compact' : ''}`} aria-label="Relevant blockers">
      {blockers.map((blocker) => {
        const state = getBlockerDisplayState(blocker, blockerConclusions);

        return (
          <button
            className={`blocker-link ${state}`}
            key={blocker.id}
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              onBlockerReview(blocker.id);
            }}
          >
            <AlertCircle size={15} aria-hidden="true" />
            <span>{blocker.title}</span>
            <small>
              {blocker.kind} / {state}
            </small>
          </button>
        );
      })}
    </div>
  );
}

interface OrchestrationPlanMapV2Props {
  orchestration: OrchestrationSnapshot;
  selectedPlanNode: OrchestrationPlanNode;
  blockerConclusions: Record<EntityId, BlockerConclusion>;
  onPlanNodeSelect(planNodeId: EntityId): void;
  onBlockerReview(blockerId: EntityId): void;
}

function OrchestrationPlanMapV2({
  orchestration,
  selectedPlanNode,
  blockerConclusions,
  onPlanNodeSelect,
  onBlockerReview,
}: OrchestrationPlanMapV2Props) {
  const planNodes = collectPlanNodes(orchestration.plan);
  const completedCount = planNodes.filter((node) =>
    ['completed', 'recorded', 'merged'].includes(node.state),
  ).length;
  const blockedCount = planNodes.filter((node) => node.state === 'blocked').length;

  return (
    <div className="orchestration-plan-map">
      <section className="plan-map-board" aria-label="Proposed orchestration stages">
        <header>
          <div>
            <p className="eyebrow">Proposed Stages</p>
            <h2>{orchestration.plan.title}</h2>
          </div>
          <div className="metric-row">
            <span>{completedCount} settled</span>
            <span>{blockedCount} blocked</span>
            <span>{planNodes.length} total nodes</span>
          </div>
        </header>
        <div className="plan-tree">
          {orchestration.plan.children.map((node) => (
            <CollapsiblePlanTreeNode
              key={node.id}
              node={node}
              orchestration={orchestration}
              selectedPlanNodeId={selectedPlanNode.id}
              blockerConclusions={blockerConclusions}
              onPlanNodeSelect={onPlanNodeSelect}
              onBlockerReview={onBlockerReview}
            />
          ))}
        </div>
      </section>

      <aside className="plan-node-inspector" aria-label="Selected plan stage">
        <header>
          <p className="eyebrow">Selected Stage</p>
          <h2>{selectedPlanNode.title}</h2>
          <span className={`state-pill ${selectedPlanNode.state}`}>{selectedPlanNode.state}</span>
        </header>
        <p>{selectedPlanNode.summary}</p>
        <DetailBlock title="Current Status" value={selectedPlanNode.statusDetail} />
        <RelevantBlockerNotice
          blockers={getBlockersForNode(orchestration, selectedPlanNode)}
          blockerConclusions={blockerConclusions}
          onBlockerReview={onBlockerReview}
        />
        <DetailBlock
          title="Linked Activity"
          value={
            selectedPlanNode.activeRefs.length > 0
              ? selectedPlanNode.activeRefs.join('\n')
              : 'No active linked runs.'
          }
        />
      </aside>
    </div>
  );
}

interface CollapsiblePlanTreeNodeProps {
  node: OrchestrationPlanNode;
  orchestration: OrchestrationSnapshot;
  selectedPlanNodeId: EntityId;
  blockerConclusions: Record<EntityId, BlockerConclusion>;
  onPlanNodeSelect(planNodeId: EntityId): void;
  onBlockerReview(blockerId: EntityId): void;
}

function CollapsiblePlanTreeNode({
  node,
  orchestration,
  selectedPlanNodeId,
  blockerConclusions,
  onPlanNodeSelect,
  onBlockerReview,
}: CollapsiblePlanTreeNodeProps) {
  const [isExpanded, setIsExpanded] = useState(() => shouldExpandPlanNode(node));
  const nodeBlockers = getBlockersForNode(orchestration, node);

  return (
    <article className={`plan-node ${node.state}`}>
      <div className={`plan-node-row${node.id === selectedPlanNodeId ? ' selected' : ''}`}>
        <button
          className="plan-node-select"
          type="button"
          onClick={() => onPlanNodeSelect(node.id)}
        >
          <span className={`timeline-dot ${node.state}`} />
          <span>
            <strong>{node.title}</strong>
            <small>{node.summary}</small>
          </span>
          <em>{node.state}</em>
        </button>
        {node.children.length > 0 && (
          <button
            className="plan-node-toggle"
            type="button"
            aria-label={`${isExpanded ? 'Collapse' : 'Expand'} ${node.title}`}
            onClick={() => setIsExpanded((current) => !current)}
          >
            {isExpanded ? '-' : '+'}
          </button>
        )}
      </div>
      <RelevantBlockerNotice
        blockers={nodeBlockers}
        blockerConclusions={blockerConclusions}
        onBlockerReview={onBlockerReview}
        compact
      />
      {isExpanded && node.children.length > 0 && (
        <div className="plan-node-children">
          {node.children.map((child) => (
            <CollapsiblePlanTreeNode
              key={child.id}
              node={child}
              orchestration={orchestration}
              selectedPlanNodeId={selectedPlanNodeId}
              blockerConclusions={blockerConclusions}
              onPlanNodeSelect={onPlanNodeSelect}
              onBlockerReview={onBlockerReview}
            />
          ))}
        </div>
      )}
    </article>
  );
}

interface OrchestrationBlockersViewProps {
  orchestration: OrchestrationSnapshot;
  selectedBlocker?: OrchestrationBlocker;
  blockerConclusions: Record<EntityId, BlockerConclusion>;
  onBlockerSelect(blockerId: EntityId): void;
  onBlockerConclusion(blockerId: EntityId, conclusion: BlockerConclusion): void;
}

function OrchestrationBlockersView({
  orchestration,
  selectedBlocker,
  blockerConclusions,
  onBlockerSelect,
  onBlockerConclusion,
}: OrchestrationBlockersViewProps) {
  return (
    <div className="orchestration-blockers-view">
      <section className="blocker-board" aria-label="Product blockers">
        <header>
          <div>
            <p className="eyebrow">Direct Product Input</p>
            <h2>Blockers</h2>
          </div>
          <div className="metric-row">
            <span>
              {
                orchestration.blockers.filter(
                  (blocker) => getBlockerDisplayState(blocker, blockerConclusions) === 'open',
                ).length
              }{' '}
              open
            </span>
            <span>{orchestration.blockers.length} total</span>
          </div>
        </header>
        <div className="blocker-grid">
          {orchestration.blockers.map((blocker) => {
            const conclusion = blockerConclusions[blocker.id];
            const state = getBlockerDisplayState(blocker, blockerConclusions);

            return (
              <button
                className={`blocker-card ${state}${blocker.id === selectedBlocker?.id ? ' selected' : ''}`}
                key={blocker.id}
                type="button"
                onClick={() => onBlockerSelect(blocker.id)}
              >
                <header>
                  <strong>{blocker.title}</strong>
                  <span className={`state-pill ${state}`}>{state}</span>
                </header>
                <p>{conclusion?.conclusion ?? blocker.summary}</p>
                <small>
                  {blocker.kind} / {blocker.severity} severity
                </small>
              </button>
            );
          })}
        </div>
      </section>

      <BlockerDetailPanel
        blocker={selectedBlocker}
        conclusion={selectedBlocker ? blockerConclusions[selectedBlocker.id] : undefined}
        onConclusion={onBlockerConclusion}
      />
    </div>
  );
}

interface BlockerDetailPanelProps {
  blocker?: OrchestrationBlocker;
  conclusion?: BlockerConclusion;
  onConclusion(blockerId: EntityId, conclusion: BlockerConclusion): void;
}

function BlockerDetailPanel({ blocker, conclusion, onConclusion }: BlockerDetailPanelProps) {
  const [resolutionState, setResolutionState] = useState<BlockerConclusion['state']>(
    conclusion?.state ?? 'addressed',
  );
  const [resolutionText, setResolutionText] = useState(conclusion?.conclusion ?? '');

  useEffect(() => {
    setResolutionState(conclusion?.state ?? 'addressed');
    setResolutionText(conclusion?.conclusion ?? '');
  }, [blocker?.id, conclusion?.conclusion, conclusion?.state]);

  if (!blocker) {
    return (
      <section className="blocker-detail-panel" aria-label="Blocker detail">
        <p className="detail-empty">No blocker selected.</p>
      </section>
    );
  }

  const effectiveState = conclusion?.state ?? blocker.state;

  return (
    <section className="blocker-detail-panel" aria-label="Blocker detail">
      <header>
        <div>
          <p className="eyebrow">Blocker</p>
          <h2>{blocker.title}</h2>
        </div>
        <span className={`state-pill ${effectiveState}`}>{effectiveState}</span>
      </header>
      <p>{blocker.detail}</p>
      <dl className="blocker-metadata">
        <div>
          <dt>Severity</dt>
          <dd>{blocker.severity}</dd>
        </div>
        <div>
          <dt>Created By</dt>
          <dd>{blocker.createdByRole}</dd>
        </div>
        <div>
          <dt>Planner Input</dt>
          <dd>{blocker.nextPlannerContext}</dd>
        </div>
      </dl>
      <form
        className="blocker-resolution-form"
        onSubmit={(event) => {
          event.preventDefault();
          const trimmed = resolutionText.trim();

          if (!trimmed) {
            return;
          }

          onConclusion(blocker.id, {
            state: resolutionState,
            conclusion: trimmed,
            updatedAt: new Date().toISOString(),
          });
        }}
      >
        <label htmlFor={`blocker-resolution-${blocker.id}`}>{blocker.resolutionQuestion}</label>
        <select
          aria-label={`Resolution state for ${blocker.title}`}
          value={resolutionState}
          onChange={(event) => setResolutionState(event.target.value as BlockerConclusion['state'])}
        >
          <option value="addressed">Addressed</option>
          <option value="deferred">Deferred</option>
        </select>
        <textarea
          id={`blocker-resolution-${blocker.id}`}
          value={resolutionText}
          onChange={(event) => setResolutionText(event.target.value)}
          placeholder="Enter the product-side conclusion for the next planner"
        />
        <button className="primary-action" type="submit">
          <Check size={17} aria-hidden="true" />
          Save Conclusion
        </button>
      </form>
      {conclusion && (
        <aside className="blocker-conclusion" aria-label="Saved blocker conclusion">
          <strong>Saved for planner</strong>
          <p>{conclusion.conclusion}</p>
          <time dateTime={conclusion.updatedAt}>{formatDateTime(conclusion.updatedAt)}</time>
        </aside>
      )}
    </section>
  );
}

interface OrchestrationHistoryProps {
  orchestration: OrchestrationSnapshot;
  selectedPlanner?: OrchestrationPlannerTurn;
  selectedSlice?: OrchestrationWorkSlice;
  selectedStep?: OrchestrationStep;
  onPlannerSelect(plannerId: EntityId): void;
  onSliceSelect(sliceId: EntityId): void;
  onStepSelect(stepId: EntityId): void;
}

function OrchestrationHistory({
  orchestration,
  selectedPlanner,
  selectedSlice,
  selectedStep,
  onPlannerSelect,
  onSliceSelect,
  onStepSelect,
}: OrchestrationHistoryProps) {
  return (
    <div className="orchestration-history">
      <aside className="planner-rail" aria-label="Planner history">
        {orchestration.planners.map((planner) => (
          <button
            className={`planner-history-card${planner.id === selectedPlanner?.id ? ' selected' : ''}`}
            key={planner.id}
            type="button"
            onClick={() => onPlannerSelect(planner.id)}
          >
            <span>{formatDateTime(planner.startedAt)}</span>
            <strong>{planner.title}</strong>
            <small>{planner.workSlices.length} work slices</small>
          </button>
        ))}
      </aside>

      <section className="planner-detail" aria-label="Selected planner">
        {selectedPlanner && (
          <>
            <header className="planner-reasoning">
              <div>
                <p className="eyebrow">Planning Reasoning</p>
                <h2>{selectedPlanner.title}</h2>
              </div>
              <span className={`state-pill ${selectedPlanner.state}`}>{selectedPlanner.state}</span>
              <p>{selectedPlanner.reasoningSummary}</p>
            </header>

            <div className="work-slice-layout">
              <div className="work-slice-list" aria-label="Planner work slices">
                {selectedPlanner.workSlices.map((slice) => (
                  <button
                    className={`work-slice-button${slice.id === selectedSlice?.id ? ' selected' : ''}`}
                    key={slice.id}
                    type="button"
                    onClick={() => onSliceSelect(slice.id)}
                  >
                    <GitBranch size={16} aria-hidden="true" />
                    <span>{slice.title}</span>
                    <small>{slice.repo}</small>
                  </button>
                ))}
              </div>

              {selectedSlice && (
                <div className="work-slice-timeline-wrap">
                  <header>
                    <div>
                      <p className="eyebrow">{selectedSlice.repo}</p>
                      <h2>{selectedSlice.title}</h2>
                    </div>
                    <span className={`state-pill ${selectedSlice.state}`}>
                      {selectedSlice.state}
                    </span>
                  </header>
                  <p>{selectedSlice.summary}</p>
                  <ol className="work-slice-timeline">
                    {selectedSlice.steps.map((step) => (
                      <li key={step.id}>
                        <button
                          className={step.id === selectedStep?.id ? 'selected' : ''}
                          type="button"
                          onClick={() => onStepSelect(step.id)}
                        >
                          <span className={`timeline-dot ${step.state}`} />
                          <strong>{step.title}</strong>
                          <small>{step.role}</small>
                          <time dateTime={step.timestamp}>{formatDateTime(step.timestamp)}</time>
                        </button>
                      </li>
                    ))}
                  </ol>
                  <aside className="record-sidecar" aria-label="Recording">
                    <ScrollText size={17} aria-hidden="true" />
                    <div>
                      <strong>Recording</strong>
                      <p>{selectedSlice.recordNote}</p>
                    </div>
                  </aside>
                </div>
              )}
            </div>
          </>
        )}
      </section>

      <aside className="step-inspector" aria-label="Step detail">
        {selectedStep ? (
          <>
            <header>
              <p className="eyebrow">{selectedStep.role}</p>
              <h2>{selectedStep.title}</h2>
              <span className={`state-pill ${selectedStep.state}`}>{selectedStep.state}</span>
            </header>
            <DetailBlock title="Prompt" value={selectedStep.prompt} />
            <DetailBlock title="Output" value={selectedStep.output} />
            <DetailBlock
              title="Recorded Updates"
              value={orchestration.recordEntries
                .map(
                  (entry) =>
                    `${formatDateTime(entry.timestamp)} | ${entry.title}: ${entry.summary}`,
                )
                .join('\n')}
            />
          </>
        ) : (
          <p className="detail-empty">No step selected.</p>
        )}
      </aside>
    </div>
  );
}

interface DetailBlockProps {
  title: string;
  value: string;
}

function DetailBlock({ title, value }: DetailBlockProps) {
  return (
    <section className="detail-block">
      <h3>{title}</h3>
      <pre>{value}</pre>
    </section>
  );
}

function countWorkSlices(orchestration: OrchestrationSnapshot): number {
  return orchestration.planners.reduce((total, planner) => total + planner.workSlices.length, 0);
}

function truthStateClass(state: OrchestrationTruthState): string {
  return state.status.replaceAll('_', '-');
}

function statePillClass(state: OrchestrationTruthState): string {
  return `state-pill ${truthStateClass(state)}`;
}

function isLivePlanner(planner: OrchestrationPlannerTurn): boolean {
  return (
    ['planning', 'delegated', 'waiting'].includes(planner.state) ||
    planner.workSlices.some((slice) =>
      ['delegating', 'working', 'reviewing', 'merging', 'reporting', 'recording'].includes(
        slice.state,
      ),
    )
  );
}

function getPlannerStatusSummary(planner: OrchestrationPlannerTurn): string {
  const workingSlices = planner.workSlices.filter((slice) =>
    ['delegating', 'working', 'reviewing', 'merging', 'reporting', 'recording'].includes(
      slice.state,
    ),
  ).length;

  if (planner.state === 'planning') {
    return 'Planning the next executable step from current orchestration state.';
  }

  if (planner.state === 'waiting') {
    return 'Waiting for a concrete feedback item before it can continue.';
  }

  if (planner.state === 'delegated') {
    return `Delegated ${planner.workSlices.length} slice${planner.workSlices.length === 1 ? '' : 's'}; ${workingSlices} still active.`;
  }

  return `${planner.workSlices.length} slice${planner.workSlices.length === 1 ? '' : 's'} associated with this planner turn.`;
}

function getBlockersByIds(
  orchestration: OrchestrationSnapshot,
  blockerIds: EntityId[],
): OrchestrationBlocker[] {
  return blockerIds
    .map((blockerId) => orchestration.blockers.find((blocker) => blocker.id === blockerId))
    .filter((blocker): blocker is OrchestrationBlocker => blocker !== undefined);
}

function collectPlanNodes(root: OrchestrationPlanNode): OrchestrationPlanNode[] {
  return [root, ...root.children.flatMap((child) => collectPlanNodes(child))];
}

function getBlockersForNode(
  orchestration: OrchestrationSnapshot,
  node: OrchestrationPlanNode,
): OrchestrationBlocker[] {
  return node.blockerIds
    .map((blockerId) => orchestration.blockers.find((blocker) => blocker.id === blockerId))
    .filter((blocker): blocker is OrchestrationBlocker => blocker !== undefined);
}

function getBlockerDisplayState(
  blocker: OrchestrationBlocker,
  blockerConclusions: Record<EntityId, BlockerConclusion>,
): OrchestrationBlockerState {
  return blockerConclusions[blocker.id]?.state ?? blocker.state;
}

function findPlanNode(
  root: OrchestrationPlanNode,
  planNodeId: EntityId,
): OrchestrationPlanNode | undefined {
  if (root.id === planNodeId) {
    return root;
  }

  for (const child of root.children) {
    const match = findPlanNode(child, planNodeId);

    if (match) {
      return match;
    }
  }

  return undefined;
}

function findActivePlanNode(root: OrchestrationPlanNode): OrchestrationPlanNode | undefined {
  for (const child of root.children) {
    const match = findActivePlanNode(child);

    if (match) {
      return match;
    }
  }

  if (root.state === 'running' || root.state === 'blocked') {
    return root;
  }

  return undefined;
}

function shouldExpandPlanNode(node: OrchestrationPlanNode): boolean {
  return (
    node.state === 'running' ||
    node.children.some((child) => child.state === 'running' || shouldExpandPlanNode(child))
  );
}

function currentBuildStage(buildPackage: OrchestrationBuildPackage): OrchestrationBuildStage {
  return (
    buildPackage.stages.find((stage) => stage.state.status !== 'completed') ??
    buildPackage.stages[buildPackage.stages.length - 1]
  );
}

function displayBuildPackageTitle(buildPackage: OrchestrationBuildPackage): string {
  return buildPackage.title === internalIntakeDraftTitle
    ? 'Plan Builder intake draft'
    : buildPackage.title;
}

function buildPackageTitleDetail(buildPackage: OrchestrationBuildPackage): string {
  return buildPackage.title === internalIntakeDraftTitle
    ? 'Internal storage title; no user title has been set.'
    : buildPackage.folderPath;
}

function toIntakeAgentConversation(buildPackage: OrchestrationBuildPackage) {
  const conversation = mapBuildPackageToAgentConversation(buildPackage);

  return {
    ...conversation,
    artifacts: conversation.artifacts.filter(
      (artifact) =>
        artifact.truth.provenance === 'backend_response' ||
        artifact.truth.provenance === 'runtime_event' ||
        artifact.truth.provenance === 'persisted_snapshot',
    ),
    title: displayBuildPackageTitle(buildPackage),
  };
}

function toStageItems(
  buildPackage: OrchestrationBuildPackage,
  currentStageId: OrchestrationBuildStageId,
): OrchestrationStageItem[] {
  return buildPackage.stages.map((stage) => ({
    description: stage.summary,
    evidenceLabel: stageEvidenceLabel(stage.state),
    id: stage.id,
    isCurrent: stage.id === currentStageId,
    state: stage.state,
    title: stage.title,
  }));
}

function hasPlanBuilderOutputEvidence(buildPackage: OrchestrationBuildPackage): boolean {
  return (buildPackage.stageRuns ?? []).some(
    (stageRun) =>
      stageRun.stageId === 'plan-builder' &&
      stageRun.state.status === 'completed' &&
      Boolean(stageRun.outputArtifactId) &&
      (stageRun.state.provenance === 'backend_response' ||
        stageRun.state.provenance === 'runtime_event'),
  );
}

function hasInstantiatorOutputEvidence(buildPackage: OrchestrationBuildPackage): boolean {
  return (buildPackage.stageRuns ?? []).some(
    (stageRun) =>
      stageRun.stageId === 'instantiator' &&
      stageRun.state.status === 'completed' &&
      (Boolean(stageRun.outputArtifactId) ||
        buildPackage.generatedFiles.some(
          (file) =>
            file.state.provenance === 'backend_response' ||
            file.state.provenance === 'runtime_event' ||
            file.state.provenance === 'persisted_snapshot',
        )) &&
      (stageRun.state.provenance === 'backend_response' ||
        stageRun.state.provenance === 'runtime_event'),
  );
}

function toExpectedOutputItems(buildPackage: OrchestrationBuildPackage): OrchestrationFileItem[] {
  return buildPackage.generatedFiles.map((file) => ({
    detailLabel: file.purpose,
    evidenceLabel:
      file.state.provenance === 'unsupported'
        ? 'Expected output slot; no file has been generated.'
        : stageEvidenceLabel(file.state),
    id: file.name,
    kind: 'draft',
    name: file.name,
    state: file.state,
  }));
}

function toUploadedFileItems(files: UploadedConversationFile[]): OrchestrationFileItem[] {
  return files.map((file) => ({
    detailLabel: formatFileSize(file.size),
    evidenceLabel: 'Attached to this local draft session.',
    id: file.id,
    kind: 'uploaded',
    name: file.name,
    state: localDraftTruthState,
  }));
}

function toConversationItems(
  messages: OrchestrationConversationMessage[],
): ConversationMessageItem[] {
  return messages.map((message, index) => ({
    author: turnLabel(message, index),
    body: message.body,
    id: message.id,
    role: conversationMessageRole(message),
    sourceLabel: message.truth ? stageEvidenceLabel(message.truth) : (message.state ?? 'completed'),
    state: message.truth,
    timestampLabel: formatDateTime(message.createdAt),
  }));
}

function conversationMessageRole(
  message: OrchestrationConversationMessage,
): ConversationMessageItem['role'] {
  if (message.truth?.provenance === 'runtime_event') {
    return 'runtime';
  }

  return message.role;
}

function stageEvidenceLabel(state: OrchestrationTruthState): string {
  if (state.provenance === 'runtime_event') {
    return 'Runtime event evidence';
  }

  if (state.provenance === 'backend_response') {
    return 'Backend response evidence';
  }

  if (state.provenance === 'user_input') {
    return 'Local user input';
  }

  if (state.provenance === 'local_draft') {
    return 'Local draft session';
  }

  if (state.provenance === 'mock_fixture') {
    return 'Mock/demo fixture';
  }

  if (state.provenance === 'persisted_snapshot') {
    return 'Persisted snapshot';
  }

  return 'Unsupported capability';
}

function uploadedFilesFromFileList(fileList: FileList | null): UploadedConversationFile[] {
  if (!fileList || fileList.length === 0) {
    return [];
  }

  return Array.from(fileList).map((file) => ({
    id: `${file.name}-${file.size}-${file.lastModified}`,
    name: file.name,
    size: file.size,
    lastModified: file.lastModified,
  }));
}

function mergeUploadedFiles(
  current: UploadedConversationFile[],
  nextFiles: UploadedConversationFile[],
): UploadedConversationFile[] {
  const filesById = new Map(current.map((file) => [file.id, file]));

  for (const file of nextFiles) {
    filesById.set(file.id, file);
  }

  return [...filesById.values()];
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }

  const kib = bytes / 1024;
  if (kib < 1024) {
    return `${kib.toFixed(1)} KB`;
  }

  return `${(kib / 1024).toFixed(1)} MB`;
}
