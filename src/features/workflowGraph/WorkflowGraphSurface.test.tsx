import { fireEvent, render, screen } from '@testing-library/react';
import { WorkflowGraphConnections, WorkflowGraphNodeCard, WorkflowGraphSurface } from '.';

it('shares selectable nodes and connections without owning feature behavior', () => {
  const onConnection = vi.fn();
  const onNode = vi.fn();
  const nodes = [
    { id: 'author', name: 'Author', x: 20, y: 30, starting: true },
    { id: 'reviewer', name: 'Reviewer', x: 330, y: 30 },
  ];
  const connections = [
    { id: 'review', name: 'Review handoff', source: 'author', destination: 'reviewer' },
    { id: 'revise', name: 'Revision request', source: 'reviewer', destination: 'author' },
  ];

  render(
    <WorkflowGraphSurface width={700} height={400} aria-label="Test flow">
      <WorkflowGraphConnections nodes={nodes} connections={connections} onActivate={onConnection} />
      <WorkflowGraphNodeCard node={nodes[0]} aria-label="Open Author" onClick={onNode}>
        <strong>Author</strong>
      </WorkflowGraphNodeCard>
    </WorkflowGraphSurface>,
  );

  fireEvent.click(screen.getByRole('button', { name: 'Open 2 connections' }));
  fireEvent.click(screen.getByRole('button', { name: 'Open Author' }));
  expect(onConnection).toHaveBeenCalledWith(['review', 'revise']);
  expect(onNode).toHaveBeenCalledOnce();
  const visibleLine = document.querySelector('.workflow-connection__visible');
  expect(visibleLine).toHaveAttribute('marker-start');
  expect(visibleLine).toHaveAttribute('marker-end');
});
