import type { AgentSessionClient } from '../../application/agentSessions';
import type { ConversationHarnessInspectorSource } from '../../application/conversationHarnesses';
import { AgentSessionWorkspace, useAgentSession } from '../agentSessions';
import { HarnessAwareAgentSessionPane } from './HarnessAwareAgentSessionPane';
import './harnessInspector.css';

export interface HarnessInspectorDevelopmentComposition {
  readonly client: AgentSessionClient;
  readonly sessionId: string;
  readonly source: ConversationHarnessInspectorSource;
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
    <main className="harness-inspector-development" aria-label="Harness Inspector development">
      <header>
        <div>
          <p className="eyebrow">Development surface</p>
          <h1>Agent Session Harness Inspector</h1>
        </div>
        <p>
          Recorded adapters exercise the product component tree. They do not prove a live harness
          query or mutation path.
        </p>
      </header>
      <div className="harness-inspector-development__pane">
        <HarnessAwareAgentSessionPane sessionId={composition.sessionId} source={composition.source}>
          <AgentSessionWorkspace controller={controller} />
        </HarnessAwareAgentSessionPane>
      </div>
    </main>
  );
}
