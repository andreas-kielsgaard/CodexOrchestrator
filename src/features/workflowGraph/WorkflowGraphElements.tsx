import { useState, type ReactNode } from 'react';
import {
  Handle,
  Position,
  BaseEdge,
  EdgeLabelRenderer,
  useViewport,
  getSmoothStepPath,
  type Node,
  type NodeProps,
  type Edge,
  type EdgeProps,
} from '@xyflow/react';
import type { GraphLayout } from './workflowLayout';
export type CardData = Record<string, unknown> & {
  content: ReactNode;
  actions?: ReactNode;
  label: string;
  activate(): void;
  ports: {
    id: string;
    direction: 'in' | 'out';
    feedback: boolean;
    position?: { x: number; y: number };
  }[];
  move?(x: number, y: number): void;
};
export function WorkflowCard({
  data,
  selected,
  positionAbsoluteX,
  positionAbsoluteY,
}: NodeProps<Node<CardData>>) {
  const { zoom } = useViewport();
  return (
    <div
      className={`workflow-flow-card${selected ? ' is-selected' : ''}${zoom < 0.6 ? ' is-overview' : ''}`}
    >
      <div className="workflow-flow-card__drag" title="Drag to move node" />
      {data.ports.map((port, i) => (
        <Handle
          key={port.id}
          id={port.id}
          type={port.direction === 'in' ? 'target' : 'source'}
          position={
            port.feedback ? Position.Top : port.direction === 'in' ? Position.Left : Position.Right
          }
          style={
            port.position
              ? { left: port.position.x, top: port.position.y, right: 'auto', bottom: 'auto' }
              : port.feedback
                ? { left: `${20 + (i % 5) * 15}%` }
                : { top: `${20 + (i % 5) * 15}%` }
          }
          isConnectable={false}
        />
      ))}
      <button
        className="workflow-flow-card__body nodrag nopan"
        aria-label={data.label}
        aria-pressed={selected}
        onClick={data.activate}
        onKeyDown={(event) => {
          if (!event.altKey || !data.move || !event.key.startsWith('Arrow')) return;
          event.preventDefault();
          event.stopPropagation();
          data.move(
            positionAbsoluteX +
              (event.key === 'ArrowRight' ? 20 : event.key === 'ArrowLeft' ? -20 : 0),
            positionAbsoluteY +
              (event.key === 'ArrowDown' ? 20 : event.key === 'ArrowUp' ? -20 : 0),
          );
        }}
      >
        {data.content}
      </button>
      {data.actions && (
        <div className="workflow-flow-card__actions nodrag nopan">{data.actions}</div>
      )}
    </div>
  );
}
export type ConnectionData = Record<string, unknown> & {
  route?: GraphLayout['routes'][string];
  title: string;
  label: string;
  showLabel: boolean;
  activate(): void;
};
export function WorkflowConnection(props: EdgeProps<Edge<ConnectionData>>) {
  const [hovered, setHovered] = useState(false);
  const route = props.data?.route;
  const [fallback, x, y] = getSmoothStepPath(props);
  const path = route?.points.length
    ? route.points.map((p, i) => `${i ? 'L' : 'M'} ${p.x} ${p.y}`).join(' ')
    : fallback;
  return (
    <>
      <g onMouseEnter={() => setHovered(true)} onMouseLeave={() => setHovered(false)}>
        <BaseEdge
          id={props.id}
          path={path}
          markerEnd={props.markerEnd}
          style={props.style}
          interactionWidth={22}
        />
      </g>
      {(props.data?.showLabel || hovered || props.selected) && (
        <EdgeLabelRenderer>
          <button
            className="workflow-flow-edge-label nodrag nopan"
            style={{
              transform: `translate(-50%, -50%) translate(${route?.label?.x ?? x}px,${route?.label?.y ?? y}px)`,
            }}
            aria-label={props.data?.title}
            onClick={props.data?.activate}
            onMouseEnter={() => setHovered(true)}
            onMouseLeave={() => setHovered(false)}
          >
            {props.data?.label}
          </button>
        </EdgeLabelRenderer>
      )}
    </>
  );
}
