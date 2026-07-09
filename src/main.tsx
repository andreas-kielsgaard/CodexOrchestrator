import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import { createBrowserDevClientBundle } from './infrastructure/browserDevClients';
import {
  tauriRuntimeCommandClient,
  tauriTaskDashboardClient,
  tauriTaskRunDetailClient,
  loadTauriCodexRuntimeInfo,
} from './infrastructure/tauriCommands';
import { fallbackRuntimeInfo } from './application/codexRuntimeInfoProvider';
import { createDevRuntimeStatusClient } from './infrastructure/devRuntimeStatusClient';
import { createLocalOrchestrationClient } from './infrastructure/localOrchestrationClient';
import { tauriOrchestrationClient } from './infrastructure/tauriOrchestrationClient';
import './ui/styles.css';
import './styles.css';

const localOrchestrationClient = createLocalOrchestrationClient();
const browserDevClients = isTauriRuntime() ? null : createBrowserDevClientBundle();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App
      taskDashboardClient={browserDevClients?.taskDashboardClient ?? tauriTaskDashboardClient}
      taskRunDetailClient={browserDevClients?.taskRunDetailClient ?? tauriTaskRunDetailClient}
      runtimeCommandClient={browserDevClients?.runtimeCommandClient ?? tauriRuntimeCommandClient}
      orchestrationClient={isTauriRuntime() ? tauriOrchestrationClient : localOrchestrationClient}
      runtimeStatusClient={createDevRuntimeStatusClient()}
      loadCodexRuntimeInfo={
        isTauriRuntime() ? loadTauriCodexRuntimeInfo : async () => fallbackRuntimeInfo
      }
    />
  </StrictMode>,
);

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
