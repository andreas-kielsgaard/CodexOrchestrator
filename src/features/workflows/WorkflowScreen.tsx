import { Cable, GitBranch, Paintbrush, Plus, Trash2, X } from 'lucide-react';
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
} from 'react';
import type {
  WorkflowApplicationClient,
  WorkflowConnectionConfig,
  WorkflowConnectionElement,
  WorkflowConnectionMechanism,
  WorkflowDefinition,
  WorkflowElementRef,
  WorkflowNodeConfig,
  WorkflowNodeElement,
  WorkflowTypeSummary,
} from '../../application/workflows';
import { workflowPersistenceCoordinator } from '../../application/workflows';
import './workflow.css';

export interface WorkflowScreenProps {
  readonly client: WorkflowApplicationClient;
  readonly workflowTypeId: string | null;
  onOpenWorkflowType(workflowTypeId: string): void;
}

type LoadState<T> =
  | { readonly kind: 'loading' }
  | { readonly kind: 'ready'; readonly value: T }
  | { readonly kind: 'failed'; readonly message: string };

interface DisplayNode {
  readonly config: WorkflowNodeConfig;
  readonly element?: WorkflowNodeElement;
  readonly localDraft: boolean;
}

interface DisplayConnection {
  readonly config: WorkflowConnectionConfig;
  readonly element?: WorkflowConnectionElement;
  readonly localDraft: boolean;
}

interface ConnectionListState {
  readonly connectionIds: readonly string[];
  readonly title: string;
  readonly left: number;
  readonly top: number;
  readonly nodeId?: string;
}

export function WorkflowScreen({
  client,
  workflowTypeId,
  onOpenWorkflowType,
}: WorkflowScreenProps) {
  const persistentClient = workflowPersistenceCoordinator(client);
  return workflowTypeId ? (
    <WorkflowTypeEditor client={persistentClient} workflowTypeId={workflowTypeId} />
  ) : (
    <WorkflowLanding client={persistentClient} onOpenWorkflowType={onOpenWorkflowType} />
  );
}

