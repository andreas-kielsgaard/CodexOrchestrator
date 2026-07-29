import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import type {
  AgentSessionNavigationFolder,
  AgentSessionNavigationModel,
  AgentSessionNavigationSession,
} from '../../application/agentSessionNavigation';
import { SessionSelector } from './SessionSelector';

describe('SessionSelector hierarchy', () => {
  it('renders titled sections and supports roving keyboard navigation across their trees', async () => {
    const selected: string[] = [];
    render(<Harness onSelect={(id) => selected.push(id)} />);
    const epicTree = screen.getByRole('tree', { name: 'Epics session hierarchy' });
    const epic = screen.getByRole('treeitem', { name: /Navigation Epic/ });

    expect(screen.getByRole('heading', { name: 'Epics' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Independent Sessions' })).toBeVisible();
    expect(screen.queryByRole('treeitem', { name: 'Independent Sessions' })).toBeNull();

    fireEvent.focus(epic);
    fireEvent.keyDown(epicTree, { key: 'ArrowRight' });
    expect(epic).toHaveAttribute('aria-expanded', 'true');
    fireEvent.keyDown(epicTree, { key: 'ArrowRight' });
    await waitFor(() =>
      expect(screen.getByRole('treeitem', { name: /Epic Runner Session/ })).toHaveFocus(),
    );
    fireEvent.keyDown(epicTree, { key: 'ArrowDown' });
    await waitFor(() =>
      expect(screen.getByRole('treeitem', { name: /Current scope/ })).toHaveFocus(),
    );
    fireEvent.keyDown(epicTree, { key: 'End' });
    await waitFor(() =>
      expect(screen.getByRole('treeitem', { name: /Research Session/ })).toHaveFocus(),
    );
    fireEvent.keyDown(
      screen.getByRole('tree', { name: 'Independent Sessions session hierarchy' }),
      { key: 'Enter' },
    );
    expect(selected).toEqual(['session-independent']);
  });

  it('keeps a selected Session recoverable when its containing folder collapses', async () => {
    render(<Harness selectedSessionId="session-plan-builder" />);
    const epic = screen.getByRole('treeitem', { name: /Navigation Epic/ });
    await waitFor(() => expect(epic).toHaveAttribute('aria-expanded', 'true'));
    const planningStep = screen.getByRole('treeitem', { name: /Current scope/ });
    await waitFor(() => expect(planningStep).toHaveAttribute('aria-expanded', 'true'));
    expect(screen.getByRole('treeitem', { name: /Plan Builder Session/ })).toHaveAttribute(
      'aria-selected',
      'true',
    );

    fireEvent.click(planningStep);

    expect(planningStep).toHaveAttribute('aria-expanded', 'false');
    expect(planningStep).toHaveFocus();
    expect(planningStep).toHaveAccessibleName(/Contains selected Session/);
    expect(screen.queryByRole('treeitem', { name: /Plan Builder Session/ })).toBeNull();

    fireEvent.click(planningStep);
    expect(screen.getByRole('treeitem', { name: /Plan Builder Session/ })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });

  it('shows identity, role, status, and responsive navigation controls', () => {
    render(<Harness selectedSessionId="session-plan-builder" />);
    expect(screen.getByText('Ada: Epic Plan Builder')).toBeVisible();
    expect(screen.getAllByText('Completed')).toHaveLength(2);
    expect(screen.getByText('Processing')).toBeVisible();

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
  const epicRunner = session('session-epic-runner', 'Epic Runner Session', 'completed');
  const planBuilder = session('session-plan-builder', 'Plan Builder Session', 'completed', {
    sessionId: 'session-plan-builder',
    agentName: 'Ada',
    harnessRole: 'Epic Plan Builder',
    visualIdentity: { token: 'leaf', accentColor: '#33664f' },
  });
  const independent = session('session-independent', 'Research Session', 'running');
  const planningStep: AgentSessionNavigationFolder = {
    kind: 'folder',
    id: 'planning-step',
    label: 'Current scope',
    children: [planBuilder],
  };
  const epic: AgentSessionNavigationFolder = {
    kind: 'folder',
    id: 'epic',
    label: 'Navigation Epic',
    children: [epicRunner, planningStep],
  };
  return {
    sections: [
      { kind: 'section', id: 'epics', label: 'Epics', children: [epic] },
      {
        kind: 'section',
        id: 'independent',
        label: 'Independent Sessions',
        children: [independent],
      },
    ],
    sessions: new Map([
      [epicRunner.summary.id, epicRunner],
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
