import { layoutId } from './branchLayout';
import { buildSessionNavigation } from './navigation';
import { navigationFolders } from './navigationView';
import { navigationData } from '../../features/agentSessions/navigationTestFixtures';
const branches = [
  {
    id: 'branch:old',
    repositoryId: 'repo-a',
    branchRef: 'refs/heads/old',
    createdAt: '2026-09-01',
    paths: ['C:/old'],
  },
  {
    id: 'branch:new',
    repositoryId: 'repo-a',
    branchRef: 'refs/heads/new',
    createdAt: '2026-10-01',
    paths: ['C:/new', '/remote/new'],
  },
];
const data = () => ({
  ...navigationData(),
  branches,
  sessionBranches: [
    { sessionId: 'session-1', branchId: 'branch:old' },
    { sessionId: 'session-2', branchId: 'branch:new' },
  ],
});
function contents(model: ReturnType<typeof buildSessionNavigation>) {
  return navigationFolders(model).find((item) => item.node.id === 'repo:repo-a:sessions')!.node
    .children;
}
it('groups only instantiated branches, newest-created first, above other conversations', () => {
  const rows = contents(buildSessionNavigation(data()));
  expect(rows.slice(0, 2).map(layoutId)).toEqual(['branch:new', 'branch:old']);
  expect(rows[0]).toMatchObject({ children: [{ summary: { id: 'session-2' } }] });
  expect(rows[2]).toMatchObject({ summary: { id: 'session-3' } });
});
it('persists detached conversations between branch groups and preserves their affinity', () => {
  const rows = contents(
    buildSessionNavigation({
      ...data(),
      orders: [
        {
          scope: { kind: 'session_layout', folderId: 'repo:repo-a:sessions' },
          orderedIds: ['branch:old', 'session:session-2', 'branch:new'],
        },
      ],
    }),
  );
  expect(rows.slice(0, 3).map(layoutId)).toEqual(['branch:old', 'session:session-2', 'branch:new']);
  expect(rows[1]).toMatchObject({ branch: { id: 'branch:new' } });
  expect(rows[2]).toMatchObject({ children: [] });
});

it('moves conversations with branch affinity while retaining explicit detachment', () => {
  const initial = data();
  const changed = {
    ...initial,
    sessionBranches: initial.sessionBranches.map((item) => ({ ...item, branchId: 'branch:new' })),
  };
  const grouped = contents(buildSessionNavigation(changed));
  expect(grouped[0]).toMatchObject({
    children: [{ summary: { id: 'session-1' } }, { summary: { id: 'session-2' } }],
  });
  const detached = contents(
    buildSessionNavigation({
      ...changed,
      orders: [
        {
          scope: { kind: 'session_layout', folderId: 'repo:repo-a:sessions' },
          orderedIds: ['branch:old', 'session:session-1', 'branch:new'],
        },
      ],
    }),
  );
  expect(detached[1]).toMatchObject({
    kind: 'session',
    summary: { id: 'session-1' },
    branch: { id: 'branch:new' },
  });
  expect(detached[2]).toMatchObject({ children: [{ summary: { id: 'session-2' } }] });
});

it('preserves explicit placement in another repository after a branch switch', () => {
  const initial = data();
  const model = buildSessionNavigation({
    ...initial,
    organization: initial.organization.map((item) =>
      item.sessionId === 'session-1'
        ? { ...item, placement: { kind: 'repository', repositoryId: 'repo-b' } }
        : item,
    ),
  });
  const folder = navigationFolders(model).find(
    (item) => item.node.id === 'repo:repo-b:sessions',
  )!.node;
  expect(folder.children).toMatchObject([{ kind: 'session', summary: { id: 'session-1' } }]);
});
