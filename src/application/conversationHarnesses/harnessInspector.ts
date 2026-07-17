export type HarnessInspectorScope =
  'delivered_to_session' | 'future_invocation' | 'application_owned';

export type HarnessInspectorEditability = 'immutable' | 'read_only' | 'unsupported';

export interface HarnessInspectorSectionState {
  readonly scope: HarnessInspectorScope;
  readonly editability: HarnessInspectorEditability;
  readonly reason: string;
}

export interface ConversationHarnessInspectorSnapshot {
  readonly sessionId: string;
  readonly profile: {
    readonly key: string;
    readonly title: string;
    readonly version: number;
    readonly catalogSchemaVersion: number;
  };
  readonly provenance: {
    readonly kind: 'recorded_adapter' | 'product_query';
    readonly source: string;
    readonly summary: string;
  };
  readonly validation: {
    readonly status: 'valid' | 'invalid' | 'unverified';
    readonly checks: readonly {
      readonly label: string;
      readonly status: 'passed' | 'failed' | 'unverified';
      readonly detail: string;
    }[];
  };
  readonly promptContext: {
    readonly content: string;
    readonly delivery: 'first_query';
    readonly state: HarnessInspectorSectionState;
  };
  readonly skills: {
    readonly items: readonly {
      readonly name: string;
      readonly path: string;
      readonly purpose: string;
      readonly useWhen: string;
    }[];
    readonly state: HarnessInspectorSectionState;
  };
  readonly mcp: {
    readonly required: boolean;
    readonly tools: readonly string[];
    readonly state: HarnessInspectorSectionState;
  };
  readonly runtime: {
    readonly model: string | null;
    readonly reasoningEffort: string | null;
    readonly sandbox: 'read_only' | 'workspace_write' | 'danger_full_access';
    readonly approvalPolicy: 'never';
    readonly authorityBoundary: string;
    readonly state: HarnessInspectorSectionState;
  };
  readonly hooks: {
    readonly items: readonly {
      readonly name: string;
      readonly status: 'configured' | 'declarative_only' | 'unsupported';
      readonly detail: string;
    }[];
    readonly state: HarnessInspectorSectionState;
  };
  readonly apply: {
    readonly status: 'read_only' | 'unsupported';
    readonly reason: string;
    readonly safeSemantics: readonly string[];
  };
}

export type ConversationHarnessInspectorRead =
  | {
      readonly kind: 'available';
      readonly snapshot: ConversationHarnessInspectorSnapshot;
    }
  | {
      readonly kind: 'unavailable';
      readonly reason: string;
    };

/** Product contexts may expose this read boundary without adding harness concerns to Agent Session. */
export interface ConversationHarnessInspectorSource {
  load(input: { readonly sessionId: string }): Promise<ConversationHarnessInspectorRead>;
}
