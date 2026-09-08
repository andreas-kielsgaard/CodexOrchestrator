import { describe, expect, it } from 'vitest';
import { beginWorkflowNodeDrag, projectWorkflowNodeDrag } from './workflowNodeDrag';

describe('workflow node drag geometry', () => {
  it('projects one pointer gesture from the original node position', () => {
    const drag = beginWorkflowNodeDrag({
      nodeId: 'node-1',
      pointerId: 7,
      positionX: 100,
      positionY: 120,
      clientX: 300,
      clientY: 300,
    });
    expect(projectWorkflowNodeDrag(drag, 360, 340, { width: 900, height: 700 })).toEqual({
      nodeId: 'node-1',
      positionX: 160,
      positionY: 160,
      moved: true,
    });
  });

  it('clamps previews inside the canvas and ignores click-sized movement', () => {
    const drag = beginWorkflowNodeDrag({
      nodeId: 'node-1',
      pointerId: 7,
      positionX: 30,
      positionY: 30,
      clientX: 300,
      clientY: 300,
    });
    expect(projectWorkflowNodeDrag(drag, 302, 301, { width: 900, height: 700 })).toMatchObject({
      moved: false,
    });
    expect(projectWorkflowNodeDrag(drag, -100, -100, { width: 900, height: 700 })).toMatchObject({
      positionX: 24,
      positionY: 28,
    });
  });
});
