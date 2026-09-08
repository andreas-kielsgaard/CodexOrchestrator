import { vi } from 'vitest';
import type {
  AgentInvocationDto,
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionDto,
  AgentSessionSummaryDto,
  AgentSessionUpdateListener,
} from '../agentSessions';
import {
  createWorkflowAgentSessionClient,
  type WorkflowNodeMessageClient,
} from './workflowAgentSessionClient';

describe('Workflow Agent Session client', () => {
  it('routes creation-by-first-message through the exact Workflow instance and node binding', async () => {
    const ordinary = ordinaryClient();
    const workflow: WorkflowNodeMessageClient = {
      sendWorkflowNodeMessage: vi.fn(async () => ({
        sessionId: 'workflow-session-1',
        invocationId: 'workflow-invocation-1',
      })),
    };
    const client = createWorkflowAgentSessionClient(ordinary, workflow, {
      workflowInstanceId: 'workflow-instance-1',
      nodeId: 'review-node',
    });

    await expect(
      client.sendMessage({
        submittedText: 'Review the implementation.',
        title: 'Implementation review',
        workingDirectory: 'C:/must-not-override-the-workflow-target',
        requestedOptions: { model: 'gpt-5.6-sol', sandbox: 'workspace_write' },
      }),
    ).resolves.toEqual({
      sessionId: 'workflow-session-1',
      invocationId: 'workflow-invocation-1',
    });
    expect(workflow.sendWorkflowNodeMessage).toHaveBeenCalledOnce();
    expect(workflow.sendWorkflowNodeMessage).toHaveBeenCalledWith({
      workflowInstanceId: 'workflow-instance-1',
      nodeId: 'review-node',
      submittedText: 'Review the implementation.',
      title: 'Implementation review',
      requestedOptions: { model: 'gpt-5.6-sol', sandbox: 'workspace_write' },
    });
    expect(ordinary.sendMessage).not.toHaveBeenCalled();
  });

  it('delegates an existing Session send to the ordinary client unchanged', async () => {
    const ordinary = ordinaryClient();
    const workflow: WorkflowNodeMessageClient = {
      sendWorkflowNodeMessage: vi.fn(),
    };
    const client = createWorkflowAgentSessionClient(ordinary, workflow, {
      workflowInstanceId: 'workflow-instance-1',
      nodeId: 'review-node',
    });
    const command = {
      sessionId: 'session-1',
      submittedText: 'Continue the review.',
      workingDirectory: 'C:/ordinary-client-owned-value',
    };

    await expect(client.sendMessage(command)).resolves.toEqual({
      sessionId: 'session-1',
      invocationId: 'invocation-1',
    });
    expect(ordinary.sendMessage).toHaveBeenCalledWith(command);
    expect(workflow.sendWorkflowNodeMessage).not.toHaveBeenCalled();
  });

  it('delegates every non-send Agent Session operation without Workflow policy', async () => {
    const ordinary = ordinaryClient();
    const workflow: WorkflowNodeMessageClient = {
      sendWorkflowNodeMessage: vi.fn(),
    };
    const client = createWorkflowAgentSessionClient(ordinary, workflow, {
      workflowInstanceId: 'workflow-instance-1',
      nodeId: 'review-node',
    });
    const listener: AgentSessionUpdateListener = vi.fn();

    await client.createSession({ title: 'Delegated Session', workingDirectory: 'C:/workspace' });
    await client.listSessions({ availability: 'available', limit: 4 });
    await client.loadSession({ sessionId: 'session-1' });
    await client.reloadSession({ sessionId: 'session-1' });
    await client.subscribeUpdates(listener);
    await client.cancelInvocation({ invocationId: 'invocation-1' });
    await client.disconnectUpdates();

    expect(ordinary.createSession).toHaveBeenCalledWith({
      title: 'Delegated Session',
      workingDirectory: 'C:/workspace',
    });
    expect(ordinary.listSessions).toHaveBeenCalledWith({ availability: 'available', limit: 4 });
    expect(ordinary.loadSession).toHaveBeenCalledWith({ sessionId: 'session-1' });
    expect(ordinary.reloadSession).toHaveBeenCalledWith({ sessionId: 'session-1' });
    expect(ordinary.subscribeUpdates).toHaveBeenCalledWith(listener);
    expect(ordinary.cancelInvocation).toHaveBeenCalledWith({ invocationId: 'invocation-1' });
    expect(ordinary.disconnectUpdates).toHaveBeenCalledOnce();
    expect(workflow.sendWorkflowNodeMessage).not.toHaveBeenCalled();
  });
});

function ordinaryClient(): AgentSessionClient {
  return {
    createSession: vi.fn(async () => ({ id: 'session-1' }) as AgentSessionDto),
    listSessions: vi.fn(async () => [] as AgentSessionSummaryDto[]),
    loadSession: vi.fn(async () => ({}) as AgentSessionDetailsDto),
    reloadSession: vi.fn(async () => ({}) as AgentSessionDetailsDto),
    subscribeUpdates: vi.fn(async () => () => undefined),
    sendMessage: vi.fn(async () => ({ sessionId: 'session-1', invocationId: 'invocation-1' })),
    cancelInvocation: vi.fn(async () => ({ id: 'invocation-1' }) as AgentInvocationDto),
    disconnectUpdates: vi.fn(async () => undefined),
  };
}
