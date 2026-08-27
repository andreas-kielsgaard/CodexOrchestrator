import type {
  HarnessEffectiveConfiguration,
  HarnessMcpServerExposure,
} from '../conversationHarnesses';
import type {
  PartialAgentRuntimeOptionsDto,
  SendAgentSessionMessageResultDto,
} from '../agentSessions';
import type {
  CreateWorkflowInstanceInput,
  WorkflowInstance,
  WorkflowInstanceSummary,
} from './instanceContracts';

export type {
  CreateWorkflowInstanceInput,
  WorkflowInstance,
  WorkflowInstanceSession,
  WorkflowInstanceSummary,
} from './instanceContracts';

export interface WorkflowTypeSummary {
  readonly id: string;
  readonly name: string;
  readonly activeRecipeId: string | null;
  readonly editedElementCount: number;
  readonly createdAt: string;
  readonly updatedAt: string;
}

/** Detached definition authority shared by Roles, nodes, and Session Harness revisions. */
export type WorkflowHarnessConfig = HarnessEffectiveConfiguration;
export type WorkflowMcpServerExposure = HarnessMcpServerExposure;

export interface WorkflowHarnessOverrides {
  readonly identityName?: string | null;
  readonly identityMachineKey?: string | null;
  readonly permittedAgentNames?: readonly string[] | null;
  readonly visualIdentity?: HarnessEffectiveConfiguration['identity']['visualIdentity'] | null;
  readonly promptPrefixContent?: string | null;
  readonly skillDiscoveryPolicy?:
    HarnessEffectiveConfiguration['skills']['availableDiscoveryPolicy'] | null;
  readonly skillItems?: HarnessEffectiveConfiguration['skills']['items'] | null;
  readonly toolDiscoveryPolicy?:
    HarnessEffectiveConfiguration['tools']['availableDiscoveryPolicy'] | null;
  readonly toolItems?: HarnessEffectiveConfiguration['tools']['items'] | null;
  readonly toolSchemaBoundary?: string | null;
  readonly mcpServers?: readonly WorkflowMcpServerExposure[] | null;
  readonly runtimeModelPolicyMode?:
    HarnessEffectiveConfiguration['runtime']['modelPolicyMode'] | null;
  readonly runtimeModels?: HarnessEffectiveConfiguration['runtime']['models'] | null;
  readonly runtimeDefaultModel?: string | null;
  readonly runtimeDefaultReasoning?:
    HarnessEffectiveConfiguration['runtime']['defaultReasoning'] | null;
  readonly runtimeSandbox?: HarnessEffectiveConfiguration['runtime']['sandbox'] | null;
  readonly runtimeSandboxOptions?:
    HarnessEffectiveConfiguration['runtime']['sandboxOptions'] | null;
  readonly runtimeApprovalPolicy?:
    HarnessEffectiveConfiguration['runtime']['approvalPolicy'] | null;
  readonly runtimeApprovalPolicyOptions?:
    HarnessEffectiveConfiguration['runtime']['approvalPolicyOptions'] | null;
  readonly runtimeAuthoritySummary?: string | null;
  readonly hookItems?: HarnessEffectiveConfiguration['hooks'] | null;
  readonly updatePolicy?: HarnessEffectiveConfiguration['updatePolicy'] | null;
  /** v43 compatibility only; new editors write the canonical leaf fields above. */
  readonly harnessName?: string | null;
  readonly roleIdentity?: string | null;
  readonly instructions?: string | null;
  readonly skills?: readonly string[] | null;
  readonly hooks?: readonly string[] | null;
  readonly runtime?: {
    readonly provider: string;
    readonly model: string;
    readonly reasoningEffort: string;
  } | null;
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
  /** Missing only on definitions written before receiver placement became configurable. */
  readonly receiverSessionPolicy?: WorkflowReceiverSessionPolicy;
  readonly mechanism: WorkflowConnectionMechanism | null;
}

export type WorkflowReceiverSessionPolicy = 'fresh' | 'continue_latest';

export type WorkflowConnectionMechanism =
  | {
      readonly kind: 'turn_finished_expected_file';
      readonly fileSelector: WorkflowExpectedFileSelector;
      readonly descriptionText: string;
      readonly promptText: string;
      readonly matchSelection: 'newest';
      readonly initialCheck: 'once_immediately';
    }
  | {
      readonly kind: 'mcp_native_prompt_agent';
      readonly serverName: string;
      readonly toolName: string;
      readonly warningText: string | null;
    };

export interface WorkflowMcpComponent {
  readonly serverName: string;
  readonly toolName: string;
  readonly title: string;
  readonly participationMode: string;
  readonly interfaceId: string;
}

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
  listWorkflowMcpComponents(): Promise<readonly WorkflowMcpComponent[]>;
  createWorkflowInstance(input: CreateWorkflowInstanceInput): Promise<WorkflowInstance>;
  sendWorkflowNodeMessage(input: {
    readonly workflowInstanceId: string;
    readonly nodeId: string;
    readonly submittedText: string;
    readonly title?: string;
    readonly requestedOptions?: PartialAgentRuntimeOptionsDto;
  }): Promise<SendAgentSessionMessageResultDto>;
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
