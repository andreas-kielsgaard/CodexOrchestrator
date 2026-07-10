import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type {
  AgentInvocationDto,
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionDto,
  AgentSessionSummaryDto,
  AgentSessionUpdateDto,
  AgentSessionUpdateListener,
  CancelAgentInvocationCommandDto,
  CreateAgentSessionCommandDto,
  ListAgentSessionsQueryDto,
  LoadAgentSessionQueryDto,
  SendAgentSessionMessageCommandDto,
  SendAgentSessionMessageResultDto,
} from '../../application/agentSessions';

export const AGENT_SESSION_UPDATE_EVENT = 'agent-session-update';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
type TauriUnlisten = () => void;
type TauriListen = <T>(
  event: string,
  handler: (event: { payload: T }) => void,
) => Promise<TauriUnlisten>;

export interface TauriAgentSessionClientDependencies {
  invoke?: TauriInvoke;
  listen?: TauriListen;
}

export function createTauriAgentSessionClient(
  dependencies: TauriAgentSessionClientDependencies = {},
): AgentSessionClient {
  const invokeCommand = dependencies.invoke ?? invoke;
  const listenForEvent = dependencies.listen ?? listen;
  const listeners = new Set<AgentSessionUpdateListener>();
  let bridgePromise: Promise<TauriUnlisten> | undefined;

  const ensureUpdateBridge = (): Promise<TauriUnlisten> => {
    if (!bridgePromise) {
      bridgePromise = listenForEvent<AgentSessionUpdateDto>(AGENT_SESSION_UPDATE_EVENT, (event) => {
        for (const listener of listeners) {
          listener(event.payload);
        }
      }).catch((error: unknown) => {
        bridgePromise = undefined;
        throw error;
      });
    }
    return bridgePromise;
  };

  const load = (query: LoadAgentSessionQueryDto): Promise<AgentSessionDetailsDto> =>
    invokeCommand<AgentSessionDetailsDto>('load_agent_session', { query });

  return {
    createSession(command: CreateAgentSessionCommandDto): Promise<AgentSessionDto> {
      return invokeCommand<AgentSessionDto>('create_agent_session', { input: command });
    },

    listSessions(query: ListAgentSessionsQueryDto = {}): Promise<AgentSessionSummaryDto[]> {
      return invokeCommand<AgentSessionSummaryDto[]>('list_agent_sessions', { query });
    },

    loadSession: load,
    reloadSession: load,

    async subscribeUpdates(listener: AgentSessionUpdateListener): Promise<() => void> {
      listeners.add(listener);
      try {
        await ensureUpdateBridge();
      } catch (error) {
        listeners.delete(listener);
        throw error;
      }
      return () => listeners.delete(listener);
    },

    async sendMessage(
      command: SendAgentSessionMessageCommandDto,
    ): Promise<SendAgentSessionMessageResultDto> {
      // A fast child may persist and emit before the command acknowledgement returns.
      await ensureUpdateBridge();
      return invokeCommand<SendAgentSessionMessageResultDto>('send_agent_session_message', {
        input: command,
      });
    },

    cancelInvocation(command: CancelAgentInvocationCommandDto): Promise<AgentInvocationDto> {
      return invokeCommand<AgentInvocationDto>('cancel_agent_invocation', { input: command });
    },

    async disconnectUpdates(): Promise<void> {
      const current = bridgePromise;
      bridgePromise = undefined;
      listeners.clear();
      if (current) {
        (await current)();
      }
    },
  };
}

export const tauriAgentSessionClient = createTauriAgentSessionClient();
