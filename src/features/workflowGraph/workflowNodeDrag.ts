export interface WorkflowNodeDragState {
  readonly nodeId: string;
  readonly pointerId: number;
  readonly originX: number;
  readonly originY: number;
  readonly pointerX: number;
  readonly pointerY: number;
}

export interface WorkflowNodeDragBounds {
  readonly width: number;
  readonly height: number;
}

export interface WorkflowNodeDragPreview {
  readonly nodeId: string;
  readonly positionX: number;
  readonly positionY: number;
  readonly moved: boolean;
}

export const WORKFLOW_GRAPH_GRID_SIZE = 20;
const MOVE_THRESHOLD = 4;
const NODE_WIDTH_WITH_GAP = 234;
const NODE_HEIGHT_WITH_GAP = 120;

export function beginWorkflowNodeDrag(input: {
  readonly nodeId: string;
  readonly pointerId: number;
  readonly positionX: number;
  readonly positionY: number;
  readonly clientX: number;
  readonly clientY: number;
}): WorkflowNodeDragState {
  return {
    nodeId: input.nodeId,
    pointerId: input.pointerId,
    originX: input.positionX,
    originY: input.positionY,
    pointerX: input.clientX,
    pointerY: input.clientY,
  };
}

export function projectWorkflowNodeDrag(
  state: WorkflowNodeDragState,
  clientX: number,
  clientY: number,
  bounds: WorkflowNodeDragBounds,
  snap = true,
): WorkflowNodeDragPreview {
  const deltaX = clientX - state.pointerX;
  const deltaY = clientY - state.pointerY;
  const point = snapWorkflowPoint(
    state.originX + deltaX,
    state.originY + deltaY,
    bounds,
    snap,
  );
  return {
    nodeId: state.nodeId,
    positionX: point.x,
    positionY: point.y,
    moved: Math.hypot(deltaX, deltaY) >= MOVE_THRESHOLD,
  };
}

export function snapWorkflowPoint(
  x: number,
  y: number,
  bounds?: WorkflowNodeDragBounds,
  snap = true,
): { readonly x: number; readonly y: number } {
  const maximumX = Math.max(WORKFLOW_GRAPH_GRID_SIZE, (bounds?.width ?? Infinity) - NODE_WIDTH_WITH_GAP);
  const maximumY = Math.max(WORKFLOW_GRAPH_GRID_SIZE, (bounds?.height ?? Infinity) - NODE_HEIGHT_WITH_GAP);
  const clampedX = clamp(x, WORKFLOW_GRAPH_GRID_SIZE, maximumX);
  const clampedY = clamp(y, WORKFLOW_GRAPH_GRID_SIZE, maximumY);
  if (!snap) return { x: clampedX, y: clampedY };
  return {
    x: clamp(Math.round(clampedX / WORKFLOW_GRAPH_GRID_SIZE) * WORKFLOW_GRAPH_GRID_SIZE, WORKFLOW_GRAPH_GRID_SIZE, maximumX),
    y: clamp(Math.round(clampedY / WORKFLOW_GRAPH_GRID_SIZE) * WORKFLOW_GRAPH_GRID_SIZE, WORKFLOW_GRAPH_GRID_SIZE, maximumY),
  };
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(value, maximum));
}
