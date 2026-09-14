import type {
  AgentSessionClient,
  AgentSessionProfileClient,
} from '../../application/agentSessions';
import type { SessionEventQueryClient } from '../../application/sessionEvents';
import { useSessionDeliveries } from '../sessionEvents/useSessionDeliveries';
import { useSessionExecutionSelection } from './useSessionExecutionSelection';
import { useAgentSession, type UseAgentSessionOptions } from './useAgentSession';
export function useProfiledAgentSession(
  client: AgentSessionClient,
  profileClient: AgentSessionProfileClient | undefined,
  queryClient: SessionEventQueryClient | undefined,
  options: UseAgentSessionOptions,
) {
  const { profile, error, selection, setSelection, execution } = useSessionExecutionSelection(
    profileClient,
    options.selectedSessionId,
    options.draftId,
    options.executionTarget
      ? `${options.executionTarget.capabilityProfileId}:${options.executionTarget.worktreeId}`
      : undefined,
  );
  const deliveries = useSessionDeliveries(queryClient, options.selectedSessionId);
  const session = useAgentSession(client, {
    ...options,
    execution: execution ? { ...execution, target: options.executionTarget } : undefined,
  });
  const updateIdentity =
    client.updateIdentity && session.details
      ? async (
          assignedIdentity: Parameters<
            NonNullable<AgentSessionClient['updateIdentity']>
          >[0]['assignedIdentity'],
        ) => {
          await client.updateIdentity!({
            sessionId: session.details!.session.id,
            assignedIdentity,
          });
          await session.reload();
        }
      : undefined;
  const sendUnavailableReason =
    profileClient && options.selectedSessionId && profile?.sessionId !== options.selectedSessionId
      ? error
        ? 'This Session has no available pinned configuration. Its history is still readable.'
        : 'Loading Session configuration…'
      : undefined;
  return {
    session,
    profile,
    profileError: error,
    selection,
    setSelection,
    deliveries,
    updateIdentity,
    sendUnavailableReason,
  };
}
