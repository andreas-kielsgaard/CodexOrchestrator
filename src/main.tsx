import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import {
  tauriRuntimeCommandClient,
  tauriTaskDashboardClient,
  tauriTaskRunDetailClient,
} from './infrastructure/tauriCommands';
import { createDevRuntimeStatusClient } from './infrastructure/devRuntimeStatusClient';
import { tauriAgentSessionClient } from './infrastructure/agentSessions/tauriAgentSessionClient';
import './styles.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App
      taskDashboardClient={tauriTaskDashboardClient}
      taskRunDetailClient={tauriTaskRunDetailClient}
      runtimeCommandClient={tauriRuntimeCommandClient}
      runtimeStatusClient={createDevRuntimeStatusClient()}
      agentSessionClient={tauriAgentSessionClient}
    />
  </StrictMode>,
);
