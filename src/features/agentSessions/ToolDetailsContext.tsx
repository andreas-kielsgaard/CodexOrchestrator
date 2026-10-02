import { createContext, useContext } from 'react';
import type { AgentRuntimeEventDto } from '../../application/agentSessions';
export const ToolDetailsContext = createContext<
  ((id: string) => Promise<AgentRuntimeEventDto>) | undefined
>(undefined);
export function useToolDetails() {
  return useContext(ToolDetailsContext);
}
