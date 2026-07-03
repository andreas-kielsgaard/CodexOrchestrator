import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import {
  tauriRuntimeCommandClient,
  tauriTaskDashboardClient,
} from './infrastructure/tauriCommands';
import './styles.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App
      taskDashboardClient={tauriTaskDashboardClient}
      runtimeCommandClient={tauriRuntimeCommandClient}
    />
  </StrictMode>,
);
