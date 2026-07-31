import type { AgentIdentity, AgentSessionClient } from '../../application/agentSessions';
import type { ConversationHarnessManagementSource } from '../../application/conversationHarnesses';
import { AgentSessionWorkspace, useAgentSession } from '../agentSessions';
import { HarnessAwareAgentSessionPane } from './HarnessAwareAgentSessionPane';
import './harnessInspector.css';

export interface HarnessInspectorDevelopmentComposition {
  readonly client: AgentSessionClient;
  readonly sessionId: string;
  readonly source: ConversationHarnessManagementSource;
  readonly agentIdentity?: AgentIdentity;
}

export function HarnessInspectorDevelopmentSurface({
  composition,
}: {
  readonly composition: HarnessInspectorDevelopmentComposition;
}) {
  const controller = useAgentSession(composition.client, {
    selectedSessionId: composition.sessionId,
  });

  return (
    <main className="harness-inspector-development" aria-label="Harness Management preview">
      <div className="harness-inspector-development__pane">
        <HarnessAwareAgentSessionPane sessionId={composition.sessionId} source={composition.source}>
          <AgentSessionWorkspace
            controller={controller}
            presentation={{
              identityHeader: {
                ...(composition.agentIdentity ? { agentIdentity: composition.agentIdentity } : {}),
                title: 'Epic Plan Builder',
              },
            }}
          />
        </HarnessAwareAgentSessionPane>
      </div>
    </main>
  );
}
