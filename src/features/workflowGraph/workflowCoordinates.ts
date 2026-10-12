export function snapWorkflowPoint(
  x: number,
  y: number,
  bounds?: { readonly width: number; readonly height: number },
  snap = true,
): { readonly x: number; readonly y: number } {
  const maximumX = Math.max(
    WORKFLOW_GRAPH_GRID_SIZE,
    (bounds?.width ?? Infinity) - WORKFLOW_GRAPH_NODE_WIDTH - WORKFLOW_GRAPH_GRID_SIZE,
  );
  const maximumY = Math.max(
    WORKFLOW_GRAPH_GRID_SIZE,
    (bounds?.height ?? Infinity) - WORKFLOW_GRAPH_NODE_HEIGHT - WORKFLOW_GRAPH_GRID_SIZE,
  );
  const clampedX = clamp(x, WORKFLOW_GRAPH_GRID_SIZE, maximumX);
  const clampedY = clamp(y, WORKFLOW_GRAPH_GRID_SIZE, maximumY);
  if (!snap) return { x: clampedX, y: clampedY };
  return {
    x: clamp(
      Math.round(clampedX / WORKFLOW_GRAPH_GRID_SIZE) * WORKFLOW_GRAPH_GRID_SIZE,
      WORKFLOW_GRAPH_GRID_SIZE,
      maximumX,
    ),
    y: clamp(
      Math.round(clampedY / WORKFLOW_GRAPH_GRID_SIZE) * WORKFLOW_GRAPH_GRID_SIZE,
      WORKFLOW_GRAPH_GRID_SIZE,
      maximumY,
    ),
  };
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(value, maximum));
}
import {
  WORKFLOW_GRAPH_GRID_SIZE,
  WORKFLOW_GRAPH_NODE_HEIGHT,
  WORKFLOW_GRAPH_NODE_WIDTH,
} from './workflowGraphModel';
