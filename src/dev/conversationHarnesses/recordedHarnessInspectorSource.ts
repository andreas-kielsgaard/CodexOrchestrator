import type { AgentRuntimeEventDto, AgentSessionDetailsDto } from '../../application/agentSessions';
import type {
  ConversationHarnessInspectorSnapshot,
  ConversationHarnessInspectorSource,
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

export const recordedHarnessInspectorSessionDetails: AgentSessionDetailsDto =
  createRecordedHarnessInspectorSession();

export const recordedHarnessInspectorSource: ConversationHarnessInspectorSource = {
  async load({ sessionId }) {
    if (sessionId !== recordedHarnessInspectorSessionId)
      return {
        kind: 'unavailable',
        reason: 'This recorded Agent Session has no product harness configuration.',
      };
    try {
      return { kind: 'available', snapshot: buildSnapshot(sessionId) };
    } catch {
      return {
        kind: 'unavailable',
        reason: 'The checked-in Conversation Harness catalog is invalid or unavailable.',
      };
    }
  },
};

function buildSnapshot(sessionId: string): ConversationHarnessInspectorSnapshot {
  if (catalog.schemaVersion !== 2) throw new Error('unsupported schema');
  const profile = catalog.harnesses.find((candidate) => candidate.key === profileKey);
  if (!profile || profile.version < 1 || !profile.context.trim())
    throw new Error('invalid profile');
  if (profile.lifecycle.contextDelivery !== 'first_query') throw new Error('invalid delivery');
  if (profile.runtime.approvalPolicy !== 'never') throw new Error('invalid approval policy');
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
    throw new Error('invalid declarative settings');

  return {
    sessionId,
    profile: {
      key: profile.key,
      title: 'Epic Plan Builder',
      version: profile.version,
      catalogSchemaVersion: catalog.schemaVersion,
    },
    provenance: {
      kind: 'recorded_adapter',
      source: 'src-tauri/src/orchestration/conversation_harness_catalog.json',
      summary:
        'Parsed from the checked-in catalog by a recorded development adapter. No live product query or durable session binding was performed.',
    },
    validation: {
      status: 'unverified',
      checks: [
        {
          label: 'Catalog schema',
          status: 'passed',
          detail: 'Schema v2 and the selected profile satisfy the recorded adapter shape checks.',
        },
        {
          label: 'Runtime policy shape',
          status: 'passed',
          detail: 'Sandbox, approval policy, skills, and MCP allow-list are structurally valid.',
        },
        {
          label: 'Repository skill source',
          status: 'unverified',
          detail:
            'The canonical path is recorded; this browser adapter does not prove file discovery.',
        },
        {
          label: 'Delivered session context',
          status: 'unverified',
          detail: 'Delivery is fixture state, not a durable product observation from this session.',
        },
      ],
    },
    promptContext: {
      content: profile.context,
      delivery: 'first_query',
      state: {
        scope: 'delivered_to_session',
        editability: 'immutable',
        reason:
          'The recorded adapter marks this prefix delivered before the first user query. Editing cannot change context already received by the session.',
      },
    },
    skills: {
      items: profile.skillGuidance.map((skill) => ({
        name: skill.canonicalName,
        path: skill.canonicalPath,
        purpose: skill.purpose,
        useWhen: skill.useWhen,
      })),
      state: {
        scope: 'future_invocation',
        editability: 'read_only',
        reason:
          'Skill guidance could be revised for a future invocation. Catalog presence does not prove that Codex discovered or selected a skill.',
      },
    },
    mcp: {
      required: profile.mcp.required,
      tools: profile.mcp.enabledTools,
      state: {
        scope: 'future_invocation',
        editability: 'read_only',
        reason:
          'A specialized product service assembles this allow-list before launch. The current invocation is not mutated.',
      },
    },
    runtime: {
      model: profile.runtime.model,
      reasoningEffort: profile.runtime.reasoningEffort,
      sandbox,
      approvalPolicy: 'never',
      authorityBoundary:
        'Sandbox limits runtime access; product effects still require the MCP allow-list and server-side semantic authorization. A profile alone grants no effect.',
      state: {
        scope: 'future_invocation',
        editability: 'read_only',
        reason:
          'Model, reasoning, sandbox, and approval settings are assembled before launch. Inherited values remain explicit.',
      },
    },
    hooks: {
      items: [
        {
          name: 'Initial prompt delivery',
          status: 'configured',
          detail: 'The managed product service delivers the prefix on the first query only.',
        },
        {
          name: 'Completion criteria',
          status: 'declarative_only',
          detail: `${profile.lifecycle.completionCriteria.join(', ')}. This criterion does not apply a product effect.`,
        },
        {
          name: 'Configuration apply',
          status: 'unsupported',
          detail: 'No application command is connected to persist or activate harness edits.',
        },
      ],
      state: {
        scope: 'application_owned',
        editability: 'unsupported',
        reason:
          'Hooks remain product-owned integration points. This inspector can report them but cannot grant or invoke them.',
      },
    },
    apply: {
      status: 'unsupported',
      reason:
        'This exploration has no persistence command, authorization decision, or runtime mutation transport.',
      safeSemantics: [
        'Validate the complete proposed profile before persistence.',
        'Reject stale revisions instead of overwriting newer configuration.',
        'Create a new version for future invocations; never rewrite delivered context.',
        'Keep sandbox, tools, and hooks within product policy and server-side authority.',
        'Record configuration provenance and the activation result separately.',
      ],
    },
  };
}

function runtimeSandbox(value: string): ConversationHarnessInspectorSnapshot['runtime']['sandbox'] {
  if (value === 'read_only' || value === 'workspace_write' || value === 'danger_full_access')
    return value;
  throw new Error('unsupported sandbox');
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