function WorkflowLanding({
  client,
  onOpenWorkflowType,
}: Pick<WorkflowScreenProps, 'client' | 'onOpenWorkflowType'>) {
  const [tab, setTab] = useState<'launched' | 'types'>('launched');
  const [types, setTypes] = useState<LoadState<readonly WorkflowTypeSummary[]>>({
    kind: 'loading',
  });
  const [newName, setNewName] = useState('');
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void client.listWorkflowTypes().then(
      (value) => current && setTypes({ kind: 'ready', value }),
      (error: unknown) => current && setTypes({ kind: 'failed', message: errorMessage(error) }),
    );
    return () => {
      current = false;
    };
  }, [client]);

  const createType = async () => {
    const name = newName.trim();
    if (!name || creating) return;
    setCreating(true);
    setCreateError(null);
    try {
      const definition = await client.createWorkflowType({ name });
      onOpenWorkflowType(definition.workflowType.id);
    } catch (error) {
      setCreateError(errorMessage(error));
      setCreating(false);
    }
  };

  return (
    <main className="workflow-screen workflow-landing" aria-label="Workflows">
      <header className="workflow-heading">
        <div>
          <p className="eyebrow">Workflow</p>
          <h1>Agent workflows</h1>
          <p>Compose reusable agent roles into a visual workflow.</p>
        </div>
      </header>

      <div className="workflow-tabs" role="tablist" aria-label="Workflow lists">
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'launched'}
          className={tab === 'launched' ? 'active' : undefined}
          onClick={() => setTab('launched')}
        >
          Launched workflows
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'types'}
          className={tab === 'types' ? 'active' : undefined}
          onClick={() => setTab('types')}
        >
          Workflow types
        </button>
      </div>

      {tab === 'launched' ? (
        <section className="workflow-empty" role="tabpanel">
          <GitBranch aria-hidden="true" />
          <h2>No launched workflows</h2>
          <p>Workflow launching and instance history are not connected in this increment.</p>
        </section>
      ) : (
        <section className="workflow-type-list" role="tabpanel">
          <form
            className="workflow-create-type"
            onSubmit={(event) => {
              event.preventDefault();
              void createType();
            }}
          >
            <label htmlFor="workflow-type-name">New workflow type</label>
            <input
              id="workflow-type-name"
              value={newName}
              placeholder="For example, architecture review"
              onChange={(event) => setNewName(event.currentTarget.value)}
            />
            <button
              className="workflow-primary-button"
              type="submit"
              disabled={!newName.trim() || creating}
            >
              <Plus size={16} aria-hidden="true" />
              {creating ? 'Creating…' : 'Create workflow'}
            </button>
          </form>
          {createError ? (
            <p className="workflow-error" role="alert">
              {createError}
            </p>
          ) : null}
          {types.kind === 'loading' ? (
            <p className="workflow-status">Loading workflow types…</p>
          ) : types.kind === 'failed' ? (
            <p className="workflow-error" role="alert">
              {types.message}
            </p>
          ) : types.value.length === 0 ? (
            <div className="workflow-empty is-compact">
              <h2>No workflow types yet</h2>
              <p>Create one to open the canvas editor.</p>
            </div>
          ) : (
            <ul className="workflow-type-cards" aria-label="Workflow types">
              {types.value.map((workflowType) => (
                <li key={workflowType.id}>
                  <button type="button" onClick={() => onOpenWorkflowType(workflowType.id)}>
                    <span>
                      <strong>{workflowType.name}</strong>
                      <small>
                        {workflowType.activeRecipeId ? 'Active recipe' : 'Not activated'}
                      </small>
                    </span>
                    {workflowType.editedElementCount > 0 ? (
                      <span className="workflow-draft-badge">
                        {workflowType.editedElementCount} edited
                      </span>
                    ) : null}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </main>
  );
}

function WorkflowTypeEditor({
  client,
  workflowTypeId,
}: Pick<WorkflowScreenProps, 'client'> & { readonly workflowTypeId: string }) {
  const [load, setLoad] = useState<LoadState<WorkflowDefinition>>({ kind: 'loading' });
  const [nodeBrush, setNodeBrush] = useState(false);
  const [connectionBrush, setConnectionBrush] = useState(false);
  const [connectionSourceId, setConnectionSourceId] = useState<string | null>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedConnection, setSelectedConnection] = useState<{
    readonly id: string;
    readonly full: boolean;
  } | null>(null);
  const [connectionList, setConnectionList] = useState<ConnectionListState | null>(null);
  const [hoveredConnectionId, setHoveredConnectionId] = useState<string | null>(null);
  const [bulkOpen, setBulkOpen] = useState(false);
  const [bulkSelection, setBulkSelection] = useState<ReadonlySet<string>>(() => new Set());
  const [workingNodes, setWorkingNodes] = useState<ReadonlyMap<string, WorkflowNodeConfig>>(
    () => new Map(),
  );
  const [workingConnections, setWorkingConnections] = useState<
    ReadonlyMap<string, WorkflowConnectionConfig>
  >(() => new Map());
  const workingNodesRef = useRef<ReadonlyMap<string, WorkflowNodeConfig>>(new Map());
  const workingConnectionsRef = useRef<ReadonlyMap<string, WorkflowConnectionConfig>>(new Map());
  const workingRevisionsRef = useRef<Map<string, number>>(new Map());
  const connectionRevisionsRef = useRef<Map<string, number>>(new Map());
  const dragSourceRef = useRef<string | null>(null);
  const dragReconnectConnectionIdRef = useRef<string | null>(null);
  const suppressConnectionClickRef = useRef(false);
  const definitionRef = useRef<WorkflowDefinition | null>(null);
  const mountedRef = useRef(true);
  const [saving, setSaving] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const replaceWorkingNodes = useCallback((next: ReadonlyMap<string, WorkflowNodeConfig>) => {
    workingNodesRef.current = next;
    setWorkingNodes(next);
  }, []);

  const replaceWorkingConnections = useCallback(
    (next: ReadonlyMap<string, WorkflowConnectionConfig>) => {
      workingConnectionsRef.current = next;
      setWorkingConnections(next);
    },
    [],
  );

  const queueNodeSave = useCallback(
    (node: WorkflowNodeConfig): Promise<WorkflowDefinition> => {
      const definition = definitionRef.current;
      const startingPointDemotions = node.isStartingPoint
        ? (definition?.nodes ?? []).flatMap((element) => {
            const existing = element.draft ?? element.live;
            return existing?.id !== node.id && existing?.isStartingPoint
              ? [{ ...existing, isStartingPoint: false }]
              : [];
          })
        : [];
      let operation: Promise<WorkflowDefinition> | null = null;
      for (const existing of startingPointDemotions)
        operation = operation
          ? operation.then(() => client.saveNodeDraft(workflowTypeId, existing))
          : client.saveNodeDraft(workflowTypeId, existing);
      return operation
        ? operation.then(() => client.saveNodeDraft(workflowTypeId, node))
        : client.saveNodeDraft(workflowTypeId, node);
    },
    [client, workflowTypeId],
  );

  const queueConnectionSave = useCallback(
    (connection: WorkflowConnectionConfig) =>
      client.saveConnectionDraft(workflowTypeId, connection),
    [client, workflowTypeId],
  );

  useEffect(() => {
    let current = true;
    setLoad({ kind: 'loading' });
    definitionRef.current = null;
    replaceWorkingNodes(new Map());
    replaceWorkingConnections(new Map());
    workingRevisionsRef.current.clear();
    connectionRevisionsRef.current.clear();
    setSelectedNodeId(null);
    setSelectedConnection(null);
    setConnectionList(null);
    void client.loadWorkflowType(workflowTypeId).then(
      (value) => {
        if (!current) return;
        definitionRef.current = value;
        setLoad({ kind: 'ready', value });
      },
      (error: unknown) => current && setLoad({ kind: 'failed', message: errorMessage(error) }),
    );
    return () => {
      current = false;
    };
  }, [client, replaceWorkingConnections, replaceWorkingNodes, workflowTypeId]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      for (const node of workingNodesRef.current.values())
        void queueNodeSave(node).catch(() => undefined);
      for (const connection of workingConnectionsRef.current.values())
        void queueConnectionSave(connection).catch(() => undefined);
    };
  }, [queueConnectionSave, queueNodeSave]);

  useEffect(() => {
    const clearMissedPointerGesture = () => {
      dragSourceRef.current = null;
      dragReconnectConnectionIdRef.current = null;
    };
    window.addEventListener('pointerup', clearMissedPointerGesture);
    window.addEventListener('pointercancel', clearMissedPointerGesture);
    return () => {
      window.removeEventListener('pointerup', clearMissedPointerGesture);
      window.removeEventListener('pointercancel', clearMissedPointerGesture);
    };
  }, []);

  if (load.kind === 'loading')
    return (
      <main className="workflow-screen workflow-status" aria-label="Workflow type editor">
        Loading workflow type…
      </main>
    );
  if (load.kind === 'failed')
    return (
      <main className="workflow-screen workflow-error" aria-label="Workflow type editor">
        {load.message}
      </main>
    );

  const definition = load.value;
  const displayNodes = mergeDisplayNodes(definition, workingNodes);
  const displayConnections = mergeDisplayConnections(definition, workingConnections);
  const connectionGroups = groupDisplayConnections(displayConnections);
  const editedElements = listEditedElements(definition);
  const selectedNode = selectedNodeId
    ? displayNodes.find((node) => node.config.id === selectedNodeId)
    : undefined;
  const selectedConnectionDisplay = selectedConnection
    ? displayConnections.find((connection) => connection.config.id === selectedConnection.id)
    : undefined;
  const persistNode = async (
    node: WorkflowNodeConfig,
    options: { readonly blocking?: boolean; readonly closeAfter?: boolean } = {},
  ) => {
    const revision = workingRevisionsRef.current.get(node.id) ?? 0;
    if (options.blocking) setSaving(true);
    setActionError(null);
    try {
      const nextDefinition = await queueNodeSave(node);
      if (!mountedRef.current) return;
      definitionRef.current = nextDefinition;
      setLoad({ kind: 'ready', value: nextDefinition });
      if (workingRevisionsRef.current.get(node.id) === revision) {
        const next = new Map(workingNodesRef.current);
        next.delete(node.id);
        replaceWorkingNodes(next);
        if (options.closeAfter) setSelectedNodeId(null);
      }
    } catch (error) {
      if (mountedRef.current) setActionError(errorMessage(error));
    } finally {
      if (options.blocking && mountedRef.current) setSaving(false);
    }
  };

  const updateWorkingNode = (node: WorkflowNodeConfig) => {
    const scratchNode = { ...node, roleName: null };
    const next = new Map(workingNodesRef.current);
    next.set(scratchNode.id, scratchNode);
    workingRevisionsRef.current.set(
      scratchNode.id,
      (workingRevisionsRef.current.get(scratchNode.id) ?? 0) + 1,
    );
    replaceWorkingNodes(next);
    void persistNode(scratchNode);
  };

  const persistConnection = async (
    connection: WorkflowConnectionConfig,
    options: { readonly blocking?: boolean; readonly closeAfter?: boolean } = {},
  ) => {
    const revision = connectionRevisionsRef.current.get(connection.id) ?? 0;
    if (options.blocking) setSaving(true);
    setActionError(null);
    try {
      const nextDefinition = await queueConnectionSave(connection);
      if (!mountedRef.current) return;
      definitionRef.current = nextDefinition;
      setLoad({ kind: 'ready', value: nextDefinition });
      if (connectionRevisionsRef.current.get(connection.id) === revision) {
        const next = new Map(workingConnectionsRef.current);
        next.delete(connection.id);
        replaceWorkingConnections(next);
        if (options.closeAfter) setSelectedConnection(null);
      }
    } catch (error) {
      if (mountedRef.current) setActionError(errorMessage(error));
    } finally {
      if (options.blocking && mountedRef.current) setSaving(false);
    }
  };

  const updateWorkingConnection = (connection: WorkflowConnectionConfig) => {
    const next = new Map(workingConnectionsRef.current);
    next.set(connection.id, connection);
    connectionRevisionsRef.current.set(
      connection.id,
      (connectionRevisionsRef.current.get(connection.id) ?? 0) + 1,
    );
    replaceWorkingConnections(next);
    void persistConnection(connection);
  };

  const placeNodeAt = (positionX: number, positionY: number) => {
    if (!nodeBrush || saving || selectedNodeId) return;
    const id = globalThis.crypto?.randomUUID?.() ?? `node-${Date.now()}`;
    const node: WorkflowNodeConfig = {
      id,
      name: '',
      harnessName: '',
      roleName: null,
      positionX,
      positionY,
      isStartingPoint: displayNodes.length === 0,
    };
    const next = new Map(workingNodesRef.current);
    next.set(id, node);
    workingRevisionsRef.current.set(id, 1);
    replaceWorkingNodes(next);
    setSelectedNodeId(id);
    void persistNode(node, { blocking: true });
  };

  const placeNode = (event: MouseEvent<HTMLDivElement>) => {
    if (suppressConnectionClickRef.current) {
      suppressConnectionClickRef.current = false;
      return;
    }
    if (selectedNodeId) {
      if (!saving && selectedNode) {
        const locallyChanged = workingNodesRef.current.get(selectedNode.config.id);
        if (locallyChanged) void persistNode(locallyChanged, { blocking: true, closeAfter: true });
        else setSelectedNodeId(null);
      }
      return;
    }
    if (selectedConnection || connectionList) {
      setSelectedConnection(null);
      setConnectionList(null);
      setHoveredConnectionId(null);
      return;
    }
    if (bulkOpen) {
      setBulkOpen(false);
      return;
    }
    if (!nodeBrush || saving || event.target !== event.currentTarget) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    placeNodeAt(
      Math.max(24, Math.min(event.clientX - bounds.left, bounds.width - 244)),
      Math.max(28, Math.min(event.clientY - bounds.top, bounds.height - 124)),
    );
  };

  const placeNodeWithKeyboard = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!nodeBrush || selectedNodeId || saving || (event.key !== 'Enter' && event.key !== ' '))
      return;
    event.preventDefault();
    const bounds = event.currentTarget.getBoundingClientRect();
    placeNodeAt(Math.max(24, bounds.width / 2 - 105), Math.max(28, bounds.height / 2 - 46));
  };

  const activateElements = async (elements: readonly WorkflowElementRef[]) => {
    if (saving) return;
    setSaving(true);
    setActionError(null);
    try {
      const next = await client.activateChanges(
        workflowTypeId,
        expandActivation(elements, definition),
      );
      definitionRef.current = next;
      setLoad({ kind: 'ready', value: next });
      setBulkOpen(false);
      setBulkSelection(new Set());
    } catch (error) {
      setActionError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  };

  const createConnection = (senderNodeId: string, receiverNodeId: string) => {
    if (saving) return;
    const sender = displayNodes.find((node) => node.config.id === senderNodeId)?.config;
    const receiver = displayNodes.find((node) => node.config.id === receiverNodeId)?.config;
    if (!sender || !receiver) return;
    const id = globalThis.crypto?.randomUUID?.() ?? `connection-${Date.now()}`;
    const connection: WorkflowConnectionConfig = {
      id,
      name: `${sender.name || sender.harnessName || 'Sender'} to ${receiver.name || receiver.harnessName || 'Receiver'}`,
      senderNodeId,
      receiverNodeId,
      mechanism: null,
    };
    const next = new Map(workingConnectionsRef.current);
    next.set(id, connection);
    connectionRevisionsRef.current.set(id, 1);
    replaceWorkingConnections(next);
    setConnectionSourceId(null);
    setSelectedConnection({ id, full: true });
    setSelectedNodeId(null);
    setConnectionList(null);
    void persistConnection(connection, { blocking: true });
  };

  const handleConnectionNodeClick = (nodeId: string) => {
    if (suppressConnectionClickRef.current) {
      suppressConnectionClickRef.current = false;
      return;
    }
    if (connectionSourceId) createConnection(connectionSourceId, nodeId);
    else setConnectionSourceId(nodeId);
  };

  const handleNodePointerDown = (nodeId: string) => {
    if (!connectionBrush || saving) return;
    dragReconnectConnectionIdRef.current = null;
    dragSourceRef.current = nodeId;
  };

  const handleNodePointerUp = (nodeId: string) => {
    const reconnectConnectionId = dragReconnectConnectionIdRef.current;
    dragReconnectConnectionIdRef.current = null;
    if (connectionBrush && reconnectConnectionId) {
      const connection = displayConnections.find(
        (candidate) => candidate.config.id === reconnectConnectionId,
      );
      if (connection) {
        suppressConnectionClickRef.current = true;
        const reconnected = { ...connection.config, receiverNodeId: nodeId };
        updateWorkingConnection(reconnected);
        setSelectedConnection({ id: reconnectConnectionId, full: true });
        setConnectionList(null);
        setConnectionSourceId(null);
        globalThis.setTimeout(() => {
          suppressConnectionClickRef.current = false;
        }, 0);
      }
      return;
    }
    const source = dragSourceRef.current;
    dragSourceRef.current = null;
    if (!connectionBrush || !source || source === nodeId) return;
    suppressConnectionClickRef.current = true;
    createConnection(source, nodeId);
    globalThis.setTimeout(() => {
      suppressConnectionClickRef.current = false;
    }, 0);
  };

  const openConnectionList = (state: ConnectionListState) => {
    setSelectedNodeId(null);
    setSelectedConnection(null);
    setHoveredConnectionId(null);
    setConnectionList(state);
  };

  const deleteNode = async (nodeId: string) => {
    if (saving) return;
    setSaving(true);
    setActionError(null);
    try {
      const next = await client.deleteNodeDraft(workflowTypeId, nodeId);
      definitionRef.current = next;
      setLoad({ kind: 'ready', value: next });
      const local = new Map(workingNodesRef.current);
      local.delete(nodeId);
      replaceWorkingNodes(local);
      setSelectedNodeId(null);
    } catch (error) {
      setActionError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  };

  const deleteConnection = async (connectionId: string) => {
    if (saving) return;
    setSaving(true);
    setActionError(null);
    try {
      const next = await client.deleteConnectionDraft(workflowTypeId, connectionId);
      definitionRef.current = next;
      setLoad({ kind: 'ready', value: next });
      const local = new Map(workingConnectionsRef.current);
      local.delete(connectionId);
      replaceWorkingConnections(local);
      setSelectedConnection(null);
    } catch (error) {
      setActionError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <main
      className="workflow-screen workflow-editor"
      aria-label={`Edit ${definition.workflowType.name}`}
    >
      <header className="workflow-editor__header">
        <div>
          <p className="eyebrow">Workflow type</p>
          <h1>{definition.workflowType.name}</h1>
        </div>
        <div className="workflow-editor__summary" aria-label="Workflow draft summary">
          <span>
            {displayNodes.length} {displayNodes.length === 1 ? 'node' : 'nodes'}
          </span>
          <span>{definition.workflowType.editedElementCount} edited</span>
          <span>
            {definition.activeRecipe
              ? `Recipe ${definition.activeRecipe.ordinal}`
              : 'Not activated'}
          </span>
        </div>
      </header>

      <div className="workflow-editor__toolbar" role="toolbar" aria-label="Workflow tools">
        <button
          type="button"
          aria-pressed={nodeBrush}
          className={nodeBrush ? 'active' : undefined}
          disabled={saving}
          onClick={() => {
            setNodeBrush((current) => !current);
            setConnectionBrush(false);
            setConnectionSourceId(null);
          }}
        >
          <Paintbrush size={17} aria-hidden="true" />
          Node
        </button>
        <button
          type="button"
          aria-pressed={connectionBrush}
          className={connectionBrush ? 'active' : undefined}
          disabled={saving || displayNodes.length === 0}
          onClick={() => {
            setConnectionBrush((current) => !current);
            setNodeBrush(false);
            setConnectionSourceId(null);
          }}
        >
          <Cable size={17} aria-hidden="true" />
          Connection
        </button>
        <button
          type="button"
          disabled={saving || editedElements.length === 0}
          aria-expanded={bulkOpen}
          onClick={() => {
            setBulkOpen((current) => !current);
            setBulkSelection(new Set());
          }}
        >
          Activate edits ({editedElements.length})
        </button>
        <p id="workflow-canvas-instructions">
          {connectionBrush
            ? connectionSourceId
              ? 'Connection brush active · select a target node'
              : 'Connection brush active · select a source, then a target, or drag between nodes'
            : nodeBrush
              ? 'Node brush active · click the canvas to place nodes'
              : 'Select a brush to edit the workflow'}
        </p>
      </div>

      <div
        className={`workflow-canvas${nodeBrush || connectionBrush ? ' has-node-brush' : ''}`}
        aria-label="Workflow canvas"
        aria-describedby="workflow-canvas-instructions"
        tabIndex={0}
        onClick={placeNode}
        onKeyDown={placeNodeWithKeyboard}
      >
        <svg className="workflow-canvas__connections" aria-label="Workflow connections">
          <defs>
            <marker
              id="workflow-arrow"
              markerWidth="8"
              markerHeight="8"
              refX="7"
              refY="4"
              orient="auto"
            >
              <path d="M0,0 L8,4 L0,8 z" />
            </marker>
          </defs>
          {connectionGroups.map((group) => {
            const sender = displayNodes.find((node) => node.config.id === group.senderNodeId);
            const receiver = group.endpointNodeId
              ? displayNodes.find((node) => node.config.id === group.endpointNodeId)
              : undefined;
            if (!sender) return null;
            const receiverX = receiver?.config.positionX ?? sender.config.positionX + 320;
            const receiverY = receiver?.config.positionY ?? sender.config.positionY + 130;
            const highlighted = group.connections.some(
              (connection) => connection.config.id === hoveredConnectionId,
            );
            const destination = group.dangling
              ? 'a dangling endpoint'
              : receiver?.config.name || receiver?.config.harnessName || 'receiver';
            const label = `${group.connections.length} connection${group.connections.length === 1 ? '' : 's'} from ${sender.config.name || sender.config.harnessName || 'sender'} to ${destination}`;
            const left = (sender.config.positionX + receiverX) / 2 + 84;
            const top = (sender.config.positionY + receiverY) / 2 + 56;
            const openList = () =>
              openConnectionList({
                connectionIds: group.connections.map((connection) => connection.config.id),
                title: label,
                left,
                top,
              });
            return (
              <g
                key={group.key}
                role="button"
                tabIndex={0}
                aria-label={label}
                className={`${highlighted ? 'is-highlighted ' : ''}${group.dangling ? 'is-dangling' : ''}`}
                onPointerDown={(event) => {
                  if (!connectionBrush || !group.dangling || group.connections.length !== 1) return;
                  event.stopPropagation();
                  dragSourceRef.current = null;
                  dragReconnectConnectionIdRef.current = group.connections[0]!.config.id;
                }}
                onClick={(event) => {
                  event.stopPropagation();
                  openList();
                }}
                onKeyDown={(event) => {
                  if (event.key !== 'Enter' && event.key !== ' ') return;
                  event.preventDefault();
                  openList();
                }}
              >
                <line
                  className="workflow-connection__visible"
                  x1={sender.config.positionX + 210}
                  y1={sender.config.positionY + 46}
                  x2={receiverX}
                  y2={receiverY + 46}
                  markerEnd={group.dangling ? undefined : 'url(#workflow-arrow)'}
                />
                <line
                  className="workflow-connection__hitbox"
                  x1={sender.config.positionX + 210}
                  y1={sender.config.positionY + 46}
                  x2={receiverX}
                  y2={receiverY + 46}
                />
                {group.connections.length > 1 ? (
                  <text
                    x={(sender.config.positionX + 210 + receiverX) / 2}
                    y={(sender.config.positionY + receiverY) / 2 + 38}
                  >
                    {group.connections.length}
                  </text>
                ) : null}
              </g>
            );
          })}
        </svg>

        {displayNodes.map(({ config, element, localDraft }) => (
          <button
            key={config.id}
            type="button"
            disabled={saving}
            className={`workflow-node${config.isStartingPoint ? ' is-start' : ''}${connectionSourceId === config.id ? ' is-connection-source' : ''}${element?.draft === null && element.live ? ' is-deleted' : ''}`}
            style={{ left: config.positionX, top: config.positionY }}
            onPointerDown={() => handleNodePointerDown(config.id)}
            onPointerUp={() => handleNodePointerUp(config.id)}
            onClick={(event) => {
              event.stopPropagation();
              if (connectionBrush) {
                handleConnectionNodeClick(config.id);
                return;
              }
              const outgoing = displayConnections.filter(
                (connection) => connection.config.senderNodeId === config.id,
              );
              if (outgoing.length > 1) {
                openConnectionList({
                  connectionIds: outgoing.map((connection) => connection.config.id),
                  title: `Connections from ${config.name || config.harnessName || 'node'}`,
                  left: config.positionX + 222,
                  top: config.positionY,
                  nodeId: config.id,
                });
                return;
              }
              setSelectedNodeId(config.id);
              setSelectedConnection(null);
              setConnectionList(null);
              setActionError(null);
            }}
            aria-label={`Configure ${config.name || 'new node'}`}
          >
            <span className="workflow-node__badges">
              {config.isStartingPoint ? <small>Start</small> : null}
              {localDraft || element?.hasUnpublishedChanges ? (
                <small className="is-draft">Draft</small>
              ) : null}
              {element?.draft === null && element.live ? <small>Delete</small> : null}
            </span>
            <strong>{config.harnessName || 'Choose a role'}</strong>
            <span>{config.name || 'Name this node'}</span>
          </button>
        ))}

        {displayNodes.length === 0 ? (
          <div className="workflow-canvas__empty" aria-hidden="true">
            <Paintbrush />
            <strong>Paint your first agent role</strong>
            <span>Select Node, then click anywhere on the canvas.</span>
          </div>
        ) : null}

        {selectedNode ? (
          <NodeConfiguration
            node={selectedNode.config}
            persistedElement={selectedNode.element}
            locallyChanged={selectedNode.localDraft}
            busy={saving}
            error={actionError}
            onChange={updateWorkingNode}
            onClose={() => {
              const localDraft = workingNodesRef.current.get(selectedNode.config.id);
              if (localDraft) void persistNode(localDraft, { blocking: true, closeAfter: true });
              else setSelectedNodeId(null);
            }}
            onSave={() => void persistNode(selectedNode.config, { blocking: true })}
            onActivate={() => void activateElements([{ kind: 'node', id: selectedNode.config.id }])}
            onDelete={() => void deleteNode(selectedNode.config.id)}
          />
        ) : null}

        {connectionList ? (
          <ConnectionList
            state={connectionList}
            connections={displayConnections}
            nodes={displayNodes}
            onHover={setHoveredConnectionId}
            onClose={() => {
              setConnectionList(null);
              setHoveredConnectionId(null);
            }}
            onConfigureNode={
              connectionList.nodeId
                ? () => {
                    setSelectedNodeId(connectionList.nodeId!);
                    setConnectionList(null);
                  }
                : undefined
            }
            onOpen={(id) => {
              setSelectedConnection({ id, full: false });
              setConnectionList(null);
              setHoveredConnectionId(null);
            }}
          />
        ) : null}

        {selectedConnectionDisplay && selectedConnection ? (
          selectedConnection.full ? (
            <ConnectionConfiguration
              connection={selectedConnectionDisplay.config}
              persistedElement={selectedConnectionDisplay.element}
              locallyChanged={selectedConnectionDisplay.localDraft}
              nodes={displayNodes}
              busy={saving}
              error={actionError}
              onChange={updateWorkingConnection}
              onClose={() => {
                const local = workingConnectionsRef.current.get(
                  selectedConnectionDisplay.config.id,
                );
                if (local) void persistConnection(local, { blocking: true, closeAfter: true });
                else setSelectedConnection(null);
              }}
              onSave={() =>
                void persistConnection(selectedConnectionDisplay.config, { blocking: true })
              }
              onActivate={() =>
                void activateElements([
                  { kind: 'connection', id: selectedConnectionDisplay.config.id },
                ])
              }
              onDelete={() => void deleteConnection(selectedConnectionDisplay.config.id)}
            />
          ) : (
            <ConnectionPreview
              connection={selectedConnectionDisplay}
              nodes={displayNodes}
              busy={saving}
              onClose={() => setSelectedConnection(null)}
              onOpen={() =>
                setSelectedConnection({ id: selectedConnectionDisplay.config.id, full: true })
              }
            />
          )
        ) : null}

        {bulkOpen ? (
          <BulkActivation
            elements={editedElements}
            selection={bulkSelection}
            busy={saving}
            error={actionError}
            onChange={setBulkSelection}
            onClose={() => setBulkOpen(false)}
            onActivate={() =>
              void activateElements(
                editedElements
                  .filter((element) => bulkSelection.has(elementKey(element.ref)))
                  .map((element) => element.ref),
              )
            }
          />
        ) : null}

        {actionError && !selectedNode && !selectedConnectionDisplay && !bulkOpen ? (
          <p className="workflow-canvas__error workflow-error" role="alert">
            {actionError}
          </p>
        ) : null}
      </div>
    </main>
  );
}

function NodeConfiguration({
  node,
  persistedElement,
  locallyChanged,
  busy,
  error,
  onChange,
  onClose,
  onSave,
  onActivate,
  onDelete,
}: {
  readonly node: WorkflowNodeConfig;
  readonly persistedElement?: WorkflowNodeElement;
  readonly locallyChanged: boolean;
  readonly busy: boolean;
  readonly error: string | null;
  onChange(node: WorkflowNodeConfig): void;
  onClose(): void;
  onSave(): void;
  onActivate(): void;
  onDelete(): void;
}) {
  const pendingDeletion = Boolean(persistedElement?.live && !persistedElement.draft);
  const valid = Boolean(node.name.trim() && node.harnessName.trim());
  const canActivate = Boolean(
    (valid || pendingDeletion) && persistedElement?.hasUnpublishedChanges && !locallyChanged,
  );

  return (
    <section
      className="workflow-node-config"
      role="dialog"
      aria-label={node.name ? `Configure ${node.name}` : 'Configure new node'}
      onClick={(event) => event.stopPropagation()}
    >
      <header>
        <div>
          <p className="eyebrow">Node configuration</p>
          <h2>{node.name || 'New agent role'}</h2>
        </div>
        <button
          type="button"
          aria-label="Close node configuration"
          onClick={onClose}
          disabled={busy}
        >
          <X size={18} aria-hidden="true" />
        </button>
      </header>

      {pendingDeletion ? (
        <p className="workflow-deletion-note">This node is marked for deletion.</p>
      ) : null}

      <fieldset className="workflow-role-choice" disabled={busy || pendingDeletion}>
        <legend>Start with a role</legend>
        <label>
          <input type="radio" name={`role-mode-${node.id}`} checked={false} disabled readOnly />
          <span>
            Existing role
            <small>Unavailable until the role catalog is connected</small>
          </span>
        </label>
        <label>
          <input type="radio" name={`role-mode-${node.id}`} checked readOnly />
          From scratch
        </label>
      </fieldset>

      <label>
        Harness name
        <input
          value={node.harnessName}
          placeholder="For example, Architecture reviewer"
          disabled={busy || pendingDeletion}
          onChange={(event) =>
            onChange({ ...node, roleName: null, harnessName: event.currentTarget.value })
          }
        />
      </label>

      <label>
        Node name
        <input
          value={node.name}
          placeholder="For example, Review proposed architecture"
          disabled={busy || pendingDeletion}
          onChange={(event) =>
            onChange({ ...node, roleName: null, name: event.currentTarget.value })
          }
        />
      </label>

      <label className="workflow-start-choice">
        <input
          type="checkbox"
          checked={node.isStartingPoint}
          disabled={busy || pendingDeletion}
          onChange={(event) =>
            onChange({
              ...node,
              roleName: null,
              isStartingPoint: event.currentTarget.checked,
            })
          }
        />
        Starting node
        <small>The activated workflow has exactly one starting node.</small>
      </label>

      {error ? (
        <p className="workflow-error" role="alert">
          {error}
        </p>
      ) : null}

      <footer>
        <span>
          {pendingDeletion
            ? 'Deletion draft'
            : locallyChanged || persistedElement?.hasUnpublishedChanges
              ? 'Draft changes'
              : 'Activated'}
        </span>
        <div>
          {!pendingDeletion ? (
            <>
              <button type="button" onClick={onDelete} disabled={busy || !persistedElement}>
                <Trash2 size={15} aria-hidden="true" />
                Delete
              </button>
              <button type="button" onClick={onSave} disabled={!valid || busy || !locallyChanged}>
                {busy ? 'Saving…' : 'Save draft'}
              </button>
            </>
          ) : null}
          {canActivate ? (
            <button
              className="workflow-primary-button"
              type="button"
              onClick={onActivate}
              disabled={busy}
            >
              {busy ? 'Activating…' : pendingDeletion ? 'Activate deletion' : 'Activate node'}
            </button>
          ) : null}
        </div>
      </footer>
    </section>
  );
}

function ConnectionList({
  state,
  connections,
  nodes,
  onHover,
  onClose,
  onConfigureNode,
  onOpen,
}: {
  readonly state: ConnectionListState;
  readonly connections: readonly DisplayConnection[];
  readonly nodes: readonly DisplayNode[];
  onHover(id: string | null): void;
  onClose(): void;
  onConfigureNode?(): void;
  onOpen(id: string): void;
}) {
  const listed = state.connectionIds.flatMap((id) => {
    const connection = connections.find((candidate) => candidate.config.id === id);
    return connection ? [connection] : [];
  });
  return (
    <section
      className="workflow-connection-list"
      role="dialog"
      aria-label={state.title}
      style={{ left: state.left, top: state.top }}
      onClick={(event) => event.stopPropagation()}
    >
      <header>
        <strong>{state.title}</strong>
        <button type="button" aria-label="Close connection list" onClick={onClose}>
          <X size={15} aria-hidden="true" />
        </button>
      </header>
      <ul>
        {listed.map((connection) => {
          const receiver = nodes.find(
            (node) => node.config.id === connection.config.receiverNodeId,
          );
          return (
            <li key={connection.config.id}>
              <button
                type="button"
                onMouseEnter={() => onHover(connection.config.id)}
                onMouseLeave={() => onHover(null)}
                onFocus={() => onHover(connection.config.id)}
                onBlur={() => onHover(null)}
                onClick={() => onOpen(connection.config.id)}
              >
                <span>{connection.config.name || 'Unnamed connection'}</span>
                <small>
                  {connection.config.receiverNodeId
                    ? `To ${receiver?.config.name || receiver?.config.harnessName || 'node'}`
                    : 'Dangling target'}
                  {connection.element?.hasUnpublishedChanges ? ' · Draft' : ''}
                </small>
              </button>
            </li>
          );
        })}
      </ul>
      {onConfigureNode ? (
        <button type="button" className="workflow-list-secondary" onClick={onConfigureNode}>
          Configure node
        </button>
      ) : null}
    </section>
  );
}

function ConnectionPreview({
  connection,
  nodes,
  busy,
  onClose,
  onOpen,
}: {
  readonly connection: DisplayConnection;
  readonly nodes: readonly DisplayNode[];
  readonly busy: boolean;
  onClose(): void;
  onOpen(): void;
}) {
  const sender = nodes.find((node) => node.config.id === connection.config.senderNodeId)?.config;
  const receiver = nodes.find(
    (node) => node.config.id === connection.config.receiverNodeId,
  )?.config;
  return (
    <section
      className="workflow-connection-preview"
      role="dialog"
      aria-label={`Preview ${connection.config.name || 'connection'}`}
      onClick={(event) => event.stopPropagation()}
    >
      <header>
        <div>
          <p className="eyebrow">Connection preview</p>
          <h2>{connection.config.name || 'Unnamed connection'}</h2>
        </div>
        <button type="button" aria-label="Close connection preview" onClick={onClose}>
          <X size={18} aria-hidden="true" />
        </button>
      </header>
      <p>
        {sender?.name || sender?.harnessName || 'Sender'} →{' '}
        {receiver?.name || receiver?.harnessName || 'Dangling target'}
      </p>
      <p>
        {connection.config.mechanism
          ? 'Turn finished · expected file'
          : 'Connecting mechanism not configured'}
      </p>
      <button type="button" className="workflow-primary-button" disabled={busy} onClick={onOpen}>
        Open connection configuration
      </button>
    </section>
  );
}

function ConnectionConfiguration({
  connection,
  persistedElement,
  locallyChanged,
  nodes,
  busy,
  error,
  onChange,
  onClose,
  onSave,
  onActivate,
  onDelete,
}: {
  readonly connection: WorkflowConnectionConfig;
  readonly persistedElement?: WorkflowConnectionElement;
  readonly locallyChanged: boolean;
  readonly nodes: readonly DisplayNode[];
  readonly busy: boolean;
  readonly error: string | null;
  onChange(connection: WorkflowConnectionConfig): void;
  onClose(): void;
  onSave(): void;
  onActivate(): void;
  onDelete(): void;
}) {
  const pendingDeletion = Boolean(persistedElement?.live && !persistedElement.draft);
  const mechanism = connection.mechanism;
  const complete = connectionIsComplete(connection);
  const canActivate = Boolean(
    persistedElement?.hasUnpublishedChanges && !locallyChanged && (complete || pendingDeletion),
  );
  const sender = nodes.find((node) => node.config.id === connection.senderNodeId)?.config;
  const setMechanism = (next: WorkflowConnectionMechanism | null) =>
    onChange({ ...connection, mechanism: next });

  return (
    <section
      className="workflow-node-config workflow-connection-config"
      role="dialog"
      aria-label={`Configure ${connection.name || 'connection'}`}
      onClick={(event) => event.stopPropagation()}
    >
      <header>
        <div>
          <p className="eyebrow">Connection configuration</p>
          <h2>{connection.name || 'New connection'}</h2>
        </div>
        <button
          type="button"
          aria-label="Close connection configuration"
          onClick={onClose}
          disabled={busy}
        >
          <X size={18} aria-hidden="true" />
        </button>
      </header>

      {pendingDeletion ? (
        <p className="workflow-deletion-note">This connection is marked for deletion.</p>
      ) : (
        <>
          <label>
            Connection name
            <input
              value={connection.name}
              disabled={busy}
              onChange={(event) => onChange({ ...connection, name: event.currentTarget.value })}
            />
          </label>
          <label>
            Sender
            <input
              value={sender?.name || sender?.harnessName || connection.senderNodeId}
              disabled
            />
          </label>
          <label>
            Receiver node
            <select
              value={connection.receiverNodeId ?? ''}
              disabled={busy}
              onChange={(event) =>
                onChange({ ...connection, receiverNodeId: event.currentTarget.value || null })
              }
            >
              <option value="">Choose a target</option>
              {nodes
                .filter((node) => !(node.element?.live && !node.element.draft))
                .map((node) => (
                  <option key={node.config.id} value={node.config.id}>
                    {node.config.name || node.config.harnessName || 'Unnamed node'}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Connecting mechanism
            <select
              value={mechanism?.kind ?? ''}
              disabled={busy}
              onChange={(event) =>
                setMechanism(
                  event.currentTarget.value === 'turn_finished_expected_file'
                    ? newTurnFinishedMechanism()
                    : null,
                )
              }
            >
              <option value="">Not configured</option>
              <option value="turn_finished_expected_file">Turn finished · expected file</option>
            </select>
          </label>
          {mechanism ? (
            <TurnFinishedMechanismFields
              mechanism={mechanism}
              busy={busy}
              onChange={setMechanism}
            />
          ) : (
            <p className="workflow-draft-note">
              This connection remains a draft until its connecting mechanism is complete.
            </p>
          )}
        </>
      )}

      {error ? (
        <p className="workflow-error" role="alert">
          {error}
        </p>
      ) : null}
      <footer>
        <span>
          {pendingDeletion
            ? 'Deletion draft'
            : locallyChanged || persistedElement?.hasUnpublishedChanges
              ? 'Draft changes'
              : 'Activated'}
        </span>
        <div>
          {!pendingDeletion ? (
            <>
              <button type="button" onClick={onDelete} disabled={busy || !persistedElement}>
                <Trash2 size={15} aria-hidden="true" /> Delete
              </button>
              <button type="button" onClick={onSave} disabled={busy || !locallyChanged}>
                {busy ? 'Saving…' : 'Save draft'}
              </button>
            </>
          ) : null}
          {canActivate ? (
            <button
              type="button"
              className="workflow-primary-button"
              onClick={onActivate}
              disabled={busy}
            >
              {busy ? 'Activating…' : pendingDeletion ? 'Activate deletion' : 'Activate connection'}
            </button>
          ) : null}
        </div>
      </footer>
    </section>
  );
}

function TurnFinishedMechanismFields({
  mechanism,
  busy,
  onChange,
}: {
  readonly mechanism: WorkflowConnectionMechanism;
  readonly busy: boolean;
  onChange(mechanism: WorkflowConnectionMechanism): void;
}) {
  const selector = mechanism.fileSelector;
  return (
    <fieldset className="workflow-mechanism-fields" disabled={busy}>
      <legend>Expected file</legend>
      <label>
        File locator
        <select
          value={selector.kind}
          onChange={(event) =>
            onChange({
              ...mechanism,
              fileSelector:
                event.currentTarget.value === 'folder_output_regex'
                  ? { kind: 'folder_output_regex', folder: selector.folder, outputRegex: '' }
                  : {
                      kind: 'folder_filename_pattern',
                      folder: selector.folder,
                      filenamePattern: '',
                    },
            })
          }
        >
          <option value="folder_filename_pattern">Folder + filename pattern</option>
          <option value="folder_output_regex">Folder + regex over sender output</option>
        </select>
      </label>
      <label>
        Folder
        <input
          value={selector.folder}
          placeholder="Relative folder, for example handoffs"
          onChange={(event) =>
            onChange({
              ...mechanism,
              fileSelector: { ...selector, folder: event.currentTarget.value },
            })
          }
        />
      </label>
      {selector.kind === 'folder_filename_pattern' ? (
        <label>
          Filename pattern
          <input
            value={selector.filenamePattern}
            placeholder="For example *.md"
            onChange={(event) =>
              onChange({
                ...mechanism,
                fileSelector: { ...selector, filenamePattern: event.currentTarget.value },
              })
            }
          />
        </label>
      ) : (
        <label>
          Sender output regex
          <input
            value={selector.outputRegex}
            placeholder="Capture a relative file path"
            onChange={(event) =>
              onChange({
                ...mechanism,
                fileSelector: { ...selector, outputRegex: event.currentTarget.value },
              })
            }
          />
        </label>
      )}
      <label>
        File description
        <textarea
          value={mechanism.descriptionText}
          onChange={(event) =>
            onChange({ ...mechanism, descriptionText: event.currentTarget.value })
          }
        />
      </label>
      <label>
        Fixed prompt
        <textarea
          value={mechanism.promptText}
          onChange={(event) => onChange({ ...mechanism, promptText: event.currentTarget.value })}
        />
      </label>
      <p>Uses the newest match and checks once immediately after the sender turn finishes.</p>
    </fieldset>
  );
}

interface EditedElement {
  readonly ref: WorkflowElementRef;
  readonly label: string;
}

function BulkActivation({
  elements,
  selection,
  busy,
  error,
  onChange,
  onClose,
  onActivate,
}: {
  readonly elements: readonly EditedElement[];
  readonly selection: ReadonlySet<string>;
  readonly busy: boolean;
  readonly error: string | null;
  onChange(selection: ReadonlySet<string>): void;
  onClose(): void;
  onActivate(): void;
}) {
  const allSelected =
    elements.length > 0 && elements.every((item) => selection.has(elementKey(item.ref)));
  return (
    <section
      className="workflow-bulk-activation"
      role="dialog"
      aria-label="Activate edited elements"
      onClick={(event) => event.stopPropagation()}
    >
      <header>
        <div>
          <p className="eyebrow">Publish changes</p>
          <h2>Edited elements</h2>
        </div>
        <button type="button" aria-label="Close edited elements" onClick={onClose}>
          <X size={18} aria-hidden="true" />
        </button>
      </header>
      <label className="workflow-select-all">
        <input
          type="checkbox"
          checked={allSelected}
          disabled={busy}
          onChange={(event) =>
            onChange(
              event.currentTarget.checked
                ? new Set(elements.map((item) => elementKey(item.ref)))
                : new Set(),
            )
          }
        />
        Select all edited elements
      </label>
      <ul>
        {elements.map((item) => {
          const key = elementKey(item.ref);
          return (
            <li key={key}>
              <label>
                <input
                  type="checkbox"
                  checked={selection.has(key)}
                  disabled={busy}
                  onChange={(event) => {
                    const next = new Set(selection);
                    if (event.currentTarget.checked) next.add(key);
                    else next.delete(key);
                    onChange(next);
                  }}
                />
                <span>{item.label}</span>
                <small>{item.ref.kind}</small>
              </label>
            </li>
          );
        })}
      </ul>
      {error ? (
        <p className="workflow-error" role="alert">
          {error}
        </p>
      ) : null}
      <button
        type="button"
        className="workflow-primary-button"
        disabled={busy || selection.size === 0}
        onClick={onActivate}
      >
        {busy ? 'Activating…' : `Activate selected (${selection.size})`}
      </button>
    </section>
  );
}

function mergeDisplayNodes(
  definition: WorkflowDefinition,
  workingNodes: ReadonlyMap<string, WorkflowNodeConfig>,
): DisplayNode[] {
  const nodes: DisplayNode[] = definition.nodes.flatMap((element) => {
    const persisted = element.draft ?? element.live;
    const config = workingNodes.get(element.id) ?? persisted;
    return config ? [{ config, element, localDraft: workingNodes.has(element.id) }] : [];
  });
  for (const [id, config] of workingNodes) {
    if (!definition.nodes.some((element) => element.id === id))
      nodes.push({ config, element: undefined, localDraft: true });
  }
  return nodes;
}

function mergeDisplayConnections(
  definition: WorkflowDefinition,
  workingConnections: ReadonlyMap<string, WorkflowConnectionConfig>,
): DisplayConnection[] {
  const connections: DisplayConnection[] = definition.connections.flatMap((element) => {
    const persisted = element.draft ?? element.live;
    const config = workingConnections.get(element.id) ?? persisted;
    return config ? [{ config, element, localDraft: workingConnections.has(element.id) }] : [];
  });
  for (const [id, config] of workingConnections) {
    if (!definition.connections.some((element) => element.id === id))
      connections.push({ config, element: undefined, localDraft: true });
  }
  return connections;
}

function groupDisplayConnections(connections: readonly DisplayConnection[]) {
  const groups = new Map<
    string,
    {
      readonly key: string;
      readonly senderNodeId: string;
      readonly endpointNodeId: string | null;
      readonly dangling: boolean;
      connections: DisplayConnection[];
    }
  >();
  for (const connection of connections) {
    const receiverNodeId = connection.config.receiverNodeId;
    const endpointNodeId = receiverNodeId ?? connection.element?.live?.receiverNodeId ?? null;
    const dangling = receiverNodeId === null;
    const key = dangling
      ? `${connection.config.senderNodeId}->dangling:${connection.config.id}`
      : `${connection.config.senderNodeId}->${receiverNodeId}`;
    const existing = groups.get(key);
    if (existing) existing.connections.push(connection);
    else
      groups.set(key, {
        key,
        senderNodeId: connection.config.senderNodeId,
        endpointNodeId,
        dangling,
        connections: [connection],
      });
  }
  return Array.from(groups.values());
}

function newTurnFinishedMechanism(): WorkflowConnectionMechanism {
  return {
    kind: 'turn_finished_expected_file',
    fileSelector: { kind: 'folder_filename_pattern', folder: '', filenamePattern: '' },
    descriptionText: '',
    promptText: '',
    matchSelection: 'newest',
    initialCheck: 'once_immediately',
  };
}

function connectionIsComplete(connection: WorkflowConnectionConfig): boolean {
  const mechanism = connection.mechanism;
  if (!connection.name.trim() || !connection.receiverNodeId || !mechanism) return false;
  const selectorComplete =
    Boolean(mechanism.fileSelector.folder.trim()) &&
    (mechanism.fileSelector.kind === 'folder_filename_pattern'
      ? Boolean(mechanism.fileSelector.filenamePattern.trim())
      : Boolean(mechanism.fileSelector.outputRegex.trim()));
  return Boolean(
    selectorComplete && mechanism.descriptionText.trim() && mechanism.promptText.trim(),
  );
}

function listEditedElements(definition: WorkflowDefinition): EditedElement[] {
  return [
    ...definition.nodes
      .filter((element) => element.hasUnpublishedChanges)
      .map((element) => ({
        ref: { kind: 'node' as const, id: element.id },
        label: element.draft?.name || element.live?.name || 'Deleted node',
      })),
    ...definition.connections
      .filter((element) => element.hasUnpublishedChanges)
      .map((element) => ({
        ref: { kind: 'connection' as const, id: element.id },
        label: element.draft?.name || element.live?.name || 'Deleted connection',
      })),
  ];
}

function expandActivation(
  selected: readonly WorkflowElementRef[],
  definition: WorkflowDefinition,
): WorkflowElementRef[] {
  const result = new Map(selected.map((element) => [elementKey(element), element]));
  for (const selectedElement of selected) {
    if (selectedElement.kind !== 'node') continue;
    const node = definition.nodes.find((candidate) => candidate.id === selectedElement.id);
    if (!node) continue;
    if (node.draft?.isStartingPoint && node.live?.isStartingPoint !== true) {
      for (const candidate of definition.nodes) {
        if (
          candidate.hasUnpublishedChanges &&
          candidate.live?.isStartingPoint !== candidate.draft?.isStartingPoint
        ) {
          const ref = { kind: 'node' as const, id: candidate.id };
          result.set(elementKey(ref), ref);
        }
      }
    }
    if (!node.draft && node.live) {
      for (const connection of definition.connections) {
        if (
          connection.hasUnpublishedChanges &&
          !connection.draft &&
          connection.live?.senderNodeId === node.id
        ) {
          const ref = { kind: 'connection' as const, id: connection.id };
          result.set(elementKey(ref), ref);
        }
      }
    }
  }
  return Array.from(result.values());
}

function elementKey(element: WorkflowElementRef): string {
  return `${element.kind}:${element.id}`;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
