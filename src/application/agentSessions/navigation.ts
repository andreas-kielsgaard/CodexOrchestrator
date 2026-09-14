import { applyNavigationOrder, type NavigationOrderItem } from './navigationOrder';
import type { AgentSessionSummaryDto } from './contracts';
import type {
  SessionFolderTarget,
  SessionNavigationData,
  SessionPlacement,
  WorkflowSessionOwner,
} from './organization';
export type SessionNavigationSelection =
  | { readonly kind: 'initial' }
  | { readonly kind: 'session'; readonly sessionId: string }
  | {
      readonly kind: 'draft';
      readonly draftId: string;
      readonly folderTarget: SessionFolderTarget | null;
    };
export const sessionIdOf = (selection: SessionNavigationSelection): string | null =>
  selection.kind === 'session' ? selection.sessionId : null;
export const selectionKey = (selection: SessionNavigationSelection): string =>
  selection.kind === 'session'
    ? selection.sessionId
    : selection.kind === 'draft'
      ? selection.draftId
      : 'initial';
export interface SessionNavigationRow {
  readonly kind: 'session';
  readonly id: string;
  readonly summary: AgentSessionSummaryDto;
  readonly pinned: boolean;
  readonly owner?: WorkflowSessionOwner;
  readonly ownerLabel?: string;
  readonly group?: 'added' | 'owned';
}
export interface SessionNavigationFolder {
  readonly kind: 'folder';
  readonly role: 'repository' | 'section' | 'workflow';
  readonly order: NavigationOrderItem;
  readonly id: string;
  readonly label: string;
  readonly createTarget: SessionFolderTarget | null;
  readonly placement: SessionPlacement;
  readonly children: readonly SessionNavigationNode[];
}
export type SessionNavigationNode = SessionNavigationFolder | SessionNavigationRow;
export interface SessionNavigationSection {
  readonly id: string;
  readonly label: string;
  readonly children: readonly SessionNavigationNode[];
  readonly unlimited?: boolean;
}
export interface SessionNavigationModel {
  readonly sections: readonly SessionNavigationSection[];
  readonly sessions: ReadonlyMap<string, SessionNavigationRow>;
  readonly destinations: readonly {
    readonly label: string;
    readonly placement: SessionPlacement;
  }[];
}
export const targetId = (placement: SessionPlacement): string =>
  placement.kind === 'repository'
    ? `repo:${placement.repositoryId}:sessions`
    : placement.kind === 'workflow_instance'
      ? `instance:${placement.instanceId}`
      : 'unfiled';
