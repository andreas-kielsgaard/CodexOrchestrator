import { invoke } from '@tauri-apps/api/core';
import type {
  ConversationHarnessInspectorSnapshot,
  ConversationHarnessInspectorSource,
  HarnessInspectorDeliveryStatus,
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
): ConversationHarnessInspectorSource {
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
): ConversationHarnessInspectorSnapshot {
  const delivery = deliverySnapshot(read.delivery);
  const profile = read.profile;
  return {
    sessionId: read.sessionId,
    profile: {
      key: profile.key,
      title: 'Epic Plan Builder',
      version: profile.version,
      catalogSchemaVersion: read.catalogSchemaVersion,
    },
    provenance: {
      kind: 'product_query',
      source: 'Managed Plan Builder application query',
      summary:
        'Loaded through the Epic Plan Builder product boundary from its current validated catalog and durable Agent Session records.',
    },
    validation: {
      status: 'unverified',
      checks: [
        {
          label: 'Catalog schema and profile',
          status: 'passed',
          detail: `The application validated catalog schema v${read.catalogSchemaVersion} and profile v${profile.version}.`,
        },
        {
          label: 'Runtime policy shape',
          status: 'passed',
          detail: 'Sandbox, approval policy, skills, and MCP allow-list passed product validation.',
        },
        {
          label: 'Repository skill source',
          status: 'unverified',
          detail:
            'The catalog records canonical paths; this read does not prove per-invocation Codex discovery or selection.',
        },
        {
          label: 'First-query delivery evidence',
          status: delivery.validationStatus,
          detail: delivery.detail,
        },
      ],
    },
    promptContext: {
      content: profile.context,
      delivery: {
        policy: 'first_query',
        status: delivery.status,
        detail: delivery.detail,
      },
      state: {
        scope: 'profile_configuration',
        editability: 'read_only',
        reason:
          'This is the currently configured profile value. Delivery evidence is reported separately and does not rewrite the session.',
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
          'Skill guidance applies to a future invocation. Catalog presence does not prove Codex discovery or selection.',
      },
    },
    mcp: {
      required: profile.mcp.required,
      tools: profile.mcp.enabledTools,
      state: {
        scope: 'future_invocation',
        editability: 'read_only',
        reason:
          'The managed product service assembles this allow-list before launch. This read does not mutate the invocation.',
      },
    },
    runtime: {
      model: profile.runtime.model,
      reasoningEffort: profile.runtime.reasoningEffort,
      sandbox: profile.runtime.sandbox,
      approvalPolicy: profile.runtime.approvalPolicy,
      authorityBoundary:
        'Sandbox limits runtime access; product effects still require the MCP allow-list and server-side semantic authorization.',
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
          detail: 'The managed product service supplies the prefix on the first query only.',
        },
        {
          name: 'Completion criteria',
          status: 'declarative_only',
          detail: `${profile.lifecycle.completionCriteria.join(', ')}. These criteria do not apply a product effect.`,
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
        'This read-only integration has no persistence command, authorization decision, or runtime mutation transport.',
      safeSemantics: [
        'Validate the complete proposed profile before persistence.',
        'Reject stale revisions instead of overwriting newer configuration.',
        'Create a new version for future invocations; never rewrite delivered context.',
        'Keep sandbox, tools, and hooks within product policy and server-side authority.',
        'Record configuration provenance and activation separately.',
      ],
    },
  };
}

function deliverySnapshot(
  delivery: Extract<ProductHarnessInspection, { kind: 'bound' }>['delivery'],
): {
  readonly status: HarnessInspectorDeliveryStatus;
  readonly validationStatus: 'passed' | 'unverified';
  readonly detail: string;
} {
  if (delivery.status === 'delivered')
    return {
      status: 'delivered',
      validationStatus: 'passed',
      detail: `Durable launch acceptance exists for first-query invocation ${delivery.invocationId}. The acceptance fact does not retain the exact prompt bytes.`,
    };
  if (delivery.status === 'not_delivered')
    return {
      status: 'not_delivered',
      validationStatus: 'passed',
      detail:
        delivery.reason === 'no_first_query'
          ? 'No first query is durably recorded for this bound Agent Session.'
          : 'The first query failed before the runtime accepted its launch.',
    };
  return {
    status: 'not_evidenced',
    validationStatus: 'unverified',
    detail:
      delivery.reason === 'binding_postdates_first_query'
        ? 'The durable Plan Builder binding postdates the first query, so this query does not attribute that launch to the harness.'
        : `First-query invocation ${delivery.invocationId} has no durable launch-acceptance fact. A started or terminal invocation alone is not delivery evidence.`,
  };
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
