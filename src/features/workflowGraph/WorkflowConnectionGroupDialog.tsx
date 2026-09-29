import { useEffect, useId, useRef } from 'react';
import type { WorkflowGraphConnection, WorkflowGraphNode } from './workflowGraphModel';

export function WorkflowConnectionGroupDialog({
  connections,
  nodes,
  summary,
  onSelect,
  onClose,
}: {
  readonly connections: readonly WorkflowGraphConnection[];
  readonly nodes: readonly WorkflowGraphNode[];
  summary?(connection: WorkflowGraphConnection): string | null;
  onSelect(connectionId: string): void;
  onClose(): void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useId();
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const element = dialog.current;
    element?.showModal();
    return () => {
      element?.close();
      previous?.focus();
    };
  }, []);
  const nodeName = (id: string) => nodes.find((node) => node.id === id)?.name ?? 'Unknown node';
  return (
    <dialog
      ref={dialog}
      className="workflow-connection-group-dialog"
      aria-labelledby={heading}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <header>
        <div>
          <p>Connections between nodes</p>
          <h2 id={heading}>{connections.length} connections</h2>
        </div>
        <button type="button" aria-label="Close connections" onClick={onClose}>
          Close
        </button>
      </header>
      <div className="workflow-connection-group-dialog__list">
        {connections.map((connection) => (
          <button type="button" key={connection.id} onClick={() => onSelect(connection.id)}>
            <strong>{connection.name}</strong>
            <span>
              {nodeName(connection.source)} → {nodeName(connection.destination)}
            </span>
            {summary?.(connection) ? <small>{summary(connection)}</small> : null}
          </button>
        ))}
      </div>
    </dialog>
  );
}
