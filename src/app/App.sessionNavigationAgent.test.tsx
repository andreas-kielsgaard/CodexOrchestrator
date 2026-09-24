import { repairClients, repairRecipe } from '../features/workflowAuthoring/testFixtures';
import type {
  WorkflowInstanceClient,
  WorkflowInstanceDetails,
} from '../application/workflowInstances';
import { fireEvent, act, render, screen, waitFor } from '@testing-library/react';
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
  const workflow = repairClients();
  const details: WorkflowInstanceDetails = {
    instance: {
      id: 'flow-a',
      name: 'Feature build',
      recipe: {
        ...repairRecipe().draft,
        nodes: repairRecipe().draft.nodes.map((n, i) =>
          i ? n : { ...n, nodeId: 'worker', name: 'Worker' },
        ),
      },
      createdAt: '2026-09-14',
      target: {
        repository: { id: 'repo-a', name: 'Alpha', gitCommonDirectory: 'C:/repo/.git' },
        branch: { id: 'main', name: 'main' },
        worktree: { id: 'main', path: 'C:/repo' },
      },
    },
    sessions: [],
    attempts: [],
  };
  const instanceClient: WorkflowInstanceClient = {
    list: async () => [details.instance],
    load: async () => details,
    create: async () => details.instance,
    messageNode: async () => {
      throw new Error('Not used');
    },
  };
  const loadSession = vi.spyOn(fixture.sessions, 'loadSession');
  render(
    <App
      orchestrationClient={{
        load: async () => ({ kind: 'unavailable', reason: 'Navigation fixture' }),
      }}
      workflowAuthoringClient={workflow.authoring}
      workflowInstanceClient={instanceClient}
      executionConfigurationClient={workflow.configuration}
      agentSessionProfileClient={fixture.profiles}
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
    await waitFor(() => expect(complete).toHaveBeenCalledWith(id, expect.any(Object), null), {
      timeout: 15_000,
    });
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
  const group = await run({
    kind: 'set_group_expanded',
    groupId: 'instance:flow-a:owned',
    expanded: false,
  });
  expect(group.groups.find((g) => g.id === 'instance:flow-a:owned')?.expanded).toBe(false);
  expect(group.visibleRows.some((r) => r.sessionId === 'session-7')).toBe(false);
  await run({ kind: 'pin_session', sessionId: 'session-2', pinned: true });
  const ordered = await run({
    kind: 'reorder_navigation',
    scope: { kind: 'pinned' },
    orderedIds: ['session-2', 'session-1'],
  });
  expect(ordered.orders.find((o) => o.scope.kind === 'pinned')?.orderedIds).toEqual([
    'session-2',
    'session-1',
  ]);
  await run({ kind: 'set_folder_expanded', folderId: 'repo:repo-a', expanded: false });
  const shortcut = await run({ kind: 'open_session', sessionId: 'session-1', source: 'pinned' });
  expect(shortcut.folders.find((f) => f.id === 'repo:repo-a')?.expanded).toBe(false);
  const opened = await run({ kind: 'open_session_workflow', sessionId: 'session-7' });
  expect(opened.openedWorkflow).toEqual({
    instanceId: 'flow-a',
    session: { nodeId: 'worker', sessionId: 'session-7' },
  });
  expect(await screen.findByRole('button', { name: 'Back to node' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Open Worker' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await waitFor(() => expect(loadSession).toHaveBeenCalledWith({ sessionId: 'session-7' }));
  fireEvent.click(screen.getByRole('button', { name: /^Feature build Review workflow/ }));
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Back to node' })).toBeNull());
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(await screen.findByRole('button', { name: 'Back to node' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Back to node' }));
  expect(screen.queryByRole('button', { name: 'Back to node' })).toBeNull();
  await run({ kind: 'open_workflow', instanceId: 'flow-a' });
  expect(await screen.findByRole('heading', { name: 'Feature build' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Back to node' })).toBeNull();
});
