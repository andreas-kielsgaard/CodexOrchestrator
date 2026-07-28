import { invoke } from '@tauri-apps/api/core';
import type {
  ConversationHarnessManagementSnapshot,
  ConversationHarnessManagementSource,
  HarnessEffectiveConfiguration,
} from '../../application/conversationHarnesses';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface ProductProfile {
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
    readonly sandbox: 'read_only' | 'workspace_write' | 'danger_full_access';
    readonly approvalPolicy: 'never';
  };
  readonly mcp: {
    readonly required: boolean;
    readonly enabledTools: readonly string[];
  };
  readonly lifecycle: {
    readonly contextDelivery: 'first_query';
    readonly completionCriteria: readonly string[];
  };
}

type ProductHarnessInspection =
  | {
      readonly kind: 'bound';
      readonly sessionId: string;
      readonly catalogSchemaVersion: number;
      readonly profile: ProductProfile;
      readonly delivery:
        | { readonly status: 'delivered'; readonly invocationId: string }
        | {
            readonly status: 'not_delivered';
            readonly reason: 'no_first_query' | 'launch_rejected';
          }
        | {
            readonly status: 'not_evidenced';
            readonly invocationId: string;
            readonly reason: 'binding_postdates_first_query' | 'launch_acceptance_missing';
          };
    }
  | { readonly kind: 'unbound'; readonly sessionId: string }
  | { readonly kind: 'invalid_catalog'; readonly sessionId: string };

export function createTauriConversationHarnessInspectorSource(
  invokeCommand: TauriInvoke = invoke,
): ConversationHarnessManagementSource {
  return {
    async load({ sessionId }) {
      try {
        const read = decodeInspection(
          await invokeCommand<unknown>('load_managed_plan_builder_harness_inspection', {
            input: { sessionId },
          }),
        );
        if (read.kind === 'unbound')
          return {
            kind: 'unbound',
            reason: 'This Agent Session is not bound to the Epic Plan Builder product context.',
          };
        if (read.kind === 'invalid_catalog')
          return {
            kind: 'invalid_catalog',
            reason: 'The product Conversation Harness catalog is invalid or unavailable.',
          };
        return { kind: 'available', snapshot: buildSnapshot(read) };
      } catch {
        return {
          kind: 'unavailable',
          reason: 'The application-owned Conversation Harness query is unavailable.',
        };
      }
    },
  };
}

function buildSnapshot(
  read: Extract<ProductHarnessInspection, { readonly kind: 'bound' }>,
): ConversationHarnessManagementSnapshot {
  const profile = read.profile;
  return {
    sessionId: read.sessionId,
    harnessKey: profile.key,
    agentIdentity: null,
    catalogRevision: profile.version,
    workingCopy: {
      baseRevision: profile.version,
      draftRevision: 0,
      state: 'clean',
      configuration: buildConfiguration(profile),
    },
    versionControl: {
      support: 'not_connected',
      committedRevision: null,
      activeRevision: null,
      reason: 'Version history and activation are not connected to the product yet.',
    },
    sessionBinding: {
      state: 'untracked',
      appliedRevision: null,
      desiredRevision: null,
      updateStrategy: null,
      relevantSessionCount: null,
      reason: 'This session association does not yet record an applied or desired harness version.',
    },
  };
}

function buildConfiguration(profile: ProductProfile): HarnessEffectiveConfiguration {
  return {
    identity: {
      name: 'Epic Plan Builder',
      machineKey: profile.key,
      role: 'Epic Plan Builder',
      permittedAgentNames: null,
      visualIdentity: null,
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
      availableModels: profile.runtime.model ? [profile.runtime.model] : [],
      allowedModels: profile.runtime.model ? [profile.runtime.model] : [],
      allowInheritedReasoning: profile.runtime.reasoningEffort === null,
      availableReasoningEfforts: profile.runtime.reasoningEffort
        ? [profile.runtime.reasoningEffort]
        : [],
      allowedReasoningEfforts: profile.runtime.reasoningEffort
        ? [profile.runtime.reasoningEffort]
        : [],
      sandbox: profile.runtime.sandbox,
      sandboxOptions: [profile.runtime.sandbox],
      approvalPolicy: profile.runtime.approvalPolicy,
      approvalPolicyOptions: [profile.runtime.approvalPolicy],
      authoritySummary:
        'Sandbox and approval settings limit runtime access. Application actions still require product authorization.',
    },
    hooks: profile.lifecycle.completionCriteria.map((criterion) => ({
      name: humanize(criterion),
      status: 'exposed',
      detail: `Application hook exposed as ${criterion}.`,
    })),
    updatePolicy: {
      status: 'not_configured',
      reason: 'The compiled catalog does not yet contain an atomic session-update policy.',
    },
  };
}

function humanize(value: string): string {
  return value.replaceAll('_', ' ');
}

function decodeInspection(value: unknown): ProductHarnessInspection {
  if (!isRecord(value) || !isString(value.kind) || !isString(value.sessionId))
    throw new Error('invalid harness inspection');
  if (value.kind === 'unbound' || value.kind === 'invalid_catalog')
    return value as ProductHarnessInspection;
  if (
    value.kind !== 'bound' ||
    !Number.isInteger(value.catalogSchemaVersion) ||
    !isProfile(value.profile) ||
    !isDelivery(value.delivery)
  )
    throw new Error('invalid bound harness inspection');
  return value as ProductHarnessInspection;
}

function isProfile(value: unknown): value is ProductProfile {
  if (!isRecord(value) || !isString(value.key) || !Number.isInteger(value.version)) return false;
  if (!isString(value.context) || !Array.isArray(value.skillGuidance)) return false;
  if (!value.skillGuidance.every(isSkillGuidance)) return false;
  if (!isRecord(value.runtime) || !isRecord(value.mcp) || !isRecord(value.lifecycle)) return false;
  return (
    (value.runtime.model === null || isString(value.runtime.model)) &&
    (value.runtime.reasoningEffort === null || isString(value.runtime.reasoningEffort)) &&
    ['read_only', 'workspace_write', 'danger_full_access'].includes(
      String(value.runtime.sandbox),
    ) &&
    value.runtime.approvalPolicy === 'never' &&
    typeof value.mcp.required === 'boolean' &&
    isStringArray(value.mcp.enabledTools) &&
    value.lifecycle.contextDelivery === 'first_query' &&
    isStringArray(value.lifecycle.completionCriteria)
  );
}

function isSkillGuidance(value: unknown): boolean {
  return (
    isRecord(value) &&
    isString(value.canonicalName) &&
    isString(value.canonicalPath) &&
    isString(value.purpose) &&
    isString(value.useWhen)
  );
}

function isDelivery(value: unknown): boolean {
  if (!isRecord(value) || !isString(value.status)) return false;
  if (value.status === 'delivered') return isString(value.invocationId);
  if (value.status === 'not_delivered')
    return value.reason === 'no_first_query' || value.reason === 'launch_rejected';
  return (
    value.status === 'not_evidenced' &&
    isString(value.invocationId) &&
    (value.reason === 'binding_postdates_first_query' ||
      value.reason === 'launch_acceptance_missing')
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function isStringArray(value: unknown): value is readonly string[] {
  return Array.isArray(value) && value.every(isString);
}
