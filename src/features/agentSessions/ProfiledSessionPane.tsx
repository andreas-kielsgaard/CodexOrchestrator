import type {
  AgentSessionClient,
  AgentSessionProfileClient,
} from '../../application/agentSessions';
import type { SessionEventQueryClient } from '../../application/sessionEvents';
import { AgentSessionExecutionSettings } from './AgentSessionExecutionSettings';
import { AgentSessionHeaderActionsProvider, AgentSessionWorkspace } from './AgentSessionWorkspace';
import { useProfiledAgentSession } from './useProfiledAgentSession';
/** A selected conversation, independent of how its enclosing feature found it. */
export function ProfiledSessionPane({
  sessionId,
  client,
  profileClient,
  queryClient,
}: {
  sessionId: string;
  client: AgentSessionClient;
  profileClient: AgentSessionProfileClient;
  queryClient?: SessionEventQueryClient;
}) {
  const view = useProfiledAgentSession(client, profileClient, queryClient, {
    selectedSessionId: sessionId,
  });
  return (
    <AgentSessionHeaderActionsProvider
      actions={null}
      settings={
        <AgentSessionExecutionSettings
          profile={view.profile}
          profileError={view.profileError}
          deliveries={view.deliveries.deliveries}
          deliveryError={view.deliveries.error}
          onReloadDeliveries={view.deliveries.reload}
          selection={view.selection}
          onSelectionChange={view.setSelection}
          disabled={view.session.sending}
          identity={view.session.details?.session.assignedIdentity}
          onIdentityChange={view.updateIdentity}
        />
      }
    >
      <AgentSessionWorkspace
        controller={view.session}
        sendUnavailableReason={view.sendUnavailableReason}
      />
    </AgentSessionHeaderActionsProvider>
  );
}
