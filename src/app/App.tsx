import { Inbox, MessagesSquare } from 'lucide-react';
import { useState } from 'react';
import type { AgentSessionClient } from '../application/agentSessions';
import type { RuntimeCommandClient } from '../application/runtimeCommandClient';
import type { RuntimeStatusClient } from '../application/runtimeStatusClient';
import type { TaskDashboardClient } from '../application/taskDashboardClient';
import type { TaskRunDetailClient } from '../application/taskRunDetailClient';
import { AgentSessionScreen } from '../features/agentSessions/AgentSessionScreen';
import { TaskDashboardScreen } from '../features/taskDashboard/TaskDashboardScreen';

interface AppProps {
  taskDashboardClient: TaskDashboardClient;
  taskRunDetailClient: TaskRunDetailClient;
  runtimeCommandClient: RuntimeCommandClient;
  runtimeStatusClient?: RuntimeStatusClient;
  agentSessionClient?: AgentSessionClient;
}

type Surface = 'sessions' | 'tasks';

export function App(props: AppProps) {
  const [surface, setSurface] = useState<Surface>('sessions');

  // Tests and embedders that do not provide the new client retain the exact legacy screen.
  if (!props.agentSessionClient) {
    return <TaskDashboardScreen {...props} />;
  }

  return (
    <div className="primary-app-shell">
      <nav className="surface-switcher" aria-label="Application surfaces">
        <button
          className={surface === 'sessions' ? 'active' : ''}
          type="button"
          onClick={() => setSurface('sessions')}
          aria-current={surface === 'sessions' ? 'page' : undefined}
        >
          <MessagesSquare size={17} aria-hidden="true" />
          Agent Sessions
        </button>
        <button
          className={surface === 'tasks' ? 'active' : ''}
          type="button"
          onClick={() => setSurface('tasks')}
          aria-current={surface === 'tasks' ? 'page' : undefined}
        >
          <Inbox size={17} aria-hidden="true" />
          Legacy Tasks
        </button>
      </nav>
      {surface === 'sessions' ? (
        <AgentSessionScreen client={props.agentSessionClient} />
      ) : (
        <TaskDashboardScreen {...props} />
      )}
    </div>
  );
}
