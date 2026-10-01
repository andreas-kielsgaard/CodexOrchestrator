import type { NavigationBranch } from './organization';
import type {
  SessionNavigationNode,
  SessionNavigationRow,
  SessionNavigationFolder,
} from './navigation';
import {
  applyNavigationOrder,
  type NavigationOrder,
  type NavigationOrderScope,
} from './navigationOrder';

export function branchColor(id: string) {
  let hash = 0;
  for (const char of id) hash = ((hash << 5) - hash + char.charCodeAt(0)) | 0;
  return `hsl(${Math.abs(hash) % 360} 52% 42%)`;
}
export const layoutId = (node: SessionNavigationNode) =>
  node.kind === 'folder' ? node.order.id : `session:${node.summary.id}`;

export function branchLayout(
  rows: readonly SessionNavigationRow[],
  branches: readonly NavigationBranch[],
  repositoryId: string,
  orders: readonly NavigationOrder[],
): SessionNavigationNode[] {
  const folderId = `repo:${repositoryId}:sessions`;
  const scope: NavigationOrderScope = { kind: 'session_layout', folderId };
  const detached = new Set(
    orders
      .find((order) => order.scope.kind === 'session_layout' && order.scope.folderId === folderId)
      ?.orderedIds.filter((id) => id.startsWith('session:')),
  );
  const groups: SessionNavigationFolder[] = [...branches]
    .filter((branch) => branch.repositoryId === repositoryId)
    .sort(
      (a, b) =>
        (b.createdAt ?? '').localeCompare(a.createdAt ?? '') ||
        a.branchRef.localeCompare(b.branchRef),
    )
    .map((branch) => ({
      kind: 'folder',
      role: 'branch',
      id: branch.id,
      label: branch.branchRef.replace(/^refs\/heads\//, ''),
      order: { scope, id: branch.id },
      branch,
      createTarget: null,
      placement: { kind: 'repository', repositoryId },
      children: rows
        .filter((row) => row.branch?.id === branch.id && !detached.has(`session:${row.summary.id}`))
        .map((row) => ({ ...row, branchGrouped: true })),
    }));
  return applyNavigationOrder<SessionNavigationNode>(
    [...groups, ...rows.filter((row) => !row.branch || detached.has(`session:${row.summary.id}`))],
    layoutId,
    scope,
    orders,
  );
}
