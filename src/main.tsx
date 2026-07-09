import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { AppRoot } from './app/AppRoot';
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
    <AppRoot
      taskDashboardClient={tauriTaskDashboardClient}
      taskRunDetailClient={tauriTaskRunDetailClient}
      runtimeCommandClient={tauriRuntimeCommandClient}
      runtimeStatusClient={createDevRuntimeStatusClient()}
      backendMaintenanceClient={tauriBackendMaintenanceClient}
    />
  </StrictMode>,
);
