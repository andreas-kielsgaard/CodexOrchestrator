import { act, renderHook, waitFor } from '@testing-library/react';
import type { SessionFolderTarget } from '../../application/agentSessions/organization';
import { repairSessionClients } from './profileTestFixtures';
import { useAgentSession } from './useAgentSession';
import type { SessionExecutionTargetDto } from '../../application/executionTargets/contracts';
import { localExecutionBinding } from '../../application/executionTargets/contracts';
import { remoteTarget } from './sessionTargetFixtures';

it('discovers folder drafts independently and keeps saved session execution context after placement changes', async () => {
  const fixture = repairSessionClients(false);
  const load = vi.spyOn(fixture.profiles, 'loadQuickFeatures');
  const start = vi.spyOn(fixture.profiles, 'startDirectUserSession');
  const execution = {
    client: fixture.profiles,
    selection: { model: 'model-b', reasoningMode: 'medium' },
    setSelection: vi.fn(),
    afterAccepted: vi.fn(),
  };
  const repository: SessionFolderTarget = { kind: 'repository', repositoryId: 'repo-1' };
  const workflow: SessionFolderTarget = { kind: 'workflow_instance', instanceId: 'flow-1' };
  const { result, rerender } = renderHook(
    (props: {
      draftId: string;
      folderTarget: SessionFolderTarget;
      selectedSessionId: string | null;
    }) => useAgentSession(fixture.sessions, { ...props, execution }),
    {
      initialProps: {
        draftId: 'draft-1',
        folderTarget: repository as SessionFolderTarget,
        selectedSessionId: null as string | null,
      },
    },
  );
  await act(() => result.current.quickFeatures!.load());
  expect(load).toHaveBeenLastCalledWith({
    sessionId: null,
    workingDirectory: null,
    folderTarget: repository,
  });
  const firstContext = result.current.quickFeatures!.contextKey;
  rerender({ draftId: 'draft-2', folderTarget: workflow, selectedSessionId: null });
  expect(result.current.quickFeatures!.contextKey).not.toBe(firstContext);
  await act(() => result.current.quickFeatures!.load());
  expect(load).toHaveBeenLastCalledWith({
    sessionId: null,
    workingDirectory: null,
    folderTarget: workflow,
  });
  const secondContext = result.current.quickFeatures!.contextKey;
  rerender({ draftId: 'draft-3', folderTarget: workflow, selectedSessionId: null });
  expect(result.current.quickFeatures!.contextKey).not.toBe(secondContext);
  expect(start).not.toHaveBeenCalled();
  act(() => result.current.setDraft('Project discussion'));
  await act(() => result.current.send());
  expect(start).toHaveBeenCalledWith(
    expect.objectContaining({ folderTarget: workflow, model: 'model-b', reasoningMode: 'medium' }),
  );
  expect(execution.afterAccepted).toHaveBeenCalledOnce();
  rerender({ draftId: 'draft-3', folderTarget: repository, selectedSessionId: 'session-1' });
  await waitFor(() => expect(result.current.details?.session.id).toBe('session-1'));
  await act(() => result.current.quickFeatures!.load());
  expect(load).toHaveBeenLastCalledWith({
    sessionId: 'session-1',
    workingDirectory: fixture.details.session.workingDirectory,
  });
});

it('carries the draft target to quick discovery and first send while retaining navigation placement', async () => {
  const fixture = repairSessionClients(false);
  const load = vi.spyOn(fixture.profiles, 'loadQuickFeatures');
  const start = vi.spyOn(fixture.profiles, 'startDirectUserSession');
  const folderTarget: SessionFolderTarget = { kind: 'repository', repositoryId: 'repo-1' };
  const { result, rerender } = renderHook(
    ({ target }: { target: SessionExecutionTargetDto | null }) =>
      useAgentSession(fixture.sessions, {
        selectedSessionId: null,
        draftId: 'draft-target',
        folderTarget,
        execution: {
          client: fixture.profiles,
          target,
          selection: { model: null, reasoningMode: null },
          setSelection: vi.fn(),
          afterAccepted: vi.fn(),
        },
      }),
    { initialProps: { target: null as SessionExecutionTargetDto | null } },
  );
  await waitFor(() => expect(result.current.loading).toBe(false));
  act(() => result.current.setDraft('Keep my prompt'));
  const originalContext = result.current.quickFeatures!.contextKey;
  const localTarget = {
    ...remoteTarget,
    execution: localExecutionBinding,
    path: 'C:/worktrees/target',
  };
  rerender({ target: localTarget });
  expect(result.current.quickFeatures!.contextKey).not.toBe(originalContext);
  await act(() => result.current.quickFeatures!.load());
  expect(load).toHaveBeenLastCalledWith({
    sessionId: null,
    workingDirectory: localTarget.path,
    folderTarget,
    executionTarget: localTarget,
  });
  const localContext = result.current.quickFeatures!.contextKey;
  rerender({ target: remoteTarget });
  expect(result.current.quickFeatures!.contextKey).not.toBe(localContext);
  await act(() => result.current.quickFeatures!.load());
  expect(load).toHaveBeenLastCalledWith({
    sessionId: null,
    workingDirectory: remoteTarget.path,
    folderTarget,
    executionTarget: remoteTarget,
  });
  expect(result.current.draft).toBe('Keep my prompt');
  await act(() => result.current.send());
  expect(start).toHaveBeenCalledWith(
    expect.objectContaining({
      submittedText: 'Keep my prompt',
      workingDirectory: remoteTarget.path,
      folderTarget,
      executionTarget: remoteTarget,
    }),
  );
});
