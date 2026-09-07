import { useCallback, useEffect, useState } from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
import type {
  AgentSessionProfileClient,
  PinnedAgentSessionProfileDto,
} from '../../application/agentSessionProfiles';
import type { SessionEventQueryClient } from '../../application/sessionEvents';
import { useSessionDeliveries } from '../sessionEvents/useSessionDeliveries';
import { AgentSessionExecutionSettings } from './AgentSessionExecutionSettings';
import { AgentSessionHeaderActionsProvider, AgentSessionWorkspace } from './AgentSessionWorkspace';
import { useAgentSession } from './useAgentSessionController';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';

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
  const [profile, setProfile] = useState<PinnedAgentSessionProfileDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selection, setSelection] = useState<PerMessageRuntimeSelection>({
    model: null,
    reasoningMode: null,
  });
  const deliveries = useSessionDeliveries(queryClient, sessionId);
  useEffect(() => {
    let active = true;
    setProfile(null);
    setError(null);
    setSelection({ model: null, reasoningMode: null });
    void profileClient.loadPinnedProfile(sessionId).then(
      (value) => active && setProfile(value),
      (cause) => active && setError(String(cause)),
    );
    return () => {
      active = false;
    };
  }, [profileClient, sessionId]);
  const send = useCallback(
    async (input: { sessionId: string; submittedText: string }) => {
      if (profile?.sessionId !== input.sessionId)
        throw new Error('Session configuration is unavailable.');
      const result = await profileClient.sendDirectUserMessage({ ...input, ...selection });
      setSelection({ model: null, reasoningMode: null });
      return result;
    },
    [profile, profileClient, selection],
  );
  const session = useAgentSession(client, {
    selectedSessionId: sessionId,
    sendExistingMessage: send,
  });
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
