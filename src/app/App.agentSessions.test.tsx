import { fireEvent, render, screen } from '@testing-library/react';
import type { AgentSessionClient } from '../application/agentSessions';
import type { RuntimeCommandClient } from '../application/runtimeCommandClient';
import type { TaskDashboardClient } from '../application/taskDashboardClient';
import type { TaskRunDetailClient } from '../application/taskRunDetailClient';
import { sessionDetails } from '../features/agentSessions/testFixtures';
import { App } from './App';

describe('App Agent Session shell', () => {
  it('defaults to Agent Sessions and does not initialize the task dashboard', async () => {
    const taskClient = failingTaskClient();
    render(
      <App
        agentSessionClient={emptyAgentClient()}
        taskDashboardClient={taskClient}
        taskRunDetailClient={emptyDetailClient}
        runtimeCommandClient={emptyRuntimeClient}
      />,
    );

    expect(await screen.findByText('Start with a message')).toBeInTheDocument();
    expect(taskClient.loadDashboard).not.toHaveBeenCalled();
    expect(screen.queryByText('Task dashboard unavailable')).not.toBeInTheDocument();
  });

  it('keeps the task dashboard secondary and its initialization failure isolated', async () => {
    const taskClient = failingTaskClient();
    render(
      <App
        agentSessionClient={emptyAgentClient()}
        taskDashboardClient={taskClient}
        taskRunDetailClient={emptyDetailClient}
        runtimeCommandClient={emptyRuntimeClient}
      />,
    );
    expect(await screen.findByText('Start with a message')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Legacy Tasks' }));
    expect(await screen.findByText('Task dashboard unavailable')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    expect(await screen.findByText('Start with a message')).toBeInTheDocument();
  });
});

function emptyAgentClient(): AgentSessionClient {
  return {
    createSession: async () => sessionDetails().session,
    listSessions: async () => [],
    loadSession: async () => sessionDetails(),
    reloadSession: async () => sessionDetails(),
    subscribeUpdates: async () => () => undefined,
    sendMessage: async () => ({ sessionId: 'session-1', invocationId: 'invocation-1' }),
    cancelInvocation: async () => sessionDetails('canceled').invocations[0].invocation,
    disconnectUpdates: async () => undefined,
  };
}

function failingTaskClient(): TaskDashboardClient & { loadDashboard: ReturnType<typeof vi.fn> } {
  return {
    loadDashboard: vi.fn().mockRejectedValue(new Error('Task dashboard unavailable')),
    createTask: vi.fn(),
    updateTask: vi.fn(),
    archiveTask: vi.fn(),
  };
}

const emptyRuntimeClient: RuntimeCommandClient = {
  startCodexTaskRun: vi.fn(),
};

const emptyDetailClient: TaskRunDetailClient = {
  loadTaskRunDetail: vi.fn(),
};
