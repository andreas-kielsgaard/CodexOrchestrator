import { useMemo, useState } from 'react';
import type { BackendMaintenanceCapability } from '../capabilities/backendMaintenance';
import type { OpenTaskDashboardCapability } from '../capabilities/openTaskDashboard';
import type { RepoOnboardingCapability } from '../capabilities/repoOnboarding';
import type { RuntimeHealthCapability } from '../capabilities/runtimeHealth';
import type { TaskRunDetailCapability } from '../capabilities/taskRunDetail';
import type { RuntimeCommandClient } from '../application/commands/runtimeCommandClient';
import type { OrchestrationClient } from '../application/orchestrationClient';
import { createAgentSessionRouter } from '../application/agentSessionRouter';
import {
  fallbackRuntimeInfo,
  type CodexRuntimeInfo,
} from '../application/codexRuntimeInfoProvider';
import { createLocalOrchestrationClient } from '../infrastructure/localOrchestrationClient';
import { useOpenTasksFeatureController } from '../features/openTasks/controllers/useOpenTasksFeatureController';
import { OpenTasksScreen } from '../features/openTasks/views/OpenTasksScreen';
import { OrchestrationsPage } from '../features/orchestrations/views/OrchestrationsPage';
import { AgentSessionPage } from '../features/agentSessions/views/AgentSessionPage';
import { AppSidebar, RuntimeStaleNotice, type AppShellView } from './views/AppChrome';

export interface AppRootProps {
  taskDashboardClient: OpenTaskDashboardCapability & RepoOnboardingCapability;
  taskRunDetailClient: TaskRunDetailCapability;
  runtimeCommandClient: RuntimeCommandClient;
  orchestrationClient?: OrchestrationClient;
  runtimeStatusClient?: RuntimeHealthCapability;
  backendMaintenanceClient?: BackendMaintenanceCapability;
  loadCodexRuntimeInfo?: () => Promise<CodexRuntimeInfo>;
  startupLoadTimeoutMs?: number;
  reloadApp?(): void;
}

const defaultLoadCodexRuntimeInfo = async () => fallbackRuntimeInfo;

export function AppRoot({
  taskDashboardClient,
  taskRunDetailClient,
  runtimeCommandClient,
  orchestrationClient: providedOrchestrationClient,
  runtimeStatusClient,
  backendMaintenanceClient,
  loadCodexRuntimeInfo = defaultLoadCodexRuntimeInfo,
  startupLoadTimeoutMs,
  reloadApp,
}: AppRootProps) {
  const [activeView, setActiveView] = useState<AppShellView>('tasks');
  const fallbackOrchestrationClient = useMemo(() => createLocalOrchestrationClient(), []);
  const orchestrationClient = providedOrchestrationClient ?? fallbackOrchestrationClient;
  const agentSessionRouter = useMemo(
    () => createAgentSessionRouter(runtimeCommandClient),
    [runtimeCommandClient],
  );
  const openTasks = useOpenTasksFeatureController({
    taskDashboardClient,
    taskRunDetailClient,
    runtimeCommandClient,
    runtimeStatusClient,
    backendMaintenanceClient,
    startupLoadTimeoutMs,
    reloadApp,
  });

  if (!openTasks.view.hasLoadedDashboard && activeView === 'tasks') {
    return <OpenTasksScreen view={openTasks.view} actions={openTasks.actions} />;
  }

  return (
    <main className="app-shell">
      <AppSidebar
        activeView={activeView}
        backendMaintenance={openTasks.view.sidebar.backendMaintenance}
        onViewChange={setActiveView}
        onCheckBackend={openTasks.actions.checkBackend}
      />

      {openTasks.view.staleNoticeMessage && (
        <RuntimeStaleNotice
          message={openTasks.view.staleNoticeMessage}
          onRefresh={openTasks.actions.refreshAppRuntime}
          onDismiss={openTasks.actions.dismissStaleNotice}
        />
      )}

      {activeView === 'orchestrations' ? (
        <OrchestrationsPage orchestrationClient={orchestrationClient} />
      ) : activeView === 'agent-session' ? (
        <AgentSessionPage
          agentSessionRouter={agentSessionRouter}
          loadCodexRuntimeInfo={loadCodexRuntimeInfo}
        />
      ) : (
        <OpenTasksScreen view={openTasks.view} actions={openTasks.actions} />
      )}
    </main>
  );
}
