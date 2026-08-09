export interface WorkflowTypeSummary {
  readonly id: string;
  readonly name: string;
  readonly activeRecipeId: string | null;
  readonly editedElementCount: number;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface WorkflowHarnessConfig {
  readonly harnessName: string;
  readonly roleIdentity: string;
  readonly instructions: string;
  readonly skills: readonly string[];
  readonly mcpServers: readonly WorkflowMcpServerExposure[];
  readonly hooks: readonly string[];
  readonly runtime: WorkflowHarnessRuntimeSettings;
}

export interface WorkflowMcpServerExposure {
  readonly serverName: string;
  readonly access:
    | { readonly kind: 'entire_server' }
    | { readonly kind: 'selected_tools'; readonly toolNames: readonly string[] };
}

export interface WorkflowHarnessRuntimeSettings {
  readonly provider: string;
  readonly model: string;
  readonly reasoningEffort: string;
}

export interface WorkflowHarnessOverrides {
  readonly harnessName?: string | null;
  readonly roleIdentity?: string | null;
  readonly instructions?: string | null;
  readonly skills?: readonly string[] | null;
  readonly mcpServers?: readonly WorkflowMcpServerExposure[] | null;
  readonly hooks?: readonly string[] | null;
  readonly runtime?: WorkflowHarnessRuntimeSettings | null;
}

export type WorkflowNodeHarness =
  | {
      readonly kind: 'role';
      readonly roleId: string;
      readonly overrides: WorkflowHarnessOverrides;
    }
  | { readonly kind: 'standalone'; readonly config: WorkflowHarnessConfig };

export interface WorkflowRole {
  readonly id: string;
  readonly name: string;
  readonly harness: WorkflowHarnessConfig;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface WorkflowNodeConfig {
  readonly id: string;
  readonly name: string;
  readonly harnessName: string;
  readonly roleName: string | null;
  readonly positionX: number;
  readonly positionY: number;
  readonly isStartingPoint: boolean;
  readonly harness?: WorkflowNodeHarness | null;
}

export interface WorkflowConnectionConfig {
  readonly id: string;
  readonly name: string;
  readonly senderNodeId: string;
  readonly receiverNodeId: string | null;
  readonly mechanism: WorkflowConnectionMechanism | null;
}

export type WorkflowConnectionMechanism = {
  readonly kind: 'turn_finished_expected_file';
  readonly fileSelector: WorkflowExpectedFileSelector;
  readonly descriptionText: string;
  readonly promptText: string;
  readonly matchSelection: 'newest';
  readonly initialCheck: 'once_immediately';
};

export type WorkflowExpectedFileSelector =
  | {
      readonly kind: 'folder_filename_pattern';
      readonly folder: string;
      readonly filenamePattern: string;
    }
  | {
      readonly kind: 'folder_output_regex';
      readonly folder: string;
      readonly outputRegex: string;
    };

export interface WorkflowNodeElement {
  readonly id: string;
  readonly draft: WorkflowNodeConfig | null;
  readonly live: WorkflowNodeConfig | null;
  readonly hasUnpublishedChanges: boolean;
  readonly draftEffectiveHarness?: WorkflowHarnessConfig | null;
  readonly liveEffectiveHarness?: WorkflowHarnessConfig | null;
}

export interface EffectiveWorkflowNodeConfig {
  readonly id: string;
  readonly name: string;
  readonly positionX: number;
  readonly positionY: number;
  readonly isStartingPoint: boolean;
  readonly harness: WorkflowHarnessConfig;
}

export interface WorkflowConnectionElement {
  readonly id: string;
  readonly draft: WorkflowConnectionConfig | null;
  readonly live: WorkflowConnectionConfig | null;
  readonly hasUnpublishedChanges: boolean;
}

export interface EffectiveWorkflowRecipe {
  readonly id: string;
  readonly workflowTypeId: string;
  readonly ordinal: number;
  readonly createdAt: string;
  readonly nodes: readonly EffectiveWorkflowNodeConfig[];
  readonly connections: readonly WorkflowConnectionConfig[];
}

export interface WorkflowDefinition {
  readonly workflowType: WorkflowTypeSummary;
  readonly nodes: readonly WorkflowNodeElement[];
  readonly connections: readonly WorkflowConnectionElement[];
  readonly activeRecipe: EffectiveWorkflowRecipe | null;
}

export type WorkflowLaunchStatus =
  'requested' | 'associated' | 'launch_requested' | 'launch_accepted' | 'failed';

export interface WorkflowInstanceSummary {
  readonly id: string;
  readonly workflowTypeId: string;
  readonly workflowTypeName: string;
  readonly recipeId: string;
  readonly name: string;
  readonly sessionCount: number;
  readonly activeSessionCount: number;
  readonly idleSessionCount: number;
  readonly launchStatus: WorkflowLaunchStatus;
  readonly createdAt: string;
}

export interface WorkflowInstanceSession {
  readonly nodeId: string;
  readonly sessionId: string;
  readonly title: string;
  readonly activity: 'active' | 'idle';
  readonly latestTurnSummary: string | null;
  readonly associatedAt: string;
}

export interface WorkflowActivation {
  readonly id: string;
  readonly sourceKind: 'human';
  readonly targetNodeId: string;
  readonly targetSessionId: string;
  readonly targetInvocationId: string;
  readonly deliveryKind: 'direct_prompt_runtime_v1';
  readonly sessionMode: 'fresh';
  readonly contextInheritance: 'none';
  readonly compression: 'none';
  readonly status: WorkflowLaunchStatus;
  readonly requestedAt: string;
  readonly associatedAt: string | null;
  readonly launchRequestedAt: string | null;
  readonly launchAcceptedAt: string | null;
  readonly failedAt: string | null;
  readonly failureStage: string | null;
  readonly failureReason: string | null;
}

export interface WorkflowInstance {
  readonly summary: WorkflowInstanceSummary;
  readonly startingPrompt: string;
  readonly workingDirectory: string;
  readonly recipe: EffectiveWorkflowRecipe;
  readonly sessions: readonly WorkflowInstanceSession[];
  readonly launchActivation: WorkflowActivation;
  readonly connectionActivations: readonly WorkflowConnectionActivation[];
}

export type WorkflowConnectionActivationStatus =
  'requested' | 'resolved' | 'associated' | 'launch_requested' | 'launch_accepted' | 'failed';

export interface WorkflowConnectionActivation {
  readonly id: string;
  readonly recipeId: string;
  readonly connectionId: string;
  readonly senderNodeId: string;
  readonly receiverNodeId: string;
  readonly sourceSessionId: string;
  readonly sourceInvocationId: string;
  readonly targetSessionId: string | null;
  readonly targetInvocationId: string | null;
  readonly deliveryKind: string;
  readonly sessionMode: string | null;
  readonly contextInheritance: string;
  readonly compression: string;
  readonly resolvedFilePath: string | null;
  readonly status: WorkflowConnectionActivationStatus;
  readonly requestedAt: string;
  readonly resolvedAt: string | null;
  readonly associatedAt: string | null;
  readonly launchRequestedAt: string | null;
  readonly launchAcceptedAt: string | null;
  readonly failedAt: string | null;
}

export type WorkflowElementRef = {
  readonly kind: 'node' | 'connection';
  readonly id: string;
};

export interface WorkflowApplicationClient {
  listWorkflowTypes(): Promise<readonly WorkflowTypeSummary[]>;
  listWorkflowInstances(): Promise<readonly WorkflowInstanceSummary[]>;
  launchWorkflowInstance(input: {
    readonly workflowTypeId: string;
    readonly name?: string | null;
    readonly startingPrompt: string;
  }): Promise<WorkflowInstance>;
  loadWorkflowInstance(workflowInstanceId: string): Promise<WorkflowInstance>;
  listRoles(): Promise<readonly WorkflowRole[]>;
  createRole(input: {
    readonly name: string;
    readonly harness: WorkflowHarnessConfig;
  }): Promise<WorkflowRole>;
  updateRole(input: {
    readonly roleId: string;
    readonly name: string;
    readonly harness: WorkflowHarnessConfig;
  }): Promise<WorkflowRole>;
  createWorkflowType(input: { readonly name: string }): Promise<WorkflowDefinition>;
  loadWorkflowType(workflowTypeId: string): Promise<WorkflowDefinition>;
  saveNodeDraft(workflowTypeId: string, node: WorkflowNodeConfig): Promise<WorkflowDefinition>;
  deleteNodeDraft(workflowTypeId: string, nodeId: string): Promise<WorkflowDefinition>;
  detachNodeRole(workflowTypeId: string, nodeId: string): Promise<WorkflowDefinition>;
  saveNodeAsRole(
    workflowTypeId: string,
    nodeId: string,
    roleName: string,
  ): Promise<WorkflowDefinition>;
  saveConnectionDraft(
    workflowTypeId: string,
    connection: WorkflowConnectionConfig,
  ): Promise<WorkflowDefinition>;
  deleteConnectionDraft(workflowTypeId: string, connectionId: string): Promise<WorkflowDefinition>;
  activateChanges(
    workflowTypeId: string,
    elements: readonly WorkflowElementRef[],
  ): Promise<WorkflowDefinition>;
}
