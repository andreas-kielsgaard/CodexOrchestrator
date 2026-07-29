import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import type {
  AgentSessionNavigationFolder,
  AgentSessionNavigationModel,
  AgentSessionNavigationSession,
} from '../../application/agentSessionNavigation';
import { SessionSelector } from './SessionSelector';

describe('SessionSelector hierarchy', () => {
  it('supports roving tree keyboard navigation and stable expansion', async () => {
    const selected: string[] = [];
    render(<Harness onSelect={(id) => selected.push(id)} />);
    const tree = screen.getByRole('tree', { name: 'Session hierarchy' });
    const epic = screen.getByRole('treeitem', { name: /Navigation Epic/ });
    fireEvent.focus(epic);

    fireEvent.keyDown(tree, { key: 'ArrowRight' });
    expect(epic).toHaveAttribute('aria-expanded', 'true');
    fireEvent.keyDown(tree, { key: 'ArrowRight' });
    await waitFor(() =>
      expect(screen.getByRole('treeitem', { name: /Plan Builder Session/ })).toHaveFocus(),
    );
    fireEvent.keyDown(tree, { key: 'ArrowLeft' });
    await waitFor(() => expect(epic).toHaveFocus());
    fireEvent.keyDown(tree, { key: 'ArrowDown' });
    await waitFor(() =>
      expect(screen.getByRole('treeitem', { name: /Plan Builder Session/ })).toHaveFocus(),
    );
    fireEvent.keyDown(tree, { key: 'Enter' });
    expect(selected).toEqual(['session-plan-builder']);
    expect(epic).toHaveAttribute('aria-expanded', 'true');
  });

  it('shows identity, role, truthful invocation status, and a narrow navigation toggle', () => {
    render(<Harness selectedSessionId="session-plan-builder" />);
    expect(screen.getByText('Ada: Epic Plan Builder')).toBeVisible();
    expect(screen.getByText('Completed')).toBeVisible();
    const tree = screen.getByRole('tree', { name: 'Session hierarchy' });
    const independentFolder = screen.getByRole('treeitem', { name: /Independent Sessions/ });
    fireEvent.focus(independentFolder);
    fireEvent.click(independentFolder);
    expect(screen.getByText('Processing')).toBeVisible();
    fireEvent.keyDown(tree, { key: 'ArrowRight' });
    expect(screen.getByRole('treeitem', { name: /Research Session/ })).toHaveFocus();

    const toggle = screen.getByRole('button', { name: 'Hide sessions' });
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    fireEvent.click(toggle);
    expect(screen.getByRole('button', { name: 'Browse sessions' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
  });
});

function Harness({
  selectedSessionId = null,
  onSelect = () => undefined,
}: {
  readonly selectedSessionId?: string | null;
  readonly onSelect?: (id: string) => void;
}) {
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(() => new Set());
  return (
    <SessionSelector
      model={model()}
      selectedSessionId={selectedSessionId}
      expandedNodeIds={expanded}
      loading={false}
      onExpandedNodeIdsChange={setExpanded}
      onSelect={onSelect}
      onNew={() => undefined}
      onReload={() => undefined}
    />
  );
}

function model(): AgentSessionNavigationModel {
  const planBuilder = session('session-plan-builder', 'Plan Builder Session', 'completed', {
    sessionId: 'session-plan-builder',
    agentName: 'Ada',
    harnessRole: 'Epic Plan Builder',
    visualIdentity: { token: 'leaf', accentColor: '#33664f' },
  });
  const independent = session('session-independent', 'Research Session', 'running');
  const roots: AgentSessionNavigationFolder[] = [
    { kind: 'folder', id: 'epic', label: 'Navigation Epic', children: [planBuilder] },
    {
      kind: 'folder',
      id: 'independent',
      label: 'Independent Sessions',
      children: [independent],
    },
  ];
  return {
    roots,
    sessions: new Map([
      [planBuilder.summary.id, planBuilder],
      [independent.summary.id, independent],
    ]),
  };
}

function session(
  id: string,
  title: string,
  status: 'completed' | 'running',
  identity?: AgentSessionNavigationSession['identity'],
): AgentSessionNavigationSession {
  return {
    kind: 'session',
    id: `session:${id}`,
    summary: {
      id,
      title,
      availability: 'available',
      hasActiveInvocation: status === 'running',
      latestInvocationStatus: status,
      createdAt: '2026-07-29T09:00:00.000Z',
      updatedAt: '2026-07-29T09:00:00.000Z',
    },
    relationshipRoles: identity ? [] : ['Independent'],
    productLocations: [],
    ...(identity ? { identity } : {}),
  };
}
