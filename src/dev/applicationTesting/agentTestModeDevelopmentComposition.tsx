import type { AppProps } from '../../app/App';
import { createRecordedDevelopmentApplicationComposition } from '../orchestrationSection/recordedOrchestrationClient';
import { AgentTestModeRoot } from './AgentTestModeRoot';

/** Development-only peer surface over the same App shell and recorded product composition. */
export function createAgentTestModeDevelopmentApplicationComposition(): AppProps {
  return {
    ...createRecordedDevelopmentApplicationComposition(),
    initialSurface: 'development',
    developmentSurface: {
      label: 'Test mode',
      render: () => <AgentTestModeRoot />,
    },
  };
}
