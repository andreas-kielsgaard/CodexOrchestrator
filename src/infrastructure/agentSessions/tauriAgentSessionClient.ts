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
  RenameAgentSessionCommandDto,
  SendAgentSessionMessageCommandDto,
  SendAgentSessionMessageResultDto,
  UpdateAgentSessionHarnessCommandDto,
  UpdateAgentSessionIdentityCommandDto,
  UpdateAgentSessionModelOverrideCommandDto,
} from '../../application/agentSessions';
import { createAgentSessionHistorySource } from './tauriAgentSessionHistorySource';

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
  const pendingRequests = new Map<string, string>();
  async function enqueue<T>(
    command: string,
    input: Record<string, unknown>,
    identity: string,
  ): Promise<T> {
    const key = JSON.stringify([command, input]);
    const id = pendingRequests.get(key) ?? crypto.randomUUID();
    pendingRequests.set(key, id);
    const result = await invokeCommand<T>(command, { input: { ...input, [identity]: id } });
    pendingRequests.delete(key);
    return result;
  }
  const listeners = new Set<AgentSessionUpdateListener>();
  let bridgePromise: Promise<TauriUnlisten> | undefined;

  const ensureUpdateBridge = (): Promise<TauriUnlisten> => {
    if (!bridgePromise) {
      bridgePromise = Promise.allSettled([
        listenForEvent<AgentSessionUpdateDto>(AGENT_SESSION_UPDATE_EVENT, (event) => {
          for (const listener of listeners) listener(event.payload);
        }),
        listenForEvent<string[]>('agent-session-replicas-updated', (event) => {
          for (const sessionId of event.payload) {
            for (const listener of listeners) listener({ kind: 'replica_updated', sessionId });
          }
        }),
      ])
        .then((results) => {
          const stops = results.flatMap((result) =>
            result.status === 'fulfilled' ? [result.value] : [],
          );
          const failure = results.find((result) => result.status === 'rejected');
          if (failure?.status === 'rejected') {
            stops.forEach((stop) => stop());
            throw failure.reason;
          }
          return () => stops.forEach((stop) => stop());
        })
        .catch((error: unknown) => {
          bridgePromise = undefined;
          throw error;
        });
    }
    return bridgePromise;
  };

  const load = (query: LoadAgentSessionQueryDto): Promise<AgentSessionDetailsDto> =>
    invokeCommand<AgentSessionDetailsDto>('load_agent_session', { query });

  const core: AgentSessionClient & AgentSessionProfileClient = {
    continuation: (sessionId) => invokeCommand('agent_session_continuation', { sessionId }),
    forkConversation: (sourceSessionId, forkId, target) =>
      invokeCommand('fork_agent_session', { sourceSessionId, forkId, target }),
    jevQueries: {
      list: (sessionId) => invokeCommand('list_jev_api_invocations', { sessionId }),
      load: (sessionId, invocationId) =>
        invokeCommand('load_jev_api_invocation', { sessionId, invocationId }),
    },
    loadCurrentProfile: (sessionId) =>
      invokeCommand('load_current_agent_session_profile', { input: { sessionId } }),
    sendPreparedMessage: async (input) => {
      await ensureUpdateBridge();
      return invokeCommand('send_prepared_agent_session_message', { input });
    },
    loadPreparation: (sessionId) =>
      invokeCommand('load_agent_session_preparation', { input: { sessionId } }),
    cancelPreparation: (invocationId) =>
      invokeCommand('cancel_agent_session_preparation', { input: { invocationId } }),
    retryPreparation: (invocationId) =>
      invokeCommand('retry_agent_session_preparation', { input: { invocationId } }),
    loadQueue: (sessionId) => invokeCommand('load_agent_session_queue', { input: { sessionId } }),
    sendQueueEntryNow: (input) => invokeCommand('send_queued_agent_session_message_now', { input }),
    runTargetOperation: (sessionId, entryId) =>
      invokeCommand('run_agent_session_target_operation', { input: { sessionId, entryId } }),
    enqueueTargetOperation: (input) =>
      enqueue('enqueue_agent_session_target_operation', { ...input }, 'operationId'),
    enqueueCompaction: (sessionId) =>
      enqueue('enqueue_agent_session_compaction', { sessionId }, 'submissionId'),
    reorderQueue: (input) => invokeCommand('reorder_agent_session_queue', { input }),
    updateQueuedMessage: (input) => invokeCommand('update_queued_agent_session_message', { input }),
    updateTargetOperation: (input) =>
      invokeCommand('update_agent_session_target_operation', { input }),
    recalculateTargetProjection: (input) =>
      invokeCommand('recalculate_agent_session_target_projection', { input }),
    retryQueueEntry: (sessionId, entryId) =>
      invokeCommand('retry_agent_session_queue_entry', { input: { sessionId, entryId } }),
    cancelQueueEntry: (sessionId, entryId) =>
      invokeCommand('cancel_agent_session_queue_entry', { input: { sessionId, entryId } }),
    loadQuickFeatures: (input) => invokeCommand('load_agent_session_quick_features', { input }),
    loadSkillDetails: (input, skillId) =>
      invokeCommand('read_agent_session_skill', { input, skillId }),
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
    editSteering: (input) => invokeCommand('edit_agent_session_steering', { input }),
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

    renameSession(command: RenameAgentSessionCommandDto): Promise<AgentSessionDto> {
      return invokeCommand<AgentSessionDto>('rename_agent_session', { input: command });
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
    loadEventDetails: (sessionId, eventId) =>
      invokeCommand('load_agent_session_event_details', { sessionId, eventId }),
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
  return {
    ...core,
    historySource: createAgentSessionHistorySource({
      load: (sessionId) =>
        invokeCommand<AgentSessionDetailsDto>('load_agent_session_display', {
          query: { sessionId },
        }),
      subscribe: (listener) => core.subscribeUpdates(listener),
    }),
  };
}

export const tauriAgentSessionClient = createTauriAgentSessionClient();
