import { useMemo } from 'react';
import { createRecordedApplicationTestModeComposition } from './recordedApplicationTestMode';
import { AgentTestModeScreen } from './AgentTestModeScreen';

export function AgentTestModeRoot() {
  const composition = useMemo(() => createRecordedApplicationTestModeComposition(), []);
  return <AgentTestModeScreen {...composition} />;
}
