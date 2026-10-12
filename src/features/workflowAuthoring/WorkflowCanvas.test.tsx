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

it('commits a grid-snapped node from the canvas position', () => {
  const place = renderCanvas();
  fireEvent.click(screen.getByRole('button', { name: 'Add node' }));
  const canvas = screen.getByRole('region', { name: 'Workflow canvas' });
  fireEvent.click(canvas, { clientX: 107, clientY: 113 });
  expect(place).toHaveBeenCalledWith(100, 120, undefined);
});

it('uses unsnapped placement coordinates while Alt is held', () => {
  const place = renderCanvas();
  fireEvent.click(screen.getByRole('button', { name: 'Add node' }));
  const canvas = screen.getByRole('region', { name: 'Workflow canvas' });
  fireEvent.click(canvas, { clientX: 107, clientY: 113, altKey: true });
  expect(place).toHaveBeenCalledWith(107, 113, undefined);
});
