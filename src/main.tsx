import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import {
  tauriRuntimeCommandClient,
  tauriTaskDashboardClient,
  tauriTaskRunDetailClient,
} from './infrastructure/tauriCommands';
import './styles.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App
      taskDashboardClient={tauriTaskDashboardClient}
      taskRunDetailClient={tauriTaskRunDetailClient}
      runtimeCommandClient={tauriRuntimeCommandClient}
    />
  </StrictMode>,
);
