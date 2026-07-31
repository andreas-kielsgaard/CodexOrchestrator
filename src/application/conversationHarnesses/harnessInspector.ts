import type { AgentIdentity } from '../agentSessions';

/** Recorded/configuration visual choice; Session identity remains injected by its owner. */
export interface HarnessVisualIdentity {
  readonly token: string;
  readonly accent: string;
}

export type HarnessSkillPolicy = 'always_applicable' | 'initial_ingestion' | 'available';
export type HarnessToolPolicy = 'every_invocation' | 'initial_invocation' | 'available';
export type HarnessDiscoveryPolicy = 'whitelist' | 'blacklist';
export type HarnessReasoningLevel = 'low' | 'medium' | 'high' | 'xhigh';

export interface HarnessModelPolicy {
  readonly models: readonly {
    readonly modelId: string;
    readonly allowed: boolean;
    readonly minReasoning: HarnessReasoningLevel;
    readonly maxReasoning: HarnessReasoningLevel;
  }[];
  readonly defaultModel: string | null;
  readonly defaultReasoning: HarnessReasoningLevel | null;
}

export interface HarnessEffectiveConfiguration {
  readonly identity: {
    readonly name: string;
    readonly machineKey: string;
    readonly permittedAgentNames: readonly string[] | null;
    readonly visualIdentity: HarnessVisualIdentity | null;
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
    readonly modelPolicyMode: 'revision_owned' | 'delegated_shared';
    readonly models: HarnessModelPolicy['models'];
    readonly defaultModel: HarnessModelPolicy['defaultModel'];
    readonly defaultReasoning: HarnessModelPolicy['defaultReasoning'];
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
  readonly agentNames: {
    readonly source: 'product_default_pool' | 'not_connected';
    readonly items: readonly string[];
    readonly reason: string;
  };
  readonly agentVisualIdentities: {
    readonly source: 'product_visual_catalog' | 'not_connected';
    readonly items: readonly {
      readonly identity: HarnessVisualIdentity;
      readonly label: string;
    }[];
    readonly reason: string;
  };
  readonly skills: {
    readonly source: 'checked_in_product_catalog' | 'not_connected';
    readonly items: readonly {
      readonly name: string;
      readonly path: string;
      readonly description: string;
      readonly text: string | null;
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
  readonly label: string;
  readonly status: 'pushed' | 'committed' | 'inspected';
  readonly configuration: HarnessEffectiveConfiguration;
  readonly activeSessionCount: number;
  readonly queuedSessionCount: number;
  readonly committedAt: string;
}

export interface ConversationHarnessManagementSnapshot {
  readonly sessionId: string;
  readonly harnessKey: string;
  readonly agentIdentity: AgentIdentity | null;
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
  readonly modelChoices: {
    readonly delegatedPolicies: readonly {
      readonly revision: number;
      readonly policy: HarnessModelPolicy;
      readonly dirty: boolean;
      readonly updatedAt: string;
    }[];
    readonly sessionOverride: {
      readonly model: string;
      readonly reasoning: HarnessReasoningLevel | null;
    } | null;
    readonly userPreference: {
      readonly support: 'recorded_preference_register' | 'not_connected';
      readonly lastUsedModel: string | null;
      readonly lastUsedReasoning: HarnessReasoningLevel | null;
      readonly reason: string;
    };
    readonly resolvedForCurrentSession: {
      readonly model: string | null;
      readonly reasoning: HarnessReasoningLevel | null;
      readonly source:
        | 'harness_revision'
        | 'delegated_shared_policy'
        | 'session_override'
        | 'user_preference'
        | 'provisional_fallback'
        | 'not_connected';
    };
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
    }
  | {
      readonly kind: 'update_session_identity';
      readonly name: string;
      readonly visualIdentity: HarnessVisualIdentity;
    }
  | {
      readonly kind: 'save_delegated_model_policy';
      readonly revision: number;
      readonly policy: HarnessModelPolicy;
    }
  | {
      readonly kind: 'set_session_model_override';
      readonly override: {
        readonly model: string;
        readonly reasoning: HarnessReasoningLevel | null;
      } | null;
    };

/** Every view uses this session-keyed boundary; components never select a harness or revision. */
export interface ConversationHarnessManagementSource {
  load(input: { readonly sessionId: string }): Promise<ConversationHarnessManagementRead>;
  dispatch?(input: {
    readonly sessionId: string;
    readonly command: ConversationHarnessManagementCommand;
  }): Promise<ConversationHarnessManagementRead>;
}