export function buildSessionNavigation(data: SessionNavigationData): SessionNavigationModel {
  const organization = new Map(data.organization.map((item) => [item.sessionId, item]));
  const owners = new Map(data.owners.map((item) => [item.sessionId, item]));
  const instances = new Map(data.instances.map((item) => [item.id, item]));
  const groups = new Map<string, SessionNavigationRow[]>();
  const sessions = new Map<string, SessionNavigationRow>();
  const validFolders = new Set([
    'unfiled',
    ...data.repositories.map((repo) => `repo:${repo.id}:sessions`),
    ...data.instances
      .filter((i) => data.repositories.some((r) => r.id === i.repositoryId))
      .map((i) => `instance:${i.id}`),
  ]);
  const sorted = [...data.summaries].sort(
    (a, b) => b.updatedAt.localeCompare(a.updatedAt) || a.id.localeCompare(b.id),
  );
  for (const summary of sorted) {
    const metadata = organization.get(summary.id);
    const owner = owners.get(summary.id);
    const placement = metadata?.placement ?? { kind: 'default' };
    let folder =
      placement.kind === 'default' && owner ? `instance:${owner.instanceId}` : targetId(placement);
    if (!validFolders.has(folder)) folder = 'unfiled';
    const ownFolder = owner && folder === `instance:${owner.instanceId}`;
    const row: SessionNavigationRow = {
      kind: 'session',
      id: `${folder}:session:${summary.id}`,
      summary,
      pinned: Boolean(metadata?.pinnedAt),
      owner,
      ownerLabel: owner
        ? `${instances.get(owner.instanceId)?.name ?? 'Workflow'} · ${owner.nodeName}`
        : undefined,
      group: folder.startsWith('instance:') ? (ownFolder ? 'owned' : 'added') : undefined,
    };
    sessions.set(summary.id, row);
    const group = groups.get(folder) ?? [];
    group.push(row);
    groups.set(folder, group);
  }
  for (const [folderId, rows] of groups)
    groups.set(
      folderId,
      applyNavigationOrder(rows, (r) => r.summary.id, { kind: 'sessions', folderId }, data.orders),
    );
  const destinations: { label: string; placement: SessionPlacement }[] = [
    { label: 'Unfiled', placement: { kind: 'unfiled' } },
  ];
  const repoFolders: SessionNavigationFolder[] = applyNavigationOrder(
    [...data.repositories].sort((a, b) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id)),
    (r) => r.id,
    { kind: 'repositories' },
    data.orders,
  ).map((repo) => {
    const target: SessionFolderTarget = { kind: 'repository', repositoryId: repo.id };
    destinations.push({ label: repo.name, placement: target });
    const workflowFolders: SessionNavigationFolder[] = applyNavigationOrder(
      data.instances
        .filter((i) => i.repositoryId === repo.id)
        .sort((a, b) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id)),
      (i) => i.id,
      { kind: 'workflows', repositoryId: repo.id },
      data.orders,
    ).map((instance) => {
      const instanceTarget: SessionFolderTarget = {
        kind: 'workflow_instance',
        instanceId: instance.id,
      };
      destinations.push({
        label: `${repo.name} / ${instance.name}`,
        placement: instanceTarget,
      });
      const rows = groups.get(`instance:${instance.id}`) ?? [];
      return {
        kind: 'folder',
        role: 'workflow',
        order: { scope: { kind: 'workflows', repositoryId: repo.id }, id: instance.id },
        id: `instance:${instance.id}`,
        label: instance.name,
        createTarget: instanceTarget,
        placement: instanceTarget,
        children: [
          ...rows.filter((r) => r.group === 'added'),
          ...rows.filter((r) => r.group === 'owned'),
        ],
      };
    });
    return {
      kind: 'folder',
      role: 'repository',
      order: { scope: { kind: 'repositories' }, id: repo.id },
      id: `repo:${repo.id}`,
      label: repo.name,
      createTarget: target,
      placement: target,
      children: applyNavigationOrder<SessionNavigationFolder>(
        [
          {
            kind: 'folder',
            role: 'section',
            order: { scope: { kind: 'sections', repositoryId: repo.id }, id: 'sessions' },
            id: `repo:${repo.id}:sessions`,
            label: 'Sessions',
            createTarget: target,
            placement: target,
            children: groups.get(`repo:${repo.id}:sessions`) ?? [],
          },
          {
            kind: 'folder',
            role: 'section',
            order: { scope: { kind: 'sections', repositoryId: repo.id }, id: 'workflows' },
            id: `repo:${repo.id}:workflows`,
            label: 'Workflows',
            createTarget: target,
            placement: target,
            children: workflowFolders,
          },
        ],
        (n) => n.order.id,
        { kind: 'sections', repositoryId: repo.id },
        data.orders,
      ),
    };
  });
  const pinned = [...sessions.values()]
    .filter((s) => s.pinned)
    .sort(
      (a, b) =>
        organization
          .get(b.summary.id)!
          .pinnedAt!.localeCompare(organization.get(a.summary.id)!.pinnedAt!) ||
        a.summary.id.localeCompare(b.summary.id),
    )
    .map((s) => ({ ...s, id: `pinned:session:${s.summary.id}`, group: undefined }));
  return {
    sessions,
    destinations,
    sections: [
      {
        id: 'pinned',
        label: 'Pinned',
        children: applyNavigationOrder(
          pinned,
          (r) => r.summary.id,
          { kind: 'pinned' },
          data.orders,
        ),
        unlimited: true,
      },
      { id: 'repositories', label: 'Repositories', children: repoFolders },
      { id: 'unfiled', label: 'Unfiled', children: groups.get('unfiled') ?? [] },
    ],
  };
}
