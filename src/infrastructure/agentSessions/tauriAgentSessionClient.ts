import type { AgentSessionProfileClient } from '../../application/agentSessions';
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
  UpdateAgentSessionHarnessCommandDto,
  UpdateAgentSessionIdentityCommandDto,
  UpdateAgentSessionModelOverrideCommandDto,
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
): AgentSessionClient & AgentSessionProfileClient {
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
    loadQuickFeatures: (input) => invokeCommand('load_agent_session_quick_features', { input }),
    startDirectUserSession: async (input) => {
      await ensureUpdateBridge();
      return invokeCommand('start_direct_user_agent_session', { input });
    },
    loadPinnedProfile: (sessionId) =>
      invokeCommand('load_pinned_agent_session_profile', { input: { sessionId } }),
    sendDirectUserMessage: async (input) => {
      await ensureUpdateBridge();
      return invokeCommand('send_direct_user_agent_session_message', { input });
    },
    resolveWorkingDirectory: (sessionId, directory) =>
      invokeCommand('resolve_agent_session_working_directory', { sessionId, directory }),
    steerSession: (input) => invokeCommand('steer_agent_session', { input }),
    respondToRuntimeRequest: (input) =>
      invokeCommand('respond_to_agent_runtime_request', { input }),

    createSession(command: CreateAgentSessionCommandDto): Promise<AgentSessionDto> {
      return invokeCommand<AgentSessionDto>('create_agent_session', { input: command });
    },

    updateHarness(command: UpdateAgentSessionHarnessCommandDto): Promise<AgentSessionDto> {
      return invokeCommand<AgentSessionDto>('update_agent_session_harness', { input: command });
    },

    updateIdentity(command: UpdateAgentSessionIdentityCommandDto): Promise<AgentSessionDto> {
      return invokeCommand<AgentSessionDto>('update_agent_session_identity', { input: command });
    },

    updateModelOverride(
      command: UpdateAgentSessionModelOverrideCommandDto,
    ): Promise<AgentSessionDto> {
      return invokeCommand<AgentSessionDto>('update_agent_session_model_override', {
        input: command,
      });
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
