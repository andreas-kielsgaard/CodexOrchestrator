import type { EntityId } from '../domain/model';
import {
  getOrchestrationStatusDescription,
  getOrchestrationStatusLabel,
  transitionOrchestrationState,
} from '../domain/orchestrationState';
import {
  integrationPendingTruthState,
  localDraftTruthState,
  type AddOrchestrationDraftNoteInput,
  type AttachOrchestrationDraftFilesInput,
  type CreateOrchestrationDraftInput,
  type OrchestrationBuildPackage,
  type OrchestrationBuildStage,
  type OrchestrationBuildStageId,
  type OrchestrationClient,
  type OrchestrationClientNotice,
  type OrchestrationClientState,
  type OrchestrationConversationMessage,
  type OrchestrationRegistrySnapshot,
  type OrchestrationRuntimeRoute,
  type OrchestrationSnapshot,
  type RequestOrchestrationBuildStageInput,
  type StartOrchestrationPlanBuilderRunInput,
  type StartOrchestrationInput,
  type StartOrchestrationResult,
  type UploadedConversationFile,
} from '../application/orchestrationClient';

interface LocalOrchestrationClientOptions {
  now?(): string;
  nextId?(prefix: string): EntityId;
}

export function createLocalOrchestrationClient(
  options: LocalOrchestrationClientOptions = {},
): OrchestrationClient {
  return new LocalOrchestrationClient(options);
}

class LocalOrchestrationClient implements OrchestrationClient {
  private buildPackages: OrchestrationBuildPackage[] = [];
  private orchestrations: OrchestrationSnapshot[] = [];
  private nextIndex = 1;
  private readonly now: () => string;
  private readonly nextId: (prefix: string) => EntityId;

  constructor(options: LocalOrchestrationClientOptions) {
    this.now = options.now ?? (() => new Date().toISOString());
    this.nextId =
      options.nextId ??
      ((prefix) => `${prefix}-${this.now().replaceAll(/[^0-9]/g, '')}-${this.nextIndex++}`);
  }

  async loadOrchestrations(): Promise<OrchestrationRegistrySnapshot> {
    return this.snapshot();
  }

  async createDraft(input: CreateOrchestrationDraftInput): Promise<OrchestrationBuildPackage> {
    const createdAt = this.now();
    const acceptedState = transitionOrchestrationState(localDraftTruthState, 'accept_user_input');
    const pendingState = transitionOrchestrationState(acceptedState, 'backend_unsupported');
    const messages: OrchestrationConversationMessage[] = [
      {
        id: this.nextId('system'),
        role: 'system',
        body: 'Draft ready. Add the source handoff, roadmap, or rough objective; backend plan-builder integration has not started yet.',
        createdAt,
        truth: { status: 'ready', provenance: 'local_draft' },
      },
      {
        id: this.nextId('user'),
        role: 'user',
        body: input.prompt,
        createdAt,
        state: 'completed',
        truth: acceptedState,
      },
      {
        id: this.nextId('local-draft'),
        role: 'system',
        body: `${getOrchestrationStatusLabel(pendingState)}. ${getOrchestrationStatusDescription(pendingState)} No plan-builder output yet.`,
        createdAt,
        state: 'completed',
        truth: pendingState,
      },
    ];
    const buildPackage = createBuildPackage(
      this.nextId('build'),
      createdAt,
      input.title,
      input.folderPath,
      input.prompt,
      input.files,
      messages,
    );

    this.buildPackages = [buildPackage, ...this.buildPackages];
    return buildPackage;
  }

  async addDraftNote(input: AddOrchestrationDraftNoteInput): Promise<OrchestrationBuildPackage> {
    const buildPackage = this.requireBuildPackage(input.buildPackageId);
    const updatedAt = this.now();
    const notice = unsupportedContinuationNotice();
    const updatedBuild: OrchestrationBuildPackage = {
      ...buildPackage,
      updatedAt,
      clientState: buildClientState(buildPackage.id, updatedAt, [notice]),
      messages: [
        ...buildPackage.messages,
        {
          id: this.nextId('user'),
          role: 'user',
          body: input.body,
          createdAt: updatedAt,
          state: 'completed',
          truth: localDraftTruthState,
        },
        {
          id: this.nextId('unsupported-continuation'),
          role: 'system',
          body: notice.message,
          createdAt: updatedAt,
          state: 'completed',
          truth: notice.truth,
        },
      ],
    };

    return this.replaceBuildPackage(updatedBuild);
  }

