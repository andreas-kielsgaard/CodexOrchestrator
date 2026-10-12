import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { WorkflowGraphCanvas } from './WorkflowGraphCanvas';
const nodes = [
  { id: 'a', name: 'Author', x: 40, y: 40 },
  { id: 'b', name: 'Reviewer', x: 400, y: 40 },
];
const connections = [
  { id: 'work', name: 'Review', source: 'a', destination: 'b' },
  { id: 'back', name: 'Revise', source: 'b', destination: 'a', role: 'feedback' as const },
];
it('selects both directions independently and retains layout through status updates', async () => {
  const select = vi.fn();
  const organize = vi.fn();
  const props = {
    nodes,
    connections,
    label: 'Test flow',
    nodeLabel: (n: (typeof nodes)[number]) => `Open ${n.name}`,
    onNodeActivate: vi.fn(),
    onConnectionActivate: select,
    onOrganize: organize,
  };
  const mounted = render(
    <WorkflowGraphCanvas {...props} renderNode={(n) => <strong>{n.name}</strong>} />,
  );
  fireEvent.click(screen.getByText('Connections · 2'));
  fireEvent.click(screen.getByRole('button', { name: 'Open Review in connection list' }));
  fireEvent.click(screen.getByRole('button', { name: 'Open Revise in connection list' }));
  expect(select.mock.calls).toEqual([[['work']], [['back']]]);
  fireEvent.click(screen.getByRole('button', { name: 'Organize' }));
  await waitFor(() => expect(organize).toHaveBeenCalledOnce());
  mounted.rerender(
    <WorkflowGraphCanvas
      {...props}
      renderNode={(n) => (
        <>
          <strong>{n.name}</strong>
          <span>Running</span>
        </>
      )}
    />,
  );
  expect(organize).toHaveBeenCalledOnce();
  expect(await screen.findByRole('button', { name: 'Open Author' })).toHaveTextContent('Running');
});
