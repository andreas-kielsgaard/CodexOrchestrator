import type {
  AgentSessionClient,
  SendAgentSessionMessageCommandDto,
  SendAgentSessionMessageResultDto,
} from '../../application/agentSessions';

export interface ManagedPlanBuilderSessionClient extends AgentSessionClient {
  requestPlan(
    command: Omit<SendAgentSessionMessageCommandDto, 'submittedText'>,
  ): Promise<SendAgentSessionMessageResultDto>;
}

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

/**
 * Plan Builder-only send seam. Session reads and updates remain the shared Agent Session contract.
 * The command acknowledgement is not evidence that a proposal was persisted.
 */
export function createTauriManagedPlanBuilderSessionClient(
  agentSessionClient: AgentSessionClient,
  invokeCommand: TauriInvoke,
): ManagedPlanBuilderSessionClient {
  return {
    ...agentSessionClient,
    sendMessage(
      command: SendAgentSessionMessageCommandDto,
    ): Promise<SendAgentSessionMessageResultDto> {
      return invokeCommand<SendAgentSessionMessageResultDto>('send_managed_plan_builder_message', {
        input: command,
      });
    },
    requestPlan(command) {
      return invokeCommand<SendAgentSessionMessageResultDto>(
        'request_managed_plan_builder_action',
        { input: command },
      );
    },
  };
}