  async attachDraftFiles(
    input: AttachOrchestrationDraftFilesInput,
  ): Promise<OrchestrationBuildPackage> {
    const buildPackage = this.requireBuildPackage(input.buildPackageId);
    const updatedAt = this.now();
    const updatedBuild: OrchestrationBuildPackage = {
      ...buildPackage,
      updatedAt,
      clientState: buildClientState(buildPackage.id, updatedAt),
      files: mergeUploadedFiles(buildPackage.files, input.files),
    };

    return this.replaceBuildPackage(updatedBuild);
  }

  async requestBuildStage(
    input: RequestOrchestrationBuildStageInput,
  ): Promise<OrchestrationBuildPackage> {
    const buildPackage = this.requireBuildPackage(input.buildPackageId);
    const updatedAt = this.now();
    const notice = missingRuntimeNotice(input.stageId);
    const updatedBuild: OrchestrationBuildPackage = {
      ...buildPackage,
      updatedAt,
      clientState: buildClientState(buildPackage.id, updatedAt, [notice]),
      stages: buildPackage.stages.map((stage) =>
        stage.id === input.stageId
          ? {
              ...stage,
              state: notice.truth,
              summary:
                input.stageId === 'instantiator'
                  ? 'Build plan approval was accepted locally; instantiator runtime is unsupported.'
                  : stage.summary,
              detail: `${stage.detail} ${notice.message}`,
            }
          : input.stageId === 'instantiator' && stage.id === 'plan-review'
            ? {
                ...stage,
                state: { status: 'completed', provenance: 'backend_response' },
                summary: 'The user confirmed the Plan Builder proposal.',
                detail:
                  'Approval was accepted before attempting instantiation. No instantiator runtime route has started.',
              }
          : stage,
      ),
      messages: [
        ...buildPackage.messages,
        {
          id: this.nextId('unsupported'),
          role: 'system',
          body: notice.message,
          createdAt: updatedAt,
          state: 'completed',
          truth: notice.truth,
        },
      ],
    };

    return this.replaceBuildPackage(updatedBuild);
  }

  async startPlanBuilderRun(
    input: StartOrchestrationPlanBuilderRunInput,
  ): Promise<OrchestrationBuildPackage> {
    const buildPackage = this.requireBuildPackage(input.buildPackageId);
    const updatedAt = this.now();
    const notice = missingRuntimeNotice('plan-builder');
    const stageRun = {
      id: this.nextId('stage-run'),
      buildPackageId: buildPackage.id,
      stageId: 'plan-builder' as const,
      state: notice.truth,
      statusReason:
        'Local orchestration client cannot start Codex. The intake draft is preserved without runtime output.',
      eventIds: [],
      evidence: {
        schema: 'orchestration-stage-run-evidence/v1',
        runtimeRoute: 'local',
        unsupported: true,
      },
      createdAt: updatedAt,
      updatedAt,
    };
    const updatedBuild: OrchestrationBuildPackage = {
      ...buildPackage,
      updatedAt,
      clientState: buildClientState(buildPackage.id, updatedAt, [notice]),
      stages: buildPackage.stages.map((stage) =>
        stage.id === 'plan-builder'
          ? {
              ...stage,
              state: notice.truth,
              summary: 'Plan-builder runtime is unsupported in the local in-memory client.',
              detail: notice.message,
            }
          : stage,
      ),
      stageRuns: [...(buildPackage.stageRuns ?? []), stageRun],
      messages: [
        ...buildPackage.messages,
        {
          id: this.nextId('unsupported'),
          role: 'system',
          body: `${notice.message} The draft remains saved; no attached file contents were sent.`,
          createdAt: updatedAt,
          state: 'completed',
          truth: notice.truth,
        },
      ],
    };

    return this.replaceBuildPackage(updatedBuild);
  }

