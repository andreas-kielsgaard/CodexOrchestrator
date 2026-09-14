import { applyNavigationOrder, insertNavigationSibling } from './navigationOrder';
import { buildSessionNavigation } from './navigation';
import { navigationFolders } from './navigationView';
import { navigationData } from '../../features/agentSessions/navigationTestFixtures';

it('retains saved sibling order and appends new siblings deterministically', () => {
  expect(
    applyNavigationOrder(['a', 'b', 'c'], (id) => id, { kind: 'repositories' }, [
      { scope: { kind: 'repositories' }, orderedIds: ['b', 'gone', 'a'] },
    ]),
  ).toEqual(['b', 'a', 'c']);
  expect(insertNavigationSibling(['a', 'b', 'c'], 'a', 'b', 'after')).toEqual(['b', 'a', 'c']);
  expect(insertNavigationSibling(['a', 'b', 'c'], 'c', 'a', 'before')).toEqual(['c', 'a', 'b']);
});

it('applies each ordering scope without changing session placement or workflow ownership', () => {
  const data = navigationData();
  const base = buildSessionNavigation(data);
  const reordered = buildSessionNavigation({
    ...data,
    instances: [...data.instances, { ...data.instances[0], id: 'flow-b', name: 'Other flow' }],
    orders: [
      { scope: { kind: 'repositories' }, orderedIds: ['repo-b', 'repo-a'] },
      {
        scope: { kind: 'sections', repositoryId: 'repo-a' },
        orderedIds: ['workflows', 'sessions'],
      },
      { scope: { kind: 'workflows', repositoryId: 'repo-a' }, orderedIds: ['flow-b', 'flow-a'] },
    ],
  });
  const folders = navigationFolders(reordered);
  expect(folders.filter((f) => !f.parentId).map((f) => f.node.order.id)).toEqual([
    'repo-b',
    'repo-a',
  ]);
  expect(folders.filter((f) => f.parentId === 'repo:repo-a').map((f) => f.node.order.id)).toEqual([
    'workflows',
    'sessions',
  ]);
  expect(
    folders.filter((f) => f.parentId === 'repo:repo-a:workflows').map((f) => f.node.order.id),
  ).toEqual(['flow-b', 'flow-a']);
  expect(reordered.sessions).toEqual(base.sessions);
});
