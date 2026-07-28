import type { AgentIdentityDto, AgentVisualIdentityDto } from '../agentSessions';

export type HarnessSkillPolicy = 'always_applicable' | 'initial_ingestion' | 'available';
export type HarnessDiscoveryPolicy = 'whitelist' | 'blacklist';
export type HarnessToolGuidancePolicy = 'none' | 'initial_ingestion' | 'always_applicable';
export type HarnessUpdateStrategy = 'next_prompt' | 'interrupt';

export interface HarnessEffectiveConfiguration {
  readonly identity: {
    readonly name: string;
    readonly machineKey: string;
    readonly role: string;
    readonly permittedAgentNames: readonly string[] | null;
    readonly visualIdentity: AgentVisualIdentityDto | null;
  };
  readonly promptPrefix: {
    readonly content: string;
    readonly initialDelivery: 'prepend';
    readonly contextCompressionDelivery: 'deferred';
  };
  readonly skills: {
    readonly discoveryPolicy: HarnessDiscoveryPolicy;
    readonly items: readonly {
      readonly name: string;
      readonly path: string;
      readonly purpose: string;
      readonly useWhen: string;
      readonly policy: HarnessSkillPolicy;
    }[];
  };
  readonly tools: {
    readonly discoveryPolicy: HarnessDiscoveryPolicy;
    readonly items: readonly {
      readonly name: string;
      readonly exposed: boolean;
      readonly guidancePolicy: HarnessToolGuidancePolicy;
    }[];
    readonly schemaBoundary: string;
  };
  readonly runtime: {
    readonly allowInheritedModel: boolean;
    readonly availableModels: readonly string[];
    readonly allowedModels: readonly string[];
    readonly allowInheritedReasoning: boolean;
    readonly availableReasoningEfforts: readonly string[];
    readonly allowedReasoningEfforts: readonly string[];
    readonly sandbox: 'read_only' | 'workspace_write' | 'danger_full_access';
    readonly sandboxOptions: readonly ('read_only' | 'workspace_write' | 'danger_full_access')[];
    readonly approvalPolicy: 'never';
    readonly approvalPolicyOptions: readonly 'never'[];
    readonly authoritySummary: string;
  };
  readonly hooks: readonly {
    readonly name: string;
    readonly status: 'exposed' | 'not_connected';
    readonly detail: string;
  }[];
  readonly updatePolicy:
    | {
        readonly status: 'configured';
        readonly defaultStrategy: HarnessUpdateStrategy;
        readonly avoidDuplicateGuidance: boolean;
        readonly notifyRemovedItems: boolean;
        readonly promptReconstruction: 'deferred';
      }
    | {
        readonly status: 'not_configured';
        readonly reason: string;
      };
}

export interface ConversationHarnessManagementSnapshot {
  readonly sessionId: string;
  readonly harnessKey: string;
  readonly agentIdentity: AgentIdentityDto | null;
  readonly catalogRevision: number;
  readonly workingCopy: {
    readonly baseRevision: number;
    readonly draftRevision: number;
    readonly state: 'clean' | 'uncommitted' | 'committed_not_active';
    readonly configuration: HarnessEffectiveConfiguration;
  };
  readonly versionControl: {
    readonly support: 'recorded_preview' | 'not_connected';
    readonly committedRevision: number | null;
    readonly activeRevision: number | null;
    readonly reason: string;
  };
  readonly sessionBinding: {
    readonly state: 'current' | 'update_available' | 'queued' | 'untracked';
    readonly appliedRevision: number | null;
    readonly desiredRevision: number | null;
    readonly updateStrategy: HarnessUpdateStrategy | null;
    readonly relevantSessionCount: number | null;
    readonly reason: string;
  };
}

export type ConversationHarnessManagementRead =
  | {
      readonly kind: 'available';
      readonly snapshot: ConversationHarnessManagementSnapshot;
    }
  | {
      readonly kind: 'unavailable';
      readonly reason: string;
    }
  | {
      readonly kind: 'invalid_catalog';
      readonly reason: string;
    }
  | {
      readonly kind: 'unbound';
      readonly reason: string;
    };

export type ConversationHarnessManagementCommand =
  | {
      readonly kind: 'save_working_copy';
      readonly expectedDraftRevision: number;
      readonly configuration: HarnessEffectiveConfiguration;
    }
  | {
      readonly kind: 'commit';
      readonly expectedDraftRevision: number;
    }
  | {
      readonly kind: 'push';
      readonly expectedCommittedRevision: number;
    }
  | {
      readonly kind: 'request_session_update';
      readonly expectedActiveRevision: number;
      readonly scope: 'current_session' | 'all_relevant_sessions';
      readonly strategy: HarnessUpdateStrategy;
    };

/** Every view uses this session-keyed boundary; components never select a harness or revision. */
export interface ConversationHarnessManagementSource {
  load(input: { readonly sessionId: string }): Promise<ConversationHarnessManagementRead>;
  dispatch?(input: {
    readonly sessionId: string;
    readonly command: ConversationHarnessManagementCommand;
  }): Promise<ConversationHarnessManagementRead>;
}
