import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import {
  tauriBackendMaintenanceClient,
  tauriRuntimeCommandClient,
  tauriTaskDashboardClient,
  tauriTaskRunDetailClient,
} from './infrastructure/tauriCommands';
import { createDevRuntimeStatusClient } from './infrastructure/devRuntimeStatusClient';
import './styles.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App
      taskDashboardClient={tauriTaskDashboardClient}
      taskRunDetailClient={tauriTaskRunDetailClient}
      runtimeCommandClient={tauriRuntimeCommandClient}
      backendMaintenanceClient={tauriBackendMaintenanceClient}
      runtimeStatusClient={createDevRuntimeStatusClient()}
    />
  </StrictMode>,
);
