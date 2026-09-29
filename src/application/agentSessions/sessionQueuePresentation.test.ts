import { describe, expect, it } from 'vitest';
import type { SessionQueueViewDto } from './sessionQueue';
import { presentSessionQueue } from './sessionQueuePresentation';

function queue(): SessionQueueViewDto {
  const execution = {
    deviceId: 'local',
    deviceName: 'Laptop',
    provider: 'codex',
    configurationRef: 'default',
    connection: { kind: 'local' as const },
  };
  const target = {
    capabilityProfileId: 'profile',
    capabilityProfileRevision: 1,
    execution,
    repositoryId: 'repo',
    branchRef: 'refs/heads/main',
    worktreeId: 'tree',
    path: 'C:\\repo',
    head: 'a'.repeat(40),
  };
  const preparation = {
    sessionId: 'session',
    phase: 'accepted' as const,
    selection: null,
    steps: [],
    error: null,
    canRetry: false,
    model: null,
    reasoningMode: null,
    sandboxMode: null,
  };
  const message = (id: string, state: 'pending' | 'executing' | 'blocked') => ({
    entry: {
      id: `entry-${id}`,
      sessionId: 'session',
      ordinal: 0,
      state,
      item: { kind: 'message' as const, invocationId: id },
      blockedReason: null,
      createdAt: '',
      updatedAt: '',
    },
    message: {
      invocation: {
        id,
        sessionId: 'session',
        submittedText: id,
        inputProvenance: 'user' as const,
        status: state === 'executing' ? ('running' as const) : ('pending' as const),
        requestedOptions: { model: null, sandbox: null },
        effectiveOptions: null,
        startedAt: null,
        completedAt: null,
        exitCode: null,
        signal: null,
        runtimeError: null,
        diagnostics: [],
        createdAt: '',
        updatedAt: '',
      },
      preparation: { ...preparation, invocationId: id },
    },
    targetOperation: null,
  });
  const operation = {
    id: 'operation',
    sessionId: 'session',
    sourceTarget: target,
    destinationSelection: {
      capabilityProfileId: 'profile',
      capabilityProfileRevision: 1,
      execution,
      workspace: { kind: 'existing' as const, target },
    },
    phase: 'staging' as const,
    actions: [],
    stagedTarget: null,
    error: null,
    generation: 0,
    createdAt: '',
    updatedAt: '',
  };
  return {
    sessionId: 'session',
    revision: 1,
    activeTarget: target,
    entries: [
      message('running', 'executing'),
      {
        entry: {
          id: 'target',
          sessionId: 'session',
          ordinal: 1,
          state: 'pending',
          item: { kind: 'target_operation' as const, operationId: 'operation' },
          blockedReason: null,
          createdAt: '',
          updatedAt: '',
        },
        message: null,
        targetOperation: operation,
      },
      message('future', 'pending'),
    ],
  };
}

describe('presentSessionQueue', () => {
  it('keeps executing messages in the transcript while presenting future entries only', () => {
    const result = presentSessionQueue(queue());
    expect(result.futureEntries.map(({ item }) => item.entry.id)).toEqual([
      'target',
      'entry-future',
    ]);
    expect(result.queuedInvocationIds).toEqual(new Set(['future']));
    expect(result.activeInvocationId).toBe('running');
  });

  it('projects staging target operations as both schedule boundaries and device activity', () => {
    const result = presentSessionQueue(queue());
    expect(result.futureEntries[0].item.entry.id).toBe('target');
    expect(result.deviceActivities.map(({ entry }) => entry.id)).toEqual(['target']);
  });
});
