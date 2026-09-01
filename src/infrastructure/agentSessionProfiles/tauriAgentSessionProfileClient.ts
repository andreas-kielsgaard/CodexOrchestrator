import { invoke } from '@tauri-apps/api/core';
import type {
  AgentSessionProfileClient,
  PinnedAgentSessionProfileDto,
  SendDirectUserAgentSessionMessageResultDto,
} from '../../application/agentSessionProfiles';

export type AgentSessionProfileInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export function createTauriAgentSessionProfileClient(
  invokeCommand: AgentSessionProfileInvoke = invoke,
): AgentSessionProfileClient {
  return {
    loadPinnedProfile: (sessionId) =>
      invokeCommand<PinnedAgentSessionProfileDto>('load_pinned_agent_session_profile', {
        input: { sessionId },
      }),
    sendDirectUserMessage: (input) =>
      invokeCommand<SendDirectUserAgentSessionMessageResultDto>(
        'send_direct_user_agent_session_message',
        { input },
      ),
  };
}

export const tauriAgentSessionProfileClient = createTauriAgentSessionProfileClient();
