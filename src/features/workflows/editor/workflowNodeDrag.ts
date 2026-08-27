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
): WorkflowNodeDragPreview {
  const deltaX = clientX - state.pointerX;
  const deltaY = clientY - state.pointerY;
  return {
    nodeId: state.nodeId,
    positionX: clamp(state.originX + deltaX, 24, Math.max(24, bounds.width - NODE_WIDTH_WITH_GAP)),
    positionY: clamp(
      state.originY + deltaY,
      28,
      Math.max(28, bounds.height - NODE_HEIGHT_WITH_GAP),
    ),
    moved: Math.hypot(deltaX, deltaY) >= MOVE_THRESHOLD,
  };
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(value, maximum));
}