  async startOrchestration(input: StartOrchestrationInput): Promise<StartOrchestrationResult> {
    const buildPackage = this.requireBuildPackage(input.buildPackageId);
    const updatedAt = this.now();
    const notice: OrchestrationClientNotice = {
      id: 'missing-live-runtime',
      kind: 'missing_capability',
      title: 'Live orchestration runtime unavailable',
      message:
        'The UI cannot create Codex threads or live orchestration roots until runtime integration is implemented.',
      truth: integrationPendingTruthState,
    };
    const updatedBuild: OrchestrationBuildPackage = {
      ...buildPackage,
      updatedAt,
      clientState: buildClientState(buildPackage.id, updatedAt, [notice]),
      messages: [
        ...buildPackage.messages,
        {
          id: this.nextId('unsupported'),
          role: 'system',
          body: notice.message,
          createdAt: updatedAt,
          state: 'completed',
          truth: notice.truth,
        },
      ],
    };

    return {
      buildPackage: this.replaceBuildPackage(updatedBuild),
      clientState: updatedBuild.clientState,
    };
  }

  async loadOrchestration(id: EntityId): Promise<OrchestrationSnapshot | null> {
    return this.orchestrations.find((orchestration) => orchestration.id === id) ?? null;
  }

  async cancelDraft(buildPackageId: EntityId): Promise<OrchestrationRegistrySnapshot> {
    this.buildPackages = this.buildPackages.filter(
      (buildPackage) => buildPackage.id !== buildPackageId,
    );
    return this.snapshot();
  }

  private snapshot(): OrchestrationRegistrySnapshot {
    return {
      orchestrations: this.orchestrations,
      buildPackages: this.buildPackages,
      clientState: {
        status: 'integration_pending',
        provenance: 'unsupported',
        currentAction:
          'Local draft registry is held in memory; runtime persistence and Codex thread support are pending.',
        updatedAt: this.now(),
        persisted: false,
        runtimeSupported: false,
        notices: [registryNotice],
      },
    };
  }

  private requireBuildPackage(buildPackageId: EntityId): OrchestrationBuildPackage {
    const buildPackage = this.buildPackages.find((candidate) => candidate.id === buildPackageId);

    if (!buildPackage) {
      throw new Error(`Unknown orchestration draft: ${buildPackageId}`);
    }

    return buildPackage;
  }

  private replaceBuildPackage(buildPackage: OrchestrationBuildPackage): OrchestrationBuildPackage {
    this.buildPackages = this.buildPackages.map((candidate) =>
      candidate.id === buildPackage.id ? buildPackage : candidate,
    );
    return buildPackage;
  }
}

function createBuildPackage(
  id: EntityId,
  createdAt: string,
  title: string,
  folderPath: string,
  prompt: string,
  files: UploadedConversationFile[],
  messages: OrchestrationConversationMessage[],
): OrchestrationBuildPackage {
  const planPreview = [
    firstMeaningfulLine(prompt) ?? title,
    'Separate strategic problem structure from executable work-slice planning.',
    'Prepare instantiator-ready files before live root threads begin.',
  ];

  return {
    id,
    title,
    folderPath,
    sourcePrompt: prompt,
    createdAt,
    updatedAt: createdAt,
    clientState: buildClientState(id, createdAt),
    messages,
    files,
    stages: createInitialBuildStages(),
    stageRuns: [],
    runtimeRoutes: [blockedPlanBuilderRoute(createdAt)],
    generatedFiles: [],
    planPreview,
  };
}

function createInitialBuildStages(): OrchestrationBuildStage[] {
  return [
    {
      id: 'plan-builder',
      title: 'Plan Builder',
      state: integrationPendingTruthState,
      summary: 'Prompt accepted locally; no plan-builder output exists yet.',
      detail:
        'The app captured the prompt and files in this session. Backend plan-builder integration is still pending.',
    },
    {
      id: 'plan-review',
      title: 'Review Pending',
      state: { status: 'blocked', provenance: 'unsupported' },
      summary: 'No plan-builder output is available to review.',
      detail: 'Review waits for real plan-builder output from a supported backend path.',
    },
    {
      id: 'instantiator',
      title: 'Instantiator',
      state: { status: 'blocked', provenance: 'unsupported' },
      summary: 'Instantiation is not available in this UI path yet.',
      detail:
        'No files have been generated. The future instantiator step needs backend support before it can write to the selected folder.',
    },
    {
      id: 'root-startup',
      title: 'Root Startup',
      state: { status: 'blocked', provenance: 'unsupported' },
      summary: 'Live root startup has not been prepared.',
      detail: 'No root orchestration or record threads have been created from this draft.',
    },
  ];
}

