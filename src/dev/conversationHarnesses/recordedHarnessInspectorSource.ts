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
  HarnessEffectiveConfiguration,
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

  return {
    sessionId,
    harnessKey: profile.key,
    agentIdentity: recordedAgentIdentity,
    catalogRevision: profile.version,
    workingCopy: {
      baseRevision: profile.version,
      draftRevision: 1,
      state: 'clean',
      configuration: buildConfiguration(profile, sandbox),
    },
    versionControl: {
      support: 'recorded_preview',
      committedRevision: profile.version,
      activeRevision: profile.version,
      reason: 'Preview actions are kept only while this app remains open.',
    },
    sessionBinding: {
      state: 'update_available',
      appliedRevision: recordedSessionAppliedRevision,
      desiredRevision: profile.version,
      updateStrategy: null,
      relevantSessionCount: 1,
      reason: 'This recorded session is one version behind the active preview.',
    },
  };
}

function buildConfiguration(
  profile: RecordedProfile,
  sandbox: HarnessEffectiveConfiguration['runtime']['sandbox'],
): HarnessEffectiveConfiguration {
  const availableModels = ['gpt-5.6-terra', 'gpt-5.6-sol'];
  const availableReasoningEfforts = ['low', 'medium', 'high', 'xhigh'];
  return {
    identity: {
      name: 'Epic Plan Builder',
      machineKey: profile.key,
      role: 'Epic Plan Builder',
      permittedAgentNames: harnessAgentNamePools.epic_plan_builder,
      visualIdentity: harnessVisualIdentities.epic_plan_builder,
    },
    promptPrefix: {
      content: profile.context,
      initialDelivery: 'prepend',
      contextCompressionDelivery: 'deferred',
    },
    skills: {
      discoveryPolicy: 'whitelist',
      items: profile.skillGuidance.map((skill) => ({
        name: skill.canonicalName,
        path: skill.canonicalPath,
        purpose: skill.purpose,
        useWhen: skill.useWhen,
        policy: 'available',
      })),
    },
    tools: {
      discoveryPolicy: 'whitelist',
      items: profile.mcp.enabledTools.map((name) => ({
        name,
        exposed: true,
        guidancePolicy: 'none',
      })),
      schemaBoundary:
        'Tool schemas come from the runtime. The harness controls exposure and guidance timing, not schema text.',
    },
    runtime: {
      allowInheritedModel: profile.runtime.model === null,
      availableModels,
      allowedModels: profile.runtime.model ? [profile.runtime.model] : availableModels,
      allowInheritedReasoning: profile.runtime.reasoningEffort === null,
      availableReasoningEfforts,
      allowedReasoningEfforts: profile.runtime.reasoningEffort
        ? [profile.runtime.reasoningEffort]
        : ['medium', 'high', 'xhigh'],
      sandbox,
      sandboxOptions: ['read_only', 'workspace_write', 'danger_full_access'],
      approvalPolicy: 'never',
      approvalPolicyOptions: ['never'],
      authoritySummary:
        'Sandbox and approval settings limit runtime access. Application actions still require product authorization.',
    },
    hooks: profile.lifecycle.completionCriteria.map((criterion) => ({
      name: humanize(criterion),
      status: 'exposed',
      detail: `Application hook exposed as ${criterion}.`,
    })),
    updatePolicy: {
      status: 'configured',
      defaultStrategy: 'next_prompt',
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
  if (command.kind === 'save_working_copy') {
    assertDraftRevision(snapshot, command.expectedDraftRevision);
    return {
      ...snapshot,
      workingCopy: {
        ...snapshot.workingCopy,
        draftRevision: snapshot.workingCopy.draftRevision + 1,
        state: 'uncommitted',
        configuration: command.configuration,
      },
    };
  }
  if (command.kind === 'commit') {
    assertDraftRevision(snapshot, command.expectedDraftRevision);
    if (snapshot.workingCopy.state !== 'uncommitted') return snapshot;
    const committedRevision =
      Math.max(
        snapshot.versionControl.committedRevision ?? snapshot.workingCopy.baseRevision,
        snapshot.versionControl.activeRevision ?? snapshot.workingCopy.baseRevision,
      ) + 1;
    return {
      ...snapshot,
      workingCopy: {
        ...snapshot.workingCopy,
        baseRevision: committedRevision,
        draftRevision: snapshot.workingCopy.draftRevision + 1,
        state: 'committed_not_active',
      },
      versionControl: {
        ...snapshot.versionControl,
        committedRevision,
      },
    };
  }
  if (command.kind === 'push') {
    if (snapshot.versionControl.committedRevision !== command.expectedCommittedRevision)
      throw new Error('The committed version changed. Reload before pushing.');
    return {
      ...snapshot,
      workingCopy: {
        ...snapshot.workingCopy,
        state: 'clean',
      },
      versionControl: {
        ...snapshot.versionControl,
        activeRevision: command.expectedCommittedRevision,
      },
      sessionBinding: {
        ...snapshot.sessionBinding,
        state:
          snapshot.sessionBinding.appliedRevision === command.expectedCommittedRevision
            ? 'current'
            : 'update_available',
        desiredRevision: command.expectedCommittedRevision,
        updateStrategy: null,
      },
    };
  }
  if (snapshot.versionControl.activeRevision !== command.expectedActiveRevision)
    throw new Error('The active version changed. Reload before updating sessions.');
  return {
    ...snapshot,
    sessionBinding: {
      ...snapshot.sessionBinding,
      state: 'queued',
      desiredRevision: command.expectedActiveRevision,
      updateStrategy: command.strategy,
      reason:
        command.scope === 'current_session'
          ? 'The update choice is recorded for this session in the preview.'
          : 'The update choice is recorded for every relevant preview session.',
    },
  };
}

function assertDraftRevision(
  snapshot: ConversationHarnessManagementSnapshot,
  expectedDraftRevision: number,
) {
  if (snapshot.workingCopy.draftRevision !== expectedDraftRevision)
    throw new Error('The working copy changed. Reload before saving.');
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
