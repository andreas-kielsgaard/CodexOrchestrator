import type { AgentSessionClient } from '../application/agentSessions';
import { AgentSessionScreen } from '../features/agentSessions/AgentSessionScreen';

interface AppProps {
  agentSessionClient: AgentSessionClient;
}

export function App({ agentSessionClient }: AppProps) {
  return <AgentSessionScreen client={agentSessionClient} />;
}
