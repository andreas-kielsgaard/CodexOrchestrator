export interface ReferenceIdentity {
  readonly namespace: string;
  readonly kind: string;
  readonly id: string;
}

export interface SessionLogicalAddress {
  readonly scope: ReferenceIdentity;
  readonly subject: ReferenceIdentity;
}

export type SessionEventTriggerBinding =
  | { readonly kind: 'user_request' }
  | {
      readonly kind: 'invocation_completed';
      readonly sourceAddress: SessionLogicalAddress | null;
    }
  | {
      readonly kind: 'mcp_call';
      readonly server: ReferenceIdentity;
      readonly tool: ReferenceIdentity;
    }
  | {
      readonly kind: 'application_event';
      readonly eventKind: ReferenceIdentity;
    }
  | {
      readonly kind: 'event_group_completed';
      readonly sourceDefinition: ReferenceIdentity;
    };

export type PromptSourceDefinition =
  | { readonly kind: 'literal'; readonly text: string }
  | { readonly kind: 'user_request_text' }
  | { readonly kind: 'invocation_output' }
  | { readonly kind: 'mcp_argument'; readonly name: string }
  | { readonly kind: 'application_event_field'; readonly field: string }
  | { readonly kind: 'referenced_content'; readonly reference: ReferenceIdentity };

export type SessionTarget =
  | { readonly kind: 'exact'; readonly session: ReferenceIdentity }
  | { readonly kind: 'logical'; readonly address: SessionLogicalAddress };

export type TargetCardinality = 'first' | 'all';
export type TargetOrdering = 'newest' | 'last_addressed';
export type RunningFilter = 'any' | 'running_only' | 'not_running';
export type MissingTargetPolicy = 'create' | 'fail' | 'noop';

export interface SessionCreationFilter {
  readonly event: ReferenceIdentity | null;
  readonly session: ReferenceIdentity | null;
}

export interface TargetSelection {
  readonly target: SessionTarget;
  readonly cardinality: TargetCardinality;
  readonly ordering: TargetOrdering;
  readonly running: RunningFilter;
  readonly createdBy: SessionCreationFilter | null;
  readonly missing: MissingTargetPolicy;
}

export type SessionEventTrigger =
  | { readonly kind: 'user_request'; readonly request: ReferenceIdentity }
  | {
      readonly kind: 'invocation_completed';
      readonly session: ReferenceIdentity;
      readonly invocation: ReferenceIdentity;
    }
  | {
      readonly kind: 'mcp_call';
      readonly call: ReferenceIdentity;
      readonly server: ReferenceIdentity;
      readonly tool: ReferenceIdentity;
    }
  | { readonly kind: 'application_event'; readonly event: ReferenceIdentity }
  | { readonly kind: 'event_group_completed'; readonly eventGroup: ReferenceIdentity };

export type SessionEventSource =
  | { readonly kind: 'user_request'; readonly request: ReferenceIdentity }
  | { readonly kind: 'application'; readonly component: ReferenceIdentity }
  | {
      readonly kind: 'session_invocation';
      readonly session: ReferenceIdentity;
      readonly invocation: ReferenceIdentity;
    }
  | {
      readonly kind: 'mcp_call';
      readonly call: ReferenceIdentity;
      readonly server: ReferenceIdentity;
      readonly tool: ReferenceIdentity;
    }
  | { readonly kind: 'application_event'; readonly event: ReferenceIdentity }
  | { readonly kind: 'event_group'; readonly eventGroup: ReferenceIdentity }
  | { readonly kind: 'external_reference'; readonly reference: ReferenceIdentity };

export type PromptSource =
  | { readonly kind: 'literal'; readonly text: string }
  | {
      readonly kind: 'user_request_text';
      readonly request: ReferenceIdentity;
      readonly text: string;
    }
  | {
      readonly kind: 'invocation_output';
      readonly invocation: ReferenceIdentity;
      readonly text: string;
    }
  | {
      readonly kind: 'mcp_argument';
      readonly call: ReferenceIdentity;
      readonly name: string;
      readonly text: string;
    }
  | {
      readonly kind: 'application_event_field';
      readonly event: ReferenceIdentity;
      readonly field: string;
      readonly text: string;
    }
  | {
      readonly kind: 'referenced_content';
      readonly reference: ReferenceIdentity;
      readonly text: string;
    };

export type EventGroupOutcome =
  'delivered' | 'partially_delivered' | 'delivery_failed' | 'no_target' | 'noop';

export type DeliveryOutcome =
  | { readonly kind: 'dispatched'; readonly invocation: ReferenceIdentity }
  | { readonly kind: 'failed'; readonly message: string };

export interface EventGroupRecord {
  readonly eventGroupId: ReferenceIdentity;
  readonly definitionRef: ReferenceIdentity;
  readonly trigger: SessionEventTrigger;
  readonly source: SessionEventSource;
  readonly promptSources: readonly PromptSource[];
  readonly createdSessionPromptSources: readonly PromptSource[];
  readonly targetSelection: TargetSelection;
  readonly resolvedSessions: readonly ReferenceIdentity[];
  readonly createdSession: ReferenceIdentity | null;
  readonly outcome: EventGroupOutcome;
  readonly deliveryCount: number;
}

export interface EventDeliveryRecord {
  readonly deliveryId: ReferenceIdentity;
  readonly eventGroupId: ReferenceIdentity;
  readonly ordinal: number;
  readonly targetSession: ReferenceIdentity;
  readonly logicalAddress: SessionLogicalAddress | null;
  readonly targetCreated: boolean;
  readonly promptContributions: readonly PromptSource[];
  readonly includedCreatedSessionContributions: readonly PromptSource[];
  readonly addressedSequence: number | null;
  readonly addressingError: string | null;
  readonly outcome: DeliveryOutcome;
}

export function referenceIdentityLabel(reference: ReferenceIdentity): string {
  return `${reference.namespace}:${reference.kind}/${reference.id}`;
}

export function promptSourceText(source: PromptSource): string {
  return source.text;
}
