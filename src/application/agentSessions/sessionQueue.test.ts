import { describe, expect, it } from 'vitest';
import { projectSessionQueueTargets, type SessionQueueViewDto } from './sessionQueue';
import type { SessionExecutionTargetDto } from '../executionTargets/contracts';

const local: SessionExecutionTargetDto = {
  capabilityProfileId: 'local-profile',
  capabilityProfileRevision: 1,
  execution: {
    deviceId: 'local',
    deviceName: 'Laptop',
    provider: 'codex',
    configurationRef: 'local',
    connection: { kind: 'local' },
  },
  repositoryId: 'repo',
  branchRef: 'refs/heads/main',
  worktreeId: 'local-tree',
  path: 'C:\\repo',
  head: 'a'.repeat(40),
};
const remote: SessionExecutionTargetDto = {
  ...local,
  capabilityProfileId: 'remote-profile',
  execution: {
    deviceId: 'server',
    deviceName: 'Server',
    provider: 'codex',
    configurationRef: 'remote',
    connection: { kind: 'ssh', target: 'server', hostExecutable: 'orchid-host' },
  },
  worktreeId: 'remote-tree',
  path: '/repo',
};

function queue(operationState: 'pending' | 'completed' | 'canceled'): SessionQueueViewDto {
  return {
    sessionId: 'session',
    revision: 1,
    activeTarget: local,
    entries: [
      {
        entry: {
          id: 'switch',
          sessionId: 'session',
          ordinal: 0,
          state: operationState,
          item: { kind: 'target_operation', operationId: 'operation' },
          blockedReason: null,
          createdAt: '',
          updatedAt: '',
        },
        message: null,
        targetOperation: {
          id: 'operation',
          sessionId: 'session',
          sourceTarget: local,
          destinationSelection: {
            capabilityProfileId: remote.capabilityProfileId,
            capabilityProfileRevision: 1,
            execution: remote.execution,
            workspace: { kind: 'existing', target: remote },
          },
          phase: operationState,
          actions: [],
          stagedTarget: operationState === 'completed' ? remote : null,
          error: null,
          generation: 0,
          createdAt: '',
          updatedAt: '',
        },
      },
      {
        entry: {
          id: 'message',
          sessionId: 'session',
          ordinal: 1,
          state: 'pending',
          item: { kind: 'message', invocationId: 'invocation' },
          blockedReason: null,
          createdAt: '',
          updatedAt: '',
        },
        message: null,
        targetOperation: null,
      },
    ],
  };
}

describe('projectSessionQueueTargets', () => {
  it('applies a target operation to the following message', () => {
    expect(projectSessionQueueTargets(queue('pending'))[1].target).toMatchObject({
      execution: { deviceId: 'server' },
    });
  });

  it('does not apply a canceled operation', () => {
    expect(projectSessionQueueTargets(queue('canceled'))[1].target).toMatchObject({
      execution: { deviceId: 'local' },
    });
  });

  it('starts from the resolved active target after a completed operation', () => {
    const completed = { ...queue('completed'), activeTarget: remote };
    expect(projectSessionQueueTargets(completed)[1].target).toBe(remote);
  });
});
