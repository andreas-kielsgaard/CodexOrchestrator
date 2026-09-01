export interface ReferenceIdentityDto {
  readonly namespace: string;
  readonly kind: string;
  readonly id: string;
}

export interface SessionLogicalAddressDto {
  readonly scope: ReferenceIdentityDto;
  readonly subject: ReferenceIdentityDto;
}

export type SessionEventTriggerBindingDto =
  | { readonly kind: 'user_request' }
  | {
      readonly kind: 'invocation_completed';
      readonly sourceAddress: SessionLogicalAddressDto | null;
    }
  | {
      readonly kind: 'mcp_call';
      readonly server: ReferenceIdentityDto;
      readonly tool: ReferenceIdentityDto;
    }
  | { readonly kind: 'application_event'; readonly eventKind: ReferenceIdentityDto }
  | {
      readonly kind: 'event_group_completed';
      readonly sourceDefinition: ReferenceIdentityDto;
    };

export type SessionEventTriggerDto =
  | { readonly kind: 'user_request'; readonly request: ReferenceIdentityDto }
  | {
      readonly kind: 'invocation_completed';
      readonly session: ReferenceIdentityDto;
      readonly invocation: ReferenceIdentityDto;
    }
  | {
      readonly kind: 'mcp_call';
      readonly call: ReferenceIdentityDto;
      readonly server: ReferenceIdentityDto;
      readonly tool: ReferenceIdentityDto;
    }
  | { readonly kind: 'application_event'; readonly event: ReferenceIdentityDto }
  | { readonly kind: 'event_group_completed'; readonly eventGroup: ReferenceIdentityDto };

export type SessionEventSourceDto =
  | { readonly kind: 'user_request'; readonly request: ReferenceIdentityDto }
  | { readonly kind: 'application'; readonly component: ReferenceIdentityDto }
  | {
      readonly kind: 'session_invocation';
      readonly session: ReferenceIdentityDto;
      readonly invocation: ReferenceIdentityDto;
    }
  | {
      readonly kind: 'mcp_call';
      readonly call: ReferenceIdentityDto;
      readonly server: ReferenceIdentityDto;
      readonly tool: ReferenceIdentityDto;
    }
  | { readonly kind: 'application_event'; readonly event: ReferenceIdentityDto }
  | { readonly kind: 'event_group'; readonly eventGroup: ReferenceIdentityDto }
  | { readonly kind: 'external_reference'; readonly reference: ReferenceIdentityDto };

export type PromptSourceDefinitionDto =
  | { readonly kind: 'literal'; readonly text: string }
  | { readonly kind: 'user_request_text' }
  | { readonly kind: 'invocation_output' }
  | { readonly kind: 'mcp_argument'; readonly name: string }
  | { readonly kind: 'application_event_field'; readonly field: string }
  | { readonly kind: 'referenced_content'; readonly reference: ReferenceIdentityDto };

export type PromptSourceDto =
  | { readonly kind: 'literal'; readonly text: string }
  | {
      readonly kind: 'user_request_text';
      readonly request: ReferenceIdentityDto;
      readonly text: string;
    }
  | {
      readonly kind: 'invocation_output';
      readonly invocation: ReferenceIdentityDto;
      readonly text: string;
    }
  | {
      readonly kind: 'mcp_argument';
      readonly call: ReferenceIdentityDto;
      readonly name: string;
      readonly text: string;
    }
  | {
      readonly kind: 'application_event_field';
      readonly event: ReferenceIdentityDto;
      readonly field: string;
      readonly text: string;
    }
  | {
      readonly kind: 'referenced_content';
      readonly reference: ReferenceIdentityDto;
      readonly text: string;
    };

export type SessionTargetDto =
  | { readonly kind: 'exact'; readonly session: ReferenceIdentityDto }
  | { readonly kind: 'logical'; readonly address: SessionLogicalAddressDto };

export type TargetCardinalityDto = 'first' | 'all';
export type TargetOrderingDto = 'newest' | 'last_addressed';
export type RunningFilterDto = 'any' | 'running_only' | 'not_running';
export type MissingTargetPolicyDto = 'create' | 'fail' | 'noop';

export interface SessionCreationFilterDto {
  readonly event: ReferenceIdentityDto | null;
  readonly session: ReferenceIdentityDto | null;
}

export interface TargetSelectionDto {
  readonly target: SessionTargetDto;
  readonly cardinality: TargetCardinalityDto;
  readonly ordering: TargetOrderingDto;
  readonly running: RunningFilterDto;
  readonly createdBy: SessionCreationFilterDto | null;
  readonly missing: MissingTargetPolicyDto;
}

export interface SessionCreationConfigurationDto {
  readonly contract: ReferenceIdentityDto;
  readonly payload: unknown;
  readonly assignedIdentity: ReferenceIdentityDto | null;
}

export interface SessionEventDefinitionDto {
  readonly definitionRef: ReferenceIdentityDto;
  readonly trigger: SessionEventTriggerBindingDto;
  readonly target: TargetSelectionDto;
  readonly promptSources: readonly PromptSourceDefinitionDto[];
  readonly createdSessionPromptSources: readonly PromptSourceDefinitionDto[];
  readonly creationConfiguration: SessionCreationConfigurationDto | null;
}

export type EventGroupOutcomeDto =
  'delivered' | 'partially_delivered' | 'delivery_failed' | 'no_target' | 'noop';

export interface EventGroupRecordDto {
  readonly eventGroupId: ReferenceIdentityDto;
  readonly definitionRef: ReferenceIdentityDto;
  readonly trigger: SessionEventTriggerDto;
  readonly source: SessionEventSourceDto;
  readonly promptSources: readonly PromptSourceDto[];
  readonly createdSessionPromptSources: readonly PromptSourceDto[];
  readonly targetSelection: TargetSelectionDto;
  readonly resolvedSessions: readonly ReferenceIdentityDto[];
  readonly createdSession: ReferenceIdentityDto | null;
  readonly outcome: EventGroupOutcomeDto;
  readonly deliveryCount: number;
}

export type DeliveryOutcomeDto =
  | { readonly kind: 'dispatched'; readonly invocation: ReferenceIdentityDto }
  | { readonly kind: 'failed'; readonly message: string };

export interface EventDeliveryRecordDto {
  readonly deliveryId: ReferenceIdentityDto;
  readonly eventGroupId: ReferenceIdentityDto;
  readonly ordinal: number;
  readonly targetSession: ReferenceIdentityDto;
  readonly logicalAddress: SessionLogicalAddressDto | null;
  readonly targetCreated: boolean;
  readonly promptContributions: readonly PromptSourceDto[];
  readonly includedCreatedSessionContributions: readonly PromptSourceDto[];
  readonly addressedSequence: number | null;
  readonly addressingError: string | null;
  readonly outcome: DeliveryOutcomeDto;
}

export interface SessionEventResultDto {
  readonly group: EventGroupRecordDto;
  readonly deliveries: readonly EventDeliveryRecordDto[];
}

export interface SessionEventQueryClient {
  loadEventGroup(eventGroupId: ReferenceIdentityDto): Promise<EventGroupRecordDto | null>;
  loadRecordedEvent(eventGroupId: ReferenceIdentityDto): Promise<SessionEventResultDto | null>;
  listDeliveriesForGroup(
    eventGroupId: ReferenceIdentityDto,
  ): Promise<readonly EventDeliveryRecordDto[]>;
  listDeliveriesForSession(
    session: ReferenceIdentityDto,
  ): Promise<readonly EventDeliveryRecordDto[]>;
}
