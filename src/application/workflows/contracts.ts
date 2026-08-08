export interface WorkflowTypeSummary {
  readonly id: string;
  readonly name: string;
  readonly activeRecipeId: string | null;
  readonly editedElementCount: number;
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
  readonly nodes: readonly WorkflowNodeConfig[];
  readonly connections: readonly WorkflowConnectionConfig[];
}

export interface WorkflowDefinition {
  readonly workflowType: WorkflowTypeSummary;
  readonly nodes: readonly WorkflowNodeElement[];
  readonly connections: readonly WorkflowConnectionElement[];
  readonly activeRecipe: EffectiveWorkflowRecipe | null;
}

export type WorkflowElementRef = {
  readonly kind: 'node' | 'connection';
  readonly id: string;
};

export interface WorkflowApplicationClient {
  listWorkflowTypes(): Promise<readonly WorkflowTypeSummary[]>;
  createWorkflowType(input: { readonly name: string }): Promise<WorkflowDefinition>;
  loadWorkflowType(workflowTypeId: string): Promise<WorkflowDefinition>;
  saveNodeDraft(workflowTypeId: string, node: WorkflowNodeConfig): Promise<WorkflowDefinition>;
  deleteNodeDraft(workflowTypeId: string, nodeId: string): Promise<WorkflowDefinition>;
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
