/** Administrative facts about a Harness, versioned separately from its behavior. */
export interface HarnessMetadata {
  readonly name: string;
}

/** Identities offered when a Session is initially instantiated with this Harness. */
export type HarnessIdentityAssignmentPolicy =
  | { readonly kind: 'unrestricted' }
  | {
      readonly kind: 'allow_list';
      readonly identityIds: readonly string[];
    };

export type HarnessDiscoveryPolicy = 'whitelist' | 'blacklist';
export type HarnessSkillPolicy = 'always_applicable' | 'initial_ingestion' | 'available';
export type HarnessToolPolicy = 'every_invocation' | 'initial_invocation' | 'available';
export type HarnessReasoningLevel = 'low' | 'medium' | 'high' | 'xhigh';
export type HarnessSandboxMode = 'read_only' | 'workspace_write' | 'danger_full_access';

export interface HarnessSkillConfiguration {
  readonly name: string;
  readonly path: string;
  readonly purpose: string;
  readonly useWhen: string;
  readonly policy: HarnessSkillPolicy;
}

export interface HarnessToolConfiguration {
  readonly name: string;
  readonly policy: HarnessToolPolicy;
}

export interface HarnessMcpServerExposure {
  readonly serverName: string;
  readonly access:
    | { readonly kind: 'entire_server' }
    | { readonly kind: 'selected_tools'; readonly toolNames: readonly string[] };
}

/** Optional preference resolved against the application-global model catalog. */
export interface HarnessModelPreference {
  readonly modelId: string;
  readonly reasoning: HarnessReasoningLevel | null;
}

export interface HarnessHookConfiguration {
  readonly name: string;
  readonly status: 'exposed' | 'proposed' | 'not_connected';
  readonly detail: string;
}

export type HarnessUpdatePolicy =
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

/**
 * Versioned Harness behavior only.
 *
 * Display metadata, provider definitions, model catalogs, Session-owned choices, and technical
 * revision evidence are deliberately outside this configuration.
 */
export interface HarnessConfiguration {
  readonly identityAssignment: HarnessIdentityAssignmentPolicy;
  readonly promptPrefix: {
    readonly content: string;
    readonly initialDelivery: 'prepend';
    readonly contextCompressionDelivery: 'deferred';
  };
  readonly skills: {
    readonly availableDiscoveryPolicy: HarnessDiscoveryPolicy;
    readonly items: readonly HarnessSkillConfiguration[];
  };
  readonly tools: {
    readonly availableDiscoveryPolicy: HarnessDiscoveryPolicy;
    readonly items: readonly HarnessToolConfiguration[];
    readonly schemaBoundary: string;
    readonly mcpServers: readonly HarnessMcpServerExposure[];
  };
  readonly runtime: {
    readonly preferredModel: HarnessModelPreference | null;
    readonly sandbox: HarnessSandboxMode;
    readonly approvalPolicy: 'never';
    readonly authoritySummary: string;
  };
  readonly hooks: readonly HarnessHookConfiguration[];
  readonly updatePolicy: HarnessUpdatePolicy;
}
