import type {
  AgentIdentityDto,
  AgentRuntimeEventDto,
  AgentSessionDetailsDto,
} from '../../application/agentSessions';
import {
  assignAgentIdentity,
  harnessAgentNamePools,
  harnessVisualIdentities,
  productDefaultAgentNames,
  validateAgentNamePool,
} from '../../application/agentSessions';
import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementRead,
  ConversationHarnessManagementSnapshot,
  ConversationHarnessManagementSource,
  HarnessConfigurationCatalogs,
  HarnessEffectiveConfiguration,
  HarnessModelPolicy,
  HarnessReasoningLevel,
} from '../../application/conversationHarnesses';
import catalogJson from '../../../src-tauri/src/orchestration/conversation_harness_catalog.json';

interface RecordedCatalog {
  readonly schemaVersion: number;
  readonly harnesses: readonly RecordedProfile[];
}

interface RecordedProfile {
  readonly key: string;
  readonly version: number;
  readonly context: string;
  readonly skillGuidance: readonly {
    readonly canonicalName: string;
    readonly canonicalPath: string;
    readonly purpose: string;
    readonly useWhen: string;
  }[];
  readonly runtime: {
    readonly model: string | null;
    readonly reasoningEffort: string | null;
    readonly sandbox: string;
    readonly approvalPolicy: string;
  };
  readonly mcp: {
    readonly required: boolean;
    readonly enabledTools: readonly string[];
  };
  readonly lifecycle: {
    readonly contextDelivery: string;
    readonly completionCriteria: readonly string[];
  };
}

