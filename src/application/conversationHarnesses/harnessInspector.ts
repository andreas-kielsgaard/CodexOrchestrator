import type { AgentIdentityDto, AgentVisualIdentityDto } from '../agentSessions';

export type HarnessSkillPolicy = 'always_applicable' | 'initial_ingestion' | 'available';
export type HarnessToolPolicy = 'every_invocation' | 'initial_invocation' | 'available';
export type HarnessDiscoveryPolicy = 'whitelist' | 'blacklist';
export type HarnessReasoningLevel = 'low' | 'medium' | 'high' | 'xhigh';

export interface HarnessEffectiveConfiguration {
  readonly identity: {
    readonly name: string;
    readonly machineKey: string;
    readonly permittedAgentNames: readonly string[] | null;
    readonly visualIdentity: AgentVisualIdentityDto | null;
  };
  readonly promptPrefix: {
    readonly content: string;
    readonly initialDelivery: 'prepend';
    readonly contextCompressionDelivery: 'deferred';
  };
  readonly skills: {
    readonly availableDiscoveryPolicy: HarnessDiscoveryPolicy;
    readonly items: readonly {
      readonly name: string;
      readonly path: string;
      readonly purpose: string;
      readonly useWhen: string;
      readonly policy: HarnessSkillPolicy;
    }[];
  };
  readonly tools: {
    readonly availableDiscoveryPolicy: HarnessDiscoveryPolicy;
    readonly items: readonly {
      readonly name: string;
      readonly policy: HarnessToolPolicy;
    }[];
    readonly schemaBoundary: string;
  };
  readonly runtime: {
    readonly models: readonly {
      readonly modelId: string;
      readonly allowed: boolean;
      readonly minReasoning: HarnessReasoningLevel;
      readonly maxReasoning: HarnessReasoningLevel;
    }[];
    readonly defaultModel: string | null;
    readonly defaultReasoning: HarnessReasoningLevel | null;
    readonly sandbox: 'read_only' | 'workspace_write' | 'danger_full_access';
    readonly sandboxOptions: readonly ('read_only' | 'workspace_write' | 'danger_full_access')[];
    readonly approvalPolicy: 'never';
    readonly approvalPolicyOptions: readonly 'never'[];
    readonly authoritySummary: string;
  };
  readonly hooks: readonly {
    readonly name: string;
    readonly status: 'exposed' | 'proposed' | 'not_connected';
    readonly detail: string;
  }[];
  readonly updatePolicy:
    | {
        readonly status: 'configured';
        readonly delivery: 'next_prompt';
        readonly avoidDuplicateGuidance: boolean;
        readonly notifyRemovedItems: boolean;
        readonly promptReconstruction: 'deferred';
      }
    | {
        readonly status: 'not_configured';
        readonly reason: string;
      };
}

export interface HarnessConfigurationCatalogs {
  readonly skills: {
    readonly source: 'checked_in_product_catalog' | 'not_connected';
    readonly items: readonly {
      readonly name: string;
      readonly path: string;
      readonly description: string;
    }[];
    readonly reason: string;
  };
  readonly tools: {
    readonly source: 'recorded_harness_tool_catalog' | 'not_connected';
    readonly items: readonly {
      readonly name: string;
      readonly description: string;
    }[];
    readonly reason: string;
  };
  readonly models: {
    readonly source: 'recorded_catalog' | 'not_connected';
    readonly items: readonly {
      readonly id: string;
      readonly label: string;
      readonly reasoningLevels: readonly HarnessReasoningLevel[];
    }[];
    readonly reason: string;
  };
}

export interface ConversationHarnessVersion {
  readonly revision: number;
  readonly configuration: HarnessEffectiveConfiguration;
  readonly activeSessionCount: number;
  readonly committedAt: string;
}

export interface ConversationHarnessManagementSnapshot {
  readonly sessionId: string;
  readonly harnessKey: string;
  readonly agentIdentity: AgentIdentityDto | null;
  readonly catalogs: HarnessConfigurationCatalogs;
  readonly workingCopy: {
    readonly baseRevision: number;
    readonly draftRevision: number;
    readonly dirty: boolean;
    readonly configuration: HarnessEffectiveConfiguration;
  } | null;
  readonly versionControl: {
    readonly support: 'recorded_preview' | 'not_connected';
    readonly pushedRevision: number | null;
    readonly versions: readonly ConversationHarnessVersion[];
    readonly reason: string;
  };
  readonly sessionBinding: {
    readonly state: 'current' | 'behind' | 'queued' | 'untracked';
    readonly appliedRevision: number | null;
    readonly desiredRevision: number | null;
    readonly relevantSessionCount: number | null;
    readonly executingPreviousInvocation: boolean;
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
      readonly kind: 'start_edit';
      readonly baseRevision: number;
    }
  | {
      readonly kind: 'save_working_copy';
      readonly configuration: HarnessEffectiveConfiguration;
    }
  | {
      readonly kind: 'commit';
      readonly expectedDraftRevision: number;
    }
  | {
      readonly kind: 'push';
      readonly revision: number;
    }
  | {
      readonly kind: 'queue_version';
      readonly revision: number;
      readonly scope: 'current_session' | 'all_relevant_sessions';
    };

/** Every view uses this session-keyed boundary; components never select a harness or revision. */
export interface ConversationHarnessManagementSource {
  load(input: { readonly sessionId: string }): Promise<ConversationHarnessManagementRead>;
  dispatch?(input: {
    readonly sessionId: string;
    readonly command: ConversationHarnessManagementCommand;
  }): Promise<ConversationHarnessManagementRead>;
}
