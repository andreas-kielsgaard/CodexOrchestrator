import type {
  AgentSessionClient,
  PartialAgentRuntimeOptionsDto,
  SendAgentSessionMessageResultDto,
} from '../agentSessions';

export interface WorkflowAgentSessionBinding {
  readonly workflowInstanceId: string;
  readonly nodeId: string;
}

export interface SendWorkflowNodeMessageCommand {
  readonly workflowInstanceId: string;
  readonly nodeId: string;
  readonly submittedText: string;
  readonly title?: string;
  readonly requestedOptions?: PartialAgentRuntimeOptionsDto;
}

export interface WorkflowNodeMessageClient {
  sendWorkflowNodeMessage(
    command: SendWorkflowNodeMessageCommand,
  ): Promise<SendAgentSessionMessageResultDto>;
}

/**
 * Presents a Workflow node as an ordinary Agent Session client.
 *
 * Only creation-by-first-message is Workflow-owned. Once a Session ID exists, the complete
 * Agent Session contract delegates to the ordinary client unchanged.
 */
export function createWorkflowAgentSessionClient(
  ordinaryClient: AgentSessionClient,
  workflowClient: WorkflowNodeMessageClient,
  binding: WorkflowAgentSessionBinding,
): AgentSessionClient {
  return {
    createSession: (command) => ordinaryClient.createSession(command),
    listSessions: (query) => ordinaryClient.listSessions(query),
    loadSession: (query) => ordinaryClient.loadSession(query),
    reloadSession: (query) => ordinaryClient.reloadSession(query),
    subscribeUpdates: (listener) => ordinaryClient.subscribeUpdates(listener),
    sendMessage: (command) => {
      if (command.sessionId !== undefined) return ordinaryClient.sendMessage(command);

      return workflowClient.sendWorkflowNodeMessage({
        workflowInstanceId: binding.workflowInstanceId,
        nodeId: binding.nodeId,
        submittedText: command.submittedText,
        ...(command.title !== undefined ? { title: command.title } : {}),
        ...(command.requestedOptions !== undefined
          ? { requestedOptions: command.requestedOptions }
          : {}),
      });
    },
    cancelInvocation: (command) => ordinaryClient.cancelInvocation(command),
    disconnectUpdates: () => ordinaryClient.disconnectUpdates(),
  };
}
