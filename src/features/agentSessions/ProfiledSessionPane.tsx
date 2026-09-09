import type { AgentSessionClient } from '../../application/agentSessions';
import type { AgentSessionProfileClient } from '../../application/agentSessions';
import type { SessionEventQueryClient } from '../../application/sessionEvents';
import { useSessionDeliveries } from '../sessionEvents/useSessionDeliveries';
import { AgentSessionExecutionSettings } from './AgentSessionExecutionSettings';
import { AgentSessionHeaderActionsProvider, AgentSessionWorkspace } from './AgentSessionWorkspace';
import { useAgentSession } from './useAgentSessionController';
import { useSessionExecutionSelection } from './useSessionExecutionSelection';

/** A selected Session conversation, independent of how its enclosing feature found it. */
export function ProfiledSessionPane({
  sessionId,
  client,
  profileClient,
  queryClient,
}: {
  readonly sessionId: string;
  readonly client: AgentSessionClient;
  readonly profileClient: AgentSessionProfileClient;
  readonly queryClient?: SessionEventQueryClient;
}) {
  const { profile, error, selection, setSelection, execution } = useSessionExecutionSelection(
    profileClient,
    sessionId,
  );
  const deliveries = useSessionDeliveries(queryClient, sessionId);
  const session = useAgentSession(client, { selectedSessionId: sessionId, execution });
  return (
    <AgentSessionHeaderActionsProvider
      actions={null}
      settings={
        <AgentSessionExecutionSettings
          profile={profile}
          profileError={error}
          deliveries={deliveries.deliveries}
          deliveryError={deliveries.error}
          onReloadDeliveries={deliveries.reload}
          selection={selection}
          onSelectionChange={setSelection}
          disabled={session.sending}
          identity={session.details?.session.assignedIdentity}
          onIdentityChange={
            client.updateIdentity
              ? async (assignedIdentity) => {
                  await client.updateIdentity!({ sessionId, assignedIdentity });
                  await session.reload();
                }
              : undefined
          }
        />
      }
    >
      <AgentSessionWorkspace
        controller={session}
        sendUnavailableReason={
          profile?.sessionId === sessionId
            ? undefined
            : error
              ? 'This Session has no available pinned configuration. Its history is still readable.'
              : 'Loading Session configuration…'
        }
      />
    </AgentSessionHeaderActionsProvider>
  );
}
