import { fireEvent, render, screen } from '@testing-library/react';
import { WorkflowCanvas } from './WorkflowCanvas';

function renderCanvas(onPlace = vi.fn()) {
  render(
    <WorkflowCanvas
      nodes={[]}
      connections={[]}
      selection={{ kind: 'node', id: null }}
      canAdd
      onSelect={vi.fn()}
      onPlace={onPlace}
      onConnect={vi.fn()}
      onMove={vi.fn()}
      onRemove={vi.fn()}
      onEntryNode={vi.fn()}
      onConnectionGroup={vi.fn()}
    />,
  );
  return onPlace;
}

it('previews and commits a grid-snapped node at the same coordinates', () => {
  const place = renderCanvas();
  fireEvent.click(screen.getByRole('button', { name: 'Add node' }));
  const canvas = screen.getByRole('region', { name: 'Workflow canvas' });
  fireEvent(canvas, new MouseEvent('pointermove', { bubbles: true, clientX: 107, clientY: 113 }));
  const preview = screen.getByText('New node').closest('.workflow-node');
  expect(canvas).toHaveClass('is-grid-visible');
  expect(preview).toHaveStyle({ left: '100px', top: '120px', width: '220px', height: '100px' });
  fireEvent.click(canvas, { clientX: 107, clientY: 113 });
  expect(place).toHaveBeenCalledWith(100, 120, undefined);
});

it('uses unsnapped placement coordinates while Alt is held', () => {
  const place = renderCanvas();
  fireEvent.click(screen.getByRole('button', { name: 'Add node' }));
  const canvas = screen.getByRole('region', { name: 'Workflow canvas' });
  fireEvent(
    canvas,
    new MouseEvent('pointermove', {
      bubbles: true,
      clientX: 107,
      clientY: 113,
      altKey: true,
    }),
  );
  expect(screen.getByText('New node').closest('.workflow-node')).toHaveStyle({
    left: '107px',
    top: '113px',
  });
  fireEvent.click(canvas, { clientX: 107, clientY: 113, altKey: true });
  expect(place).toHaveBeenCalledWith(107, 113, undefined);
});
