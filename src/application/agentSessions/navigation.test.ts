import { buildSessionNavigation } from './navigation';
import { navigationData } from '../../features/agentSessions/navigationTestFixtures';
import { visibleSessionRows, navigationFolders } from './navigationView';
it('groups registered repositories and typed workflow owners, leaving ordinary sessions unfiled', () => {
  const model = buildSessionNavigation(navigationData());
  expect(model.sections[1].children.map((n) => n.kind === 'folder' && n.label)).toEqual([
    'Alpha',
    'Empty repo',
  ]);
  expect(model.sessions.get('session-7')).toMatchObject({
    id: 'instance:flow-a:session:session-7',
    group: 'owned',
    ownerLabel: 'Feature build · Worker',
  });
  expect(model.sections[2].children).toHaveLength(1);
});
it('keeps ownership independent of moves and puts added sessions before owned sessions', () => {
  const data = navigationData();
  const model = buildSessionNavigation({
    ...data,
    organization: [
      {
        sessionId: 'session-1',
        placement: { kind: 'workflow_instance', instanceId: 'flow-a' },
        pinnedAt: null,
      },
      { sessionId: 'session-7', placement: { kind: 'unfiled' }, pinnedAt: null },
    ],
  });
  expect(model.sessions.get('session-1')).toMatchObject({ group: 'added' });
  expect(model.sessions.get('session-7')).toMatchObject({
    id: 'unfiled:session:session-7',
    owner: { instanceId: 'flow-a' },
  });
});
it('uses one five-row budget per folder and keeps all pinned shortcuts', () => {
  const data = navigationData();
  const model = buildSessionNavigation({
    ...data,
    organization: data.summaries.map((s) => ({
      sessionId: s.id,
      placement: { kind: 'workflow_instance', instanceId: 'flow-a' },
      pinnedAt: '2026-09-14T12:00:00Z',
    })),
  });
  const expanded = new Set([
    'repo:repo-a',
    'repo:repo-a:workflows',
    'repo:repo-a:sessions',
    'instance:flow-a',
    'instance:flow-a:added',
    'instance:flow-a:owned',
  ]);
  const rows = visibleSessionRows(model, expanded, new Set());
  expect(rows.filter((r) => r.sectionId === 'pinned')).toHaveLength(8);
  expect(
    rows.filter((r) => r.parentId?.startsWith('instance:flow-a:') && r.node.kind === 'session'),
  ).toHaveLength(5);
  expect(rows.find((r) => r.id === 'more:instance:flow-a')?.node).toMatchObject({ remaining: 3 });
  const all = visibleSessionRows(model, expanded, new Set(['instance:flow-a']));
  const workflow = all.filter(
    (r) => r.parentId?.startsWith('instance:flow-a:') && r.node.kind === 'session',
  );
  expect(workflow).toHaveLength(8);
  expect(workflow.at(-1)?.node).toMatchObject({ group: 'owned' });
  expect(new Set(all.map((r) => r.id)).size).toBe(all.length);
});

it('applies independent pinned and session orders while keeping owned sessions below added sessions', () => {
  const data = navigationData();
  const model = buildSessionNavigation({
    ...data,
    organization: data.summaries.map((s) => ({
      sessionId: s.id,
      placement: { kind: 'workflow_instance', instanceId: 'flow-a' },
      pinnedAt: '2026-09-14T12:00:00Z',
    })),
    orders: [
      { scope: { kind: 'pinned' }, orderedIds: ['session-8', 'session-1'] },
      {
        scope: { kind: 'sessions', folderId: 'instance:flow-a' },
        orderedIds: ['session-7', 'session-5', 'session-1'],
      },
    ],
  });
  expect(
    model.sections[0].children.slice(0, 2).map((n) => n.kind === 'session' && n.summary.id),
  ).toEqual(['session-8', 'session-1']);
  const workflow = navigationFolders(model).find((f) => f.node.id === 'instance:flow-a')!.node;
  expect(workflow.children.map((n) => n.kind === 'session' && n.summary.id)).toEqual([
    'session-5',
    'session-1',
    'session-2',
    'session-3',
    'session-4',
    'session-6',
    'session-8',
    'session-7',
  ]);
});