function buildClientState(
  id: EntityId,
  updatedAt: string,
  notices: OrchestrationClientNotice[] = [registryNotice],
): OrchestrationClientState {
  return {
    id,
    status: 'integration_pending',
    provenance: 'unsupported',
    currentAction:
      'Draft is held locally; no explicit task/worktree route is linked for plan-builder, so no Codex run can start.',
    updatedAt,
    persisted: false,
    runtimeSupported: false,
    notices,
    primaryAction: {
      id: 'request-build-stage',
      label: 'Backend integration pending',
      enabled: false,
      reason:
        'Plan-builder requires an explicit linked task/worktree route. This local draft has none.',
    },
  };
}

function missingRuntimeNotice(stageId: OrchestrationBuildStageId): OrchestrationClientNotice {
  if (stageId === 'plan-builder') {
    return {
      id: 'missing-plan-builder-route',
      kind: 'blocker',
      title: 'Plan-builder route required',
      message:
        'Plan builder cannot start because this draft has no explicit linked task/worktree route. No Codex run was started.',
      truth: { status: 'blocked', provenance: 'unsupported' },
    };
  }

  return {
    id: `missing-${stageId}-runtime`,
    kind: 'missing_capability',
    title: 'Runtime integration pending',
    message:
      stageId === 'instantiator'
        ? 'The build plan approval was accepted locally, but instantiation cannot start because no instantiator runtime route is implemented. No files were generated.'
        : `${stageTitle(stageId)} cannot advance because the orchestration runtime adapter is not implemented yet.`,
    truth: integrationPendingTruthState,
  };
}

function unsupportedContinuationNotice(): OrchestrationClientNotice {
  return {
    id: 'unsupported-plan-builder-continuation',
    kind: 'missing_capability',
    title: 'Runtime continuation unsupported',
    message:
      'Feedback was preserved locally, but it was not sent to the same Plan Builder runtime conversation because continuation is unsupported in this path.',
    truth: integrationPendingTruthState,
  };
}

function blockedPlanBuilderRoute(updatedAt: string): OrchestrationRuntimeRoute {
  return {
    stageId: 'plan-builder',
    status: 'blocked',
    truth: { status: 'blocked', provenance: 'unsupported' },
    reason:
      'No explicit Open Task/worktree runtime route is linked to this orchestration draft. The selected folder is not treated as a runnable cwd.',
    runtimeCommand: 'startCodexTaskRun',
    updatedAt,
  };
}

const registryNotice: OrchestrationClientNotice = {
  id: 'runtime-integration-pending',
  kind: 'missing_capability',
  title: 'Runtime integration pending',
  message:
    'Orchestration drafts are local-only and are not persisted, generated, or backed by Codex runtime threads.',
  truth: integrationPendingTruthState,
};

function stageTitle(stageId: OrchestrationBuildStageId): string {
  if (stageId === 'plan-builder') {
    return 'Plan builder';
  }

  if (stageId === 'plan-review') {
    return 'Plan review';
  }

  if (stageId === 'instantiator') {
    return 'Instantiator';
  }

  return 'Root startup';
}

function firstMeaningfulLine(value: string): string | undefined {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => line.length > 0);
}

function mergeUploadedFiles(
  current: UploadedConversationFile[],
  next: UploadedConversationFile[],
): UploadedConversationFile[] {
  const existingKeys = new Set(current.map((file) => uploadedFileKey(file)));
  const uniqueNext = next.filter((file) => {
    const key = uploadedFileKey(file);

    if (existingKeys.has(key)) {
      return false;
    }

    existingKeys.add(key);
    return true;
  });

  return [...current, ...uniqueNext];
}

function uploadedFileKey(file: UploadedConversationFile): string {
  return [file.name, file.size, file.lastModified ?? 'unknown'].join(':');
}
