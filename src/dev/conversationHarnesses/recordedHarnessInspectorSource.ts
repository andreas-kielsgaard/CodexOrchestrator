import type { AgentRuntimeEventDto, AgentSessionDetailsDto } from '../../application/agentSessions';
import {
  assignAgentIdentity,
  harnessAgentNamePools,
  harnessVisualIdentities,
} from '../../application/agentSessions';
import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementRead,
  ConversationHarnessManagementSnapshot,
  ConversationHarnessManagementSource,
  HarnessConfigurationCatalogs,
  HarnessEffectiveConfiguration,
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

export function createRecordedHarnessManagementSource(): ConversationHarnessManagementSource {
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
  return {
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
          configuration: currentConfiguration,
          activeSessionCount: 1,
          committedAt: '2026-07-11T14:30:00.000Z',
        },
        {
          revision: profile.version,
          configuration: currentConfiguration,
          activeSessionCount: 0,
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
  };
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
      };
    })
    .filter((skill) => skill.name)
    .sort((left, right) => left.name.localeCompare(right.name));
  return {
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
        'Tool schemas remain runtime-owned. The harness controls whether and when a tool is exposed.',
    },
    runtime: {
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
            configuration: workingCopy.configuration,
            activeSessionCount: 0,
            committedAt: new Date().toISOString(),
          },
        ],
      },
    };
  }
  if (command.kind === 'push') {
    findVersion(snapshot, command.revision);
    return {
      ...snapshot,
      versionControl: {
        ...snapshot.versionControl,
        pushedRevision: command.revision,
      },
      sessionBinding: {
        ...snapshot.sessionBinding,
        state: snapshot.sessionBinding.appliedRevision === command.revision ? 'current' : 'queued',
        desiredRevision:
          snapshot.sessionBinding.appliedRevision === command.revision ? null : command.revision,
        reason:
          snapshot.sessionBinding.appliedRevision === command.revision
            ? 'This Session already uses the pushed version.'
            : 'The pushed version is queued for this Session at its next prompt.',
      },
    };
  }
  findVersion(snapshot, command.revision);
  return {
    ...snapshot,
    sessionBinding: {
      ...snapshot.sessionBinding,
      state: snapshot.sessionBinding.appliedRevision === command.revision ? 'current' : 'queued',
      desiredRevision:
        snapshot.sessionBinding.appliedRevision === command.revision ? null : command.revision,
      reason:
        snapshot.sessionBinding.appliedRevision === command.revision
          ? 'This Session already uses the selected version.'
          : command.scope === 'current_session'
            ? 'The selected version is queued for this Session at its next prompt.'
            : 'The selected version is queued for every relevant Session at its next prompt.',
    },
  };
}

function validateConfiguration(
  configuration: HarnessEffectiveConfiguration,
  catalogs: HarnessConfigurationCatalogs,
): void {
  if (!configuration.identity.name.trim() || !configuration.identity.machineKey.trim())
    throw new Error('Harness identity is incomplete.');
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
  const modelConfigurations = new Map(
    configuration.runtime.models.map((model) => [model.modelId, model]),
  );
  if (modelConfigurations.size !== configuration.runtime.models.length)
    throw new Error('Model options must be unique.');
  for (const model of catalogs.models.items) {
    const configurationModel = modelConfigurations.get(model.id);
    if (!configurationModel) throw new Error(`Model option is missing: ${model.id}`);
    const minimum = model.reasoningLevels.indexOf(configurationModel.minReasoning);
    const maximum = model.reasoningLevels.indexOf(configurationModel.maxReasoning);
    if (minimum < 0 || maximum < minimum)
      throw new Error(`Reasoning range is invalid for ${model.label}.`);
  }
  if (configuration.runtime.defaultModel) {
    const defaultModel = modelConfigurations.get(configuration.runtime.defaultModel);
    if (!defaultModel?.allowed) throw new Error('Default model must be allowed.');
    if (configuration.runtime.defaultReasoning) {
      const catalogModel = catalogs.models.items.find(
        (model) => model.id === configuration.runtime.defaultModel,
      );
      if (!catalogModel) throw new Error('Default model is outside the recorded catalog.');
      const selected = catalogModel.reasoningLevels.indexOf(configuration.runtime.defaultReasoning);
      const minimum = catalogModel.reasoningLevels.indexOf(defaultModel.minReasoning);
      const maximum = catalogModel.reasoningLevels.indexOf(defaultModel.maxReasoning);
      if (selected < minimum || selected > maximum)
        throw new Error('Default reasoning must fit the default model range.');
    }
  } else if (configuration.runtime.defaultReasoning) {
    throw new Error('Choose a default model before choosing default reasoning.');
  }
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
