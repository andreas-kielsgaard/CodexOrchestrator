import { act, render, screen, waitFor } from '@testing-library/react';
import { App } from './App';
import type {
  SessionNavigationAgentAccess,
  SessionNavigationCommand,
  SessionNavigationCommandRequest,
  SessionNavigationState,
} from '../application/agentSessions/agentAccess';
import { repairSessionClients } from '../features/agentSessions/profileTestFixtures';
import { recordedNavigation } from '../features/agentSessions/navigationTestFixtures';

it('agent commands navigate the mounted UI and share its folders, disclosure and organization actions', async () => {
  let receive!: (request: SessionNavigationCommandRequest) => void;
  const complete = vi.fn<SessionNavigationAgentAccess['complete']>(async () => {});
  const source: SessionNavigationAgentAccess = {
    subscribe: async (listener) => {
      receive = listener;
      return () => {};
    },
    complete,
  };
  const fixture = repairSessionClients();
  render(
    <App
      orchestrationClient={{
        load: async () => ({ kind: 'unavailable', reason: 'Navigation fixture' }),
      }}
      agentSessionClient={fixture.sessions}
      sessionNavigationClient={recordedNavigation()}
      sessionNavigationAgent={source}
    />,
  );
  await waitFor(() => expect(receive).toBeDefined());
  let sequence = 0;
  const run = async (command: SessionNavigationCommand) => {
    const id = String(++sequence);
    act(() => receive({ id, command }));
    await waitFor(() => expect(complete).toHaveBeenCalledWith(id, expect.any(Object), null));
    return complete.mock.calls.find((call) => call[0] === id)![1] as SessionNavigationState;
  };
  const initial = await run({ kind: 'inspect' });
  expect(screen.getByRole('navigation', { name: 'Session list' })).toBeVisible();
  expect(
    initial.visibleRows.filter((row) => row.sessionId && row.id.startsWith('repo:repo-a:')),
  ).toHaveLength(5);
  expect(initial.folders.find((folder) => folder.id === 'repo:repo-b')).toBeDefined();
  const reordered = await run({
    kind: 'reorder_navigation',
    scope: { kind: 'sections', repositoryId: 'repo-a' },
    orderedIds: ['workflows', 'sessions'],
  });
  expect(reordered.folders.filter((f) => f.parentId === 'repo:repo-a').map((f) => f.id)).toEqual([
    'repo:repo-a:workflows',
    'repo:repo-a:sessions',
  ]);
  expect(reordered.folders.find((f) => f.id === 'instance:flow-a')).toMatchObject({
    role: 'workflow',
    parentId: 'repo:repo-a:workflows',
    orderedSiblingIds: ['flow-a'],
  });
  const revealed = await run({ kind: 'open_session', sessionId: 'session-6' });
  expect(revealed.selection).toEqual({ kind: 'session', sessionId: 'session-6' });
  expect(revealed.visibleRows.some((row) => row.sessionId === 'session-6')).toBe(true);
  const collapsed = await run({
    kind: 'set_folder_expanded',
    folderId: 'repo:repo-a',
    expanded: false,
  });
  expect(collapsed.visibleRows.some((row) => row.sessionId === 'session-6')).toBe(false);
  await run({ kind: 'set_folder_expanded', folderId: 'repo:repo-a', expanded: true });
  const draft = await run({
    kind: 'new_session',
    folderTarget: { kind: 'workflow_instance', instanceId: 'flow-a' },
  });
  expect(draft.selection).toMatchObject({
    kind: 'draft',
    folderTarget: { kind: 'workflow_instance', instanceId: 'flow-a' },
  });
  const pinned = await run({ kind: 'pin_session', sessionId: 'session-1', pinned: true });
  expect(pinned.selection).toEqual(draft.selection);
  expect(pinned.visibleRows.filter((row) => row.sessionId === 'session-1')).toHaveLength(2);
  const moved = await run({
    kind: 'move_session',
    sessionId: 'session-1',
    placement: { kind: 'workflow_instance', instanceId: 'flow-a' },
  });
  expect(moved.selection).toEqual(draft.selection);
  expect(moved.sessions.find((session) => session.id === 'session-1')).toMatchObject({
    rowId: 'instance:flow-a:session:session-1',
    group: 'added',
  });
  expect(moved.sessions.find((session) => session.id === 'session-7')).toMatchObject({
    group: 'owned',
  });
  expect((await run({ kind: 'get_deeplink', sessionId: 'session-1' })).deeplink).toBe(
    'codex-orchestrator://sessions/session-1',
  );
});