const skillDocuments = import.meta.glob('../../../.agents/skills/*/SKILL.md', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>;
const catalog = catalogJson as RecordedCatalog;
export const recordedHarnessInspectorSessionId = 'recorded-harness-inspector-plan-builder';
const profileKey = 'epic_plan_builder';
const recordedAt = '2026-07-17T09:00:00.000Z';
const recordedProfile = catalog.harnesses.find((profile) => profile.key === profileKey);
if (!recordedProfile) throw new Error('Recorded Harness profile is missing.');
const recordedSessionAppliedRevision = Math.max(1, recordedProfile.version - 1);
const recordedAgentIdentity = assignAgentIdentity({
  sessionId: recordedHarnessInspectorSessionId,
  harnessKey: profileKey,
  harnessRole: 'Epic Plan Builder',
  harnessRevision: recordedSessionAppliedRevision,
  visualIdentity: harnessVisualIdentities.epic_plan_builder,
  permittedNames: harnessAgentNamePools.epic_plan_builder,
  assignedAt: recordedAt,
  assignmentKind: 'recorded_preview',
});

export const recordedHarnessInspectorSessionDetails: AgentSessionDetailsDto =
  createRecordedHarnessInspectorSession();

export function createRecordedHarnessManagementSource(options?: {
  onSessionIdentityChange?(identity: AgentIdentityDto): void;
}): ConversationHarnessManagementSource {
  let snapshot: ConversationHarnessManagementSnapshot | null = null;
  return {
    async load({ sessionId }) {
      if (sessionId !== recordedHarnessInspectorSessionId) return unboundRead();
      try {
        snapshot ??= buildSnapshot(sessionId);
        return availableRead(snapshot);
      } catch {
        return {
          kind: 'unavailable',
          reason: 'The harness configuration could not be loaded.',
        };
      }
    },
    async dispatch({ sessionId, command }) {
      if (sessionId !== recordedHarnessInspectorSessionId) return unboundRead();
      try {
        snapshot ??= buildSnapshot(sessionId);
        snapshot = reduceRecordedCommand(snapshot, command);
        if (command.kind === 'update_session_identity' && snapshot.agentIdentity)
          options?.onSessionIdentityChange?.(snapshot.agentIdentity);
        return availableRead(snapshot);
      } catch (error) {
        return {
          kind: 'unavailable',
          reason:
            error instanceof Error ? error.message : 'The preview action could not be recorded.',
        };
      }
    },
  };
}

export const recordedHarnessInspectorSource = createRecordedHarnessManagementSource();

function buildSnapshot(sessionId: string): ConversationHarnessManagementSnapshot {
  if (catalog.schemaVersion !== 2) throw new Error('Unsupported harness catalog.');
  const profile = catalog.harnesses.find((candidate) => candidate.key === profileKey);
  if (!profile || profile.version < 1 || !profile.context.trim())
    throw new Error('Harness configuration is incomplete.');
  if (profile.lifecycle.contextDelivery !== 'first_query')
    throw new Error('Harness prompt policy is unsupported.');
  if (profile.runtime.approvalPolicy !== 'never')
    throw new Error('Harness approval policy is unsupported.');
  const sandbox = runtimeSandbox(profile.runtime.sandbox);
  if (
    (profile.runtime.model !== null && typeof profile.runtime.model !== 'string') ||
    (profile.runtime.reasoningEffort !== null &&
      typeof profile.runtime.reasoningEffort !== 'string') ||
    typeof profile.mcp.required !== 'boolean' ||
    profile.skillGuidance.some(
      (skill) =>
        !skill.canonicalName.trim() ||
        !skill.canonicalPath.startsWith('.agents/skills/') ||
        !skill.purpose.trim() ||
        !skill.useWhen.trim(),
    ) ||
    profile.mcp.enabledTools.some((tool) => !tool.trim()) ||
    profile.lifecycle.completionCriteria.some((criterion) => !criterion.trim())
  )
    throw new Error('Harness configuration is incomplete.');

  const catalogs = buildCatalogs();
  const currentConfiguration = buildConfiguration(profile, sandbox, catalogs);
  validateConfiguration(currentConfiguration, catalogs);
  const snapshot: ConversationHarnessManagementSnapshot = {
    sessionId,
    harnessKey: profile.key,
    agentIdentity: recordedAgentIdentity,
    catalogs,
    workingCopy: null,
    versionControl: {
      support: 'recorded_preview',
      pushedRevision: profile.version,
      versions: [
        {
          revision: recordedSessionAppliedRevision,
          label: 'Session binding baseline',
          status: 'pushed',
          configuration: currentConfiguration,
          activeSessionCount: 1,
          queuedSessionCount: 0,
          committedAt: '2026-07-11T14:30:00.000Z',
        },
        {
          revision: profile.version,
          label: 'Next-prompt update policy',
          status: 'pushed',
          configuration: currentConfiguration,
          activeSessionCount: 0,
          queuedSessionCount: 0,
          committedAt: recordedAt,
        },
      ],
      reason: 'Recorded actions remain in memory until this preview is closed.',
    },
    sessionBinding: {
      state: 'behind',
      appliedRevision: recordedSessionAppliedRevision,
      desiredRevision: null,
      relevantSessionCount: 1,
      executingPreviousInvocation: false,
      reason: 'This Session is still using the previous pushed version.',
    },
    modelChoices: {
      revisionProposals: [
        {
          revision: recordedSessionAppliedRevision,
          policy: policyFromConfiguration(currentConfiguration),
          dirty: false,
          updatedAt: recordedAt,
        },
        {
          revision: profile.version,
          policy: policyFromConfiguration(currentConfiguration),
          dirty: false,
          updatedAt: recordedAt,
        },
      ],
      sessionOverride: null,
      userPreference: {
        support: 'recorded_preference_register',
        lastUsedModel: 'gpt-5.6-terra',
        lastUsedReasoning: 'high',
        reason:
          'Recorded user preference only; production user-preference persistence is not connected.',
      },
      resolvedForCurrentSession: {
        model: null,
        reasoning: null,
        source: 'provisional_fallback',
      },
    },
  };
  return withResolvedModelChoice(snapshot);
}

function buildCatalogs(): HarnessConfigurationCatalogs {
  const skills = Object.entries(skillDocuments)
    .map(([sourcePath, document]) => {
      const pathMatch = sourcePath
        .replaceAll('\\', '/')
        .match(/\.agents\/skills\/([^/]+)\/SKILL\.md$/);
      const name = frontmatterValue(document, 'name') ?? pathMatch?.[1] ?? '';
      return {
        name,
        path: `.agents/skills/${pathMatch?.[1] ?? name}/SKILL.md`,
        description: frontmatterValue(document, 'description') ?? 'Product skill.',
        text: document,
      };
    })
    .filter((skill) => skill.name)
    .sort((left, right) => left.name.localeCompare(right.name));
  return {
    agentNames: {
      source: 'product_default_pool',
      items: productDefaultAgentNames,
      reason: 'Harness subsets are selected from the 100-name product pool.',
    },
    agentVisualIdentities: {
      source: 'product_visual_catalog',
      items: [
        {
          identity: harnessVisualIdentities.epic_plan_builder,
          label: 'Drafting compass',
        },
        {
          identity: harnessVisualIdentities.epic_bootstrap_generator,
          label: 'Bootstrap package',
        },
        {
          identity: harnessVisualIdentities.epic_runner,
          label: 'Runner route',
        },
      ],
      reason: 'Recorded product visual identities for Harness-backed Agent Sessions.',
    },
    skills: {
      source: 'checked_in_product_catalog',
      items: skills,
      reason: 'This preview reads every checked-in product skill with a SKILL.md file.',
    },
    tools: {
      source: 'recorded_harness_tool_catalog',
      items: [
        {
          name: 'submit_epic_plan_proposal',
          description: 'Save the Session proposal through the Plan Builder application boundary.',
        },
        {
          name: 'request_epic_initiation',
          description: 'Ask the application to start its explicit Epic initiation confirmation.',
        },
      ],
      reason: 'This recorded catalog covers the tools connected to Epic Plan Builder.',
    },
    models: {
      source: 'recorded_catalog',
      items: [
        {
          id: 'gpt-5.6-terra',
          label: 'GPT-5.6 Terra',
          reasoningLevels: ['low', 'medium', 'high', 'xhigh'],
        },
        {
          id: 'gpt-5.6-sol',
          label: 'GPT-5.6 Sol',
          reasoningLevels: ['medium', 'high', 'xhigh'],
        },
      ],
      reason:
        'No application model capability catalog is connected; these are recorded prototype options.',
    },
  };
}

function buildConfiguration(
  profile: RecordedProfile,
  sandbox: HarnessEffectiveConfiguration['runtime']['sandbox'],
  catalogs: HarnessConfigurationCatalogs,
): HarnessEffectiveConfiguration {
  return {
    identity: {
      name: 'Epic Plan Builder',
      machineKey: profile.key,
      permittedAgentNames: harnessAgentNamePools.epic_plan_builder,
      visualIdentity: harnessVisualIdentities.epic_plan_builder,
    },
    promptPrefix: {
      content: profile.context,
      initialDelivery: 'prepend',
      contextCompressionDelivery: 'deferred',
    },
    skills: {
      availableDiscoveryPolicy: 'whitelist',
      items: profile.skillGuidance.map((skill) => ({
        name: skill.canonicalName,
        path: skill.canonicalPath,
        purpose: skill.purpose,
        useWhen: skill.useWhen,
        policy: 'available',
      })),
    },
    tools: {
      availableDiscoveryPolicy: 'whitelist',
      items: profile.mcp.enabledTools.map((name) => ({
        name,
        policy: 'every_invocation',
      })),
      schemaBoundary:
        'Applicability labels describe exposure timing only. Tool schemas remain runtime-owned and are not ingested as skill text.',
    },
    runtime: {
      modelPolicyMode: 'adjustable_proposal',
      models: catalogs.models.items.map((model) => ({
        modelId: model.id,
        allowed: profile.runtime.model === null || profile.runtime.model === model.id,
        minReasoning: model.reasoningLevels[0],
        maxReasoning: model.reasoningLevels[model.reasoningLevels.length - 1],
      })),
      defaultModel: profile.runtime.model,
      defaultReasoning: reasoningLevel(profile.runtime.reasoningEffort),
      sandbox,
      sandboxOptions: ['read_only', 'workspace_write', 'danger_full_access'],
      approvalPolicy: 'never',
      approvalPolicyOptions: ['never'],
      authoritySummary:
        'Sandbox and approval settings limit runtime access. Application actions still require product authorization.',
    },
    hooks: profile.lifecycle.completionCriteria.map((criterion) => ({
      name: humanize(criterion),
      status: 'proposed',
      detail: `Proposed application hook reference: ${criterion}.`,
    })),
    updatePolicy: {
      status: 'configured',
      delivery: 'next_prompt',
      avoidDuplicateGuidance: true,
      notifyRemovedItems: true,
      promptReconstruction: 'deferred',
    },
  };
}

function reduceRecordedCommand(
  snapshot: ConversationHarnessManagementSnapshot,
  command: ConversationHarnessManagementCommand,
): ConversationHarnessManagementSnapshot {
  if (command.kind === 'start_edit') {
    if (snapshot.workingCopy) return snapshot;
    const base = findVersion(snapshot, command.baseRevision);
    return {
      ...snapshot,
      workingCopy: {
        baseRevision: base.revision,
        draftRevision: 1,
        dirty: false,
        configuration: base.configuration,
      },
    };
  }
  if (command.kind === 'save_working_copy') {
    const workingCopy = requireWorkingCopy(snapshot);
    validateConfiguration(command.configuration, snapshot.catalogs);
    return {
      ...snapshot,
      workingCopy: {
        ...workingCopy,
        draftRevision: workingCopy.draftRevision + 1,
        dirty: true,
        configuration: command.configuration,
      },
    };
  }
  if (command.kind === 'commit') {
    const workingCopy = requireWorkingCopy(snapshot);
    assertDraftRevision(workingCopy.draftRevision, command.expectedDraftRevision);
    if (!workingCopy.dirty) return snapshot;
    validateConfiguration(workingCopy.configuration, snapshot.catalogs);
    const revision =
      Math.max(...snapshot.versionControl.versions.map((version) => version.revision)) + 1;
    return {
      ...snapshot,
      workingCopy: null,
      versionControl: {
        ...snapshot.versionControl,
        versions: [
          ...snapshot.versionControl.versions,
          {
            revision,
            label: 'Harness settings update',
            status: 'committed',
            configuration: workingCopy.configuration,
            activeSessionCount: 0,
            queuedSessionCount: 0,
            committedAt: new Date().toISOString(),
          },
        ],
      },
      modelChoices: {
        ...snapshot.modelChoices,
        revisionProposals:
          workingCopy.configuration.runtime.modelPolicyMode === 'adjustable_proposal'
            ? [
                ...snapshot.modelChoices.revisionProposals,
                {
                  revision,
                  policy: policyFromConfiguration(workingCopy.configuration),
                  dirty: false,
                  updatedAt: new Date().toISOString(),
                },
              ]
            : snapshot.modelChoices.revisionProposals,
      },
    };
  }
  if (command.kind === 'update_session_identity') {
    const name = command.name.trim();
    if (!name) throw new Error('Agent name must not be blank.');
    const visualCatalog = snapshot.catalogs.agentVisualIdentities.items;
    if (
      !visualCatalog.some(
        (entry) =>
          entry.identity.token === command.visualIdentity.token &&
          entry.identity.accent === command.visualIdentity.accent,
      )
    )
      throw new Error('Agent visual identity is outside the recorded product catalog.');
    if (!snapshot.agentIdentity) throw new Error('This Session has no Agent identity to update.');
    return {
      ...snapshot,
      agentIdentity: {
        ...snapshot.agentIdentity,
        name,
        visualIdentity: command.visualIdentity,
        assignment: {
          ...snapshot.agentIdentity.assignment,
          kind: 'recorded_preview',
          assignedAt: new Date().toISOString(),
        },
      },
    };
  }
  if (command.kind === 'save_model_proposal') {
    const version = findVersion(snapshot, command.revision);
    if (version.configuration.runtime.modelPolicyMode !== 'adjustable_proposal')
      throw new Error('This Harness revision fixes its model policy.');
    validateModelPolicy(command.policy, snapshot.catalogs);
    return withResolvedModelChoice({
      ...snapshot,
      modelChoices: {
        ...snapshot.modelChoices,
        revisionProposals: [
          ...snapshot.modelChoices.revisionProposals.filter(
            (proposal) => proposal.revision !== command.revision,
          ),
          {
            revision: command.revision,
            policy: command.policy,
            dirty: true,
            updatedAt: new Date().toISOString(),
          },
        ],
      },
    });
  }
  if (command.kind === 'set_session_model_override') {
    if (command.override?.enabled) {
      validateModelPolicy(command.override.policy, snapshot.catalogs);
      validatePolicyNarrowing(
        command.override.policy,
        policyForAppliedRevision(snapshot),
        snapshot.catalogs,
      );
    }
    return withResolvedModelChoice({
      ...snapshot,
      modelChoices: {
        ...snapshot.modelChoices,
        sessionOverride: command.override,
      },
    });
  }
  if (command.kind === 'push') {
    findVersion(snapshot, command.revision);
    return queueRecordedVersion(
      {
        ...snapshot,
        versionControl: {
          ...snapshot.versionControl,
          pushedRevision: command.revision,
          versions: snapshot.versionControl.versions.map((version) =>
            version.revision === command.revision ? { ...version, status: 'pushed' } : version,
          ),
        },
      },
      command.revision,
      'all_relevant_sessions',
      'The pushed version is queued for every relevant Session at its next prompt.',
    );
  }
  return queueRecordedVersion(
    snapshot,
    command.revision,
    command.scope,
    command.scope === 'current_session'
      ? 'The selected version is queued for this Session at its next prompt.'
      : 'The selected version is queued for every relevant Session at its next prompt.',
  );
}

function queueRecordedVersion(
  snapshot: ConversationHarnessManagementSnapshot,
  revision: number,
  scope: 'current_session' | 'all_relevant_sessions',
  queuedReason: string,
): ConversationHarnessManagementSnapshot {
  findVersion(snapshot, revision);
  const alreadyApplied = snapshot.sessionBinding.appliedRevision === revision;
  const relevant = snapshot.sessionBinding.relevantSessionCount ?? 0;
  const currentDesiredRevision = snapshot.sessionBinding.desiredRevision;
  const versions = snapshot.versionControl.versions.map((version) => {
    let queuedSessionCount = version.queuedSessionCount;
    if (
      currentDesiredRevision !== null &&
      version.revision === currentDesiredRevision &&
      currentDesiredRevision !== revision
    )
      queuedSessionCount = Math.max(0, queuedSessionCount - 1);
    if (version.revision === revision) {
      queuedSessionCount = alreadyApplied
        ? 0
        : scope === 'all_relevant_sessions'
          ? Math.max(0, relevant - version.activeSessionCount)
          : Math.max(1, queuedSessionCount);
    }
    return { ...version, queuedSessionCount };
  });
  return {
    ...snapshot,
    versionControl: {
      ...snapshot.versionControl,
      versions,
    },
    sessionBinding: {
      ...snapshot.sessionBinding,
      state: alreadyApplied ? 'current' : 'queued',
      desiredRevision: alreadyApplied ? null : revision,
      reason: alreadyApplied ? 'This Session already uses the selected version.' : queuedReason,
    },
  };
}

function validateConfiguration(
  configuration: HarnessEffectiveConfiguration,
  catalogs: HarnessConfigurationCatalogs,
): void {
  if (!configuration.identity.name.trim() || !configuration.identity.machineKey.trim())
    throw new Error('Harness identity is incomplete.');
  if (configuration.identity.permittedAgentNames) {
    const names = validateAgentNamePool(configuration.identity.permittedAgentNames);
    const catalogNames = new Set(catalogs.agentNames.items);
    const unknown = names.find((name) => !catalogNames.has(name));
    if (unknown) throw new Error(`Agent name is outside the product pool: ${unknown}`);
  }
  if (!configuration.promptPrefix.content.trim())
    throw new Error('Prompt prefix must not be empty.');
  const skillNames = new Set(catalogs.skills.items.map((skill) => skill.name));
  assertUniqueCatalogItems(
    configuration.skills.items.map((skill) => skill.name),
    skillNames,
    'skill',
  );
  const toolNames = new Set(catalogs.tools.items.map((tool) => tool.name));
  assertUniqueCatalogItems(
    configuration.tools.items.map((tool) => tool.name),
    toolNames,
    'tool',
  );
  validateModelPolicy(policyFromConfiguration(configuration), catalogs);
}

function validateModelPolicy(
  policy: HarnessModelPolicy,
  catalogs: HarnessConfigurationCatalogs,
): void {
  const modelConfigurations = new Map(policy.models.map((model) => [model.modelId, model]));
  if (modelConfigurations.size !== policy.models.length)
    throw new Error('Model options must be unique.');
  for (const model of catalogs.models.items) {
    const configurationModel = modelConfigurations.get(model.id);
    if (!configurationModel) throw new Error(`Model option is missing: ${model.id}`);
    const minimum = model.reasoningLevels.indexOf(configurationModel.minReasoning);
    const maximum = model.reasoningLevels.indexOf(configurationModel.maxReasoning);
    if (minimum < 0 || maximum < minimum)
      throw new Error(`Reasoning range is invalid for ${model.label}.`);
  }
  if (policy.defaultModel) {
    const defaultModel = modelConfigurations.get(policy.defaultModel);
    if (!defaultModel?.allowed) throw new Error('Default model must be allowed.');
    if (policy.defaultReasoning) {
      const catalogModel = catalogs.models.items.find((model) => model.id === policy.defaultModel);
      if (!catalogModel) throw new Error('Default model is outside the recorded catalog.');
      const selected = catalogModel.reasoningLevels.indexOf(policy.defaultReasoning);
      const minimum = catalogModel.reasoningLevels.indexOf(defaultModel.minReasoning);
      const maximum = catalogModel.reasoningLevels.indexOf(defaultModel.maxReasoning);
      if (selected < minimum || selected > maximum)
        throw new Error('Default reasoning must fit the default model range.');
    }
  } else if (policy.defaultReasoning) {
    throw new Error('Choose a default model before choosing default reasoning.');
  }
}

function validatePolicyNarrowing(
  candidate: HarnessModelPolicy,
  base: HarnessModelPolicy,
  catalogs: HarnessConfigurationCatalogs,
): void {
  for (const candidateModel of candidate.models) {
    if (!candidateModel.allowed) continue;
    const baseModel = base.models.find((model) => model.modelId === candidateModel.modelId);
    if (!baseModel?.allowed)
      throw new Error('A Session override cannot enable a model outside its Harness policy.');
    const levels =
      catalogs.models.items.find((model) => model.id === candidateModel.modelId)?.reasoningLevels ??
      [];
    if (
      levels.indexOf(candidateModel.minReasoning) < levels.indexOf(baseModel.minReasoning) ||
      levels.indexOf(candidateModel.maxReasoning) > levels.indexOf(baseModel.maxReasoning)
    )
      throw new Error('A Session override cannot widen its Harness reasoning range.');
  }
}

function policyForAppliedRevision(
  snapshot: ConversationHarnessManagementSnapshot,
): HarnessModelPolicy {
  const applied = findVersion(snapshot, snapshot.sessionBinding.appliedRevision ?? -1);
  const proposal = snapshot.modelChoices.revisionProposals.find(
    (candidate) => candidate.revision === applied.revision,
  );
  return applied.configuration.runtime.modelPolicyMode === 'adjustable_proposal' && proposal
    ? proposal.policy
    : policyFromConfiguration(applied.configuration);
}

function policyFromConfiguration(configuration: HarnessEffectiveConfiguration): HarnessModelPolicy {
  return {
    models: configuration.runtime.models,
    defaultModel: configuration.runtime.defaultModel,
    defaultReasoning: configuration.runtime.defaultReasoning,
  };
}

function withResolvedModelChoice(
  snapshot: ConversationHarnessManagementSnapshot,
): ConversationHarnessManagementSnapshot {
  const applied = snapshot.versionControl.versions.find(
    (version) => version.revision === snapshot.sessionBinding.appliedRevision,
  );
  if (!applied)
    return {
      ...snapshot,
      modelChoices: {
        ...snapshot.modelChoices,
        resolvedForCurrentSession: {
          model: null,
          reasoning: null,
          source: 'not_connected',
        },
      },
    };
  const proposal = snapshot.modelChoices.revisionProposals.find(
    (candidate) => candidate.revision === applied.revision,
  );
  const revisionPolicy =
    applied.configuration.runtime.modelPolicyMode === 'adjustable_proposal' && proposal
      ? proposal.policy
      : policyFromConfiguration(applied.configuration);
  const sessionPolicy = snapshot.modelChoices.sessionOverride?.enabled
    ? snapshot.modelChoices.sessionOverride.policy
    : null;
  const constraints = sessionPolicy ?? revisionPolicy;
  const explicitModel = sessionPolicy?.defaultModel ?? revisionPolicy.defaultModel;
  const explicitReasoning = sessionPolicy?.defaultReasoning ?? revisionPolicy.defaultReasoning;
  if (
    explicitModel &&
    isAllowedChoice(explicitModel, explicitReasoning, constraints, snapshot.catalogs)
  )
    return {
      ...snapshot,
      modelChoices: {
        ...snapshot.modelChoices,
        resolvedForCurrentSession: {
          model: explicitModel,
          reasoning: explicitReasoning,
          source: sessionPolicy
            ? 'session_override'
            : applied.configuration.runtime.modelPolicyMode === 'adjustable_proposal'
              ? 'revision_proposal'
              : 'harness_revision',
        },
      },
    };
  const preference = snapshot.modelChoices.userPreference;
  if (
    preference.lastUsedModel &&
    isAllowedChoice(
      preference.lastUsedModel,
      preference.lastUsedReasoning,
      constraints,
      snapshot.catalogs,
    )
  )
    return {
      ...snapshot,
      modelChoices: {
        ...snapshot.modelChoices,
        resolvedForCurrentSession: {
          model: preference.lastUsedModel,
          reasoning: preference.lastUsedReasoning,
          source: 'user_preference',
        },
      },
    };
  const fallbackModel = constraints.models.find((model) => model.allowed);
  return {
    ...snapshot,
    modelChoices: {
      ...snapshot.modelChoices,
      resolvedForCurrentSession: {
        model: fallbackModel?.modelId ?? null,
        reasoning: fallbackModel?.minReasoning ?? null,
        source: 'provisional_fallback',
      },
    },
  };
}

function isAllowedChoice(
  modelId: string,
  reasoning: HarnessReasoningLevel | null,
  policy: HarnessModelPolicy,
  catalogs: HarnessConfigurationCatalogs,
): boolean {
  const model = policy.models.find((candidate) => candidate.modelId === modelId);
  if (!model?.allowed) return false;
  if (reasoning === null) return true;
  const catalog = catalogs.models.items.find((candidate) => candidate.id === modelId);
  if (!catalog) return false;
  const selected = catalog.reasoningLevels.indexOf(reasoning);
  const minimum = catalog.reasoningLevels.indexOf(model.minReasoning);
  const maximum = catalog.reasoningLevels.indexOf(model.maxReasoning);
  return selected >= minimum && selected <= maximum;
}

function assertUniqueCatalogItems(
  items: readonly string[],
  catalogItems: ReadonlySet<string>,
  label: string,
): void {
  if (new Set(items).size !== items.length) throw new Error(`Selected ${label}s must be unique.`);
  const unknown = items.find((item) => !catalogItems.has(item));
  if (unknown) throw new Error(`Unknown ${label}: ${unknown}`);
}

function requireWorkingCopy(
  snapshot: ConversationHarnessManagementSnapshot,
): NonNullable<ConversationHarnessManagementSnapshot['workingCopy']> {
  if (!snapshot.workingCopy) throw new Error('Start editing before saving or committing.');
  return snapshot.workingCopy;
}

function findVersion(snapshot: ConversationHarnessManagementSnapshot, revision: number) {
  const version = snapshot.versionControl.versions.find(
    (candidate) => candidate.revision === revision,
  );
  if (!version) throw new Error('The selected version is no longer available.');
  return version;
}

function assertDraftRevision(actual: number, expected: number): void {
  if (actual !== expected) throw new Error('The working draft changed. Reload before saving.');
}

function availableRead(
  snapshot: ConversationHarnessManagementSnapshot,
): ConversationHarnessManagementRead {
  return { kind: 'available', snapshot };
}

function unboundRead(): ConversationHarnessManagementRead {
  return {
    kind: 'unbound',
    reason: 'This Agent Session has no harness relationship.',
  };
}

function runtimeSandbox(value: string): HarnessEffectiveConfiguration['runtime']['sandbox'] {
  if (value === 'read_only' || value === 'workspace_write' || value === 'danger_full_access')
    return value;
  throw new Error('Unsupported sandbox.');
}

function reasoningLevel(value: string | null): HarnessReasoningLevel | null {
  if (value === null) return null;
  if (value === 'low' || value === 'medium' || value === 'high' || value === 'xhigh') return value;
  throw new Error('Unsupported reasoning level.');
}

function frontmatterValue(document: string, key: string): string | null {
  const match = document.match(new RegExp(`^${key}:\\s*(.+)$`, 'm'));
  return match?.[1]?.trim() ?? null;
}

function humanize(value: string): string {
  return value.replaceAll('_', ' ');
}

function createRecordedHarnessInspectorSession(): AgentSessionDetailsDto {
  const invocationId = 'recorded-harness-inspector-turn';
  const response: AgentRuntimeEventDto = {
    id: 'recorded-harness-inspector-response',
    invocationId,
    sequence: 1,
    source: 'stdout',
    rawPayload: { recorded: true },
    normalized: {
      kind: 'agent_message',
      text: 'The planning context is ready for review. No structured proposal has been changed.',
      externalContextId: null,
      usage: null,
      details: { role: 'final' },
    },
    recordedAt,
  };
  return {
    session: {
      id: recordedHarnessInspectorSessionId,
      title: 'Epic Plan Builder exploration',
      availability: 'available',
      runtimeBinding: {
        externalContextId: 'recorded-harness-inspector-thread',
        runtimeVersion: 'recorded',
      },
      workingDirectory: null,
      requestedOptions: { model: null, sandbox: 'read_only' },
      agentIdentity: recordedAgentIdentity,
      createdAt: recordedAt,
      updatedAt: recordedAt,
    },
    invocations: [
      {
        invocation: {
          id: invocationId,
          sessionId: recordedHarnessInspectorSessionId,
          submittedText: 'Review the emerging Epic boundaries before building the proposal.',
          inputProvenance: 'user',
          status: 'completed',
          requestedOptions: { model: null, sandbox: 'read_only' },
          effectiveOptions: { model: null, sandbox: 'read_only' },
          startedAt: recordedAt,
          completedAt: recordedAt,
          exitCode: 0,
          signal: null,
          runtimeError: null,
          diagnostics: [],
          createdAt: recordedAt,
          updatedAt: recordedAt,
        },
        events: [response],
      },
    ],
  };
}
