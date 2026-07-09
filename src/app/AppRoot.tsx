import type { BackendMaintenanceCapability } from '../capabilities/backendMaintenance';
import type { OpenTaskDashboardCapability } from '../capabilities/openTaskDashboard';
import type { RepoOnboardingCapability } from '../capabilities/repoOnboarding';
import type { RuntimeHealthCapability } from '../capabilities/runtimeHealth';
import type { TaskRunDetailCapability } from '../capabilities/taskRunDetail';
import type { RuntimeCommandClient } from '../application/commands/runtimeCommandClient';
import { useOpenTasksFeatureController } from '../features/openTasks/controllers/useOpenTasksFeatureController';
import { OpenTasksScreen } from '../features/openTasks/views/OpenTasksScreen';
import { AppSidebar, RuntimeStaleNotice } from './views/AppChrome';

export interface AppRootProps {
  taskDashboardClient: OpenTaskDashboardCapability & RepoOnboardingCapability;
  taskRunDetailClient: TaskRunDetailCapability;
  runtimeCommandClient: RuntimeCommandClient;
  runtimeStatusClient?: RuntimeHealthCapability;
  backendMaintenanceClient?: BackendMaintenanceCapability;
  startupLoadTimeoutMs?: number;
  reloadApp?(): void;
}

export function AppRoot({
  taskDashboardClient,
  taskRunDetailClient,
  runtimeCommandClient,
  runtimeStatusClient,
  backendMaintenanceClient,
  startupLoadTimeoutMs,
  reloadApp,
}: AppRootProps) {
  const openTasks = useOpenTasksFeatureController({
    taskDashboardClient,
    taskRunDetailClient,
    runtimeCommandClient,
    runtimeStatusClient,
    backendMaintenanceClient,
    startupLoadTimeoutMs,
    reloadApp,
  });

  if (!openTasks.view.hasLoadedDashboard) {
    return <OpenTasksScreen view={openTasks.view} actions={openTasks.actions} />;
  }

  return (
    <main className="app-shell">
      <AppSidebar
        activeView="tasks"
        backendMaintenance={openTasks.view.sidebar.backendMaintenance}
        onViewChange={() => undefined}
        onCheckBackend={openTasks.actions.checkBackend}
      />

      {openTasks.view.staleNoticeMessage && (
        <RuntimeStaleNotice
          message={openTasks.view.staleNoticeMessage}
          onRefresh={openTasks.actions.refreshAppRuntime}
          onDismiss={openTasks.actions.dismissStaleNotice}
        />
      )}

      <OpenTasksScreen view={openTasks.view} actions={openTasks.actions} />
    </main>
  );
}
