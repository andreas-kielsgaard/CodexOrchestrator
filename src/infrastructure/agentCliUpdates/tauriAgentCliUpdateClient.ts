import { invoke } from '@tauri-apps/api/core';
import type {
  AgentCliUpdateClient,
  AgentCliUpdateOverviewDto,
} from '../../application/agentCliUpdates';

export const tauriAgentCliUpdateClient: AgentCliUpdateClient = {
  overview: () => invoke<AgentCliUpdateOverviewDto>('agent_cli_update_overview'),
};
