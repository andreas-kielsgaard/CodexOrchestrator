import {
  ArrowLeft,
  Cable,
  ChevronLeft,
  ChevronRight,
  Copy,
  GitBranch,
  Paintbrush,
  Plus,
  Trash2,
  Users,
  X,
} from 'lucide-react';
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentType,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent as ReactPointerEvent,
} from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
import type { HarnessConfigurationCatalogs } from '../../application/conversationHarnesses';
import type {
  WorkflowApplicationClient,
  WorkflowConnectionConfig,
  WorkflowConnectionElement,
  WorkflowConnectionMechanism,
  WorkflowDefinition,
  WorkflowElementRef,
  WorkflowHarnessConfig,
  WorkflowHarnessOverrides,
  WorkflowInstance,
  WorkflowInstanceSummary,
  WorkflowMcpComponent,
  WorkflowNodeConfig,
  WorkflowNodeElement,
  WorkflowNodeHarness,
  WorkflowRole,
  WorkflowTypeSummary,
} from '../../application/workflows';
import {
  createWorkflowAgentSessionClient,
  workflowPersistenceCoordinator,
} from '../../application/workflows';
import type { RepoBranchWorktreeTargetSelectorProps } from '../../application/worktreeTargets';
import { AgentSessionWorkspace, useAgentSession } from '../agentSessions';
import { WorkflowAgentSessionPane } from './WorkflowAgentSessionPane';
import { WorkflowInstanceCreationDialog } from './WorkflowInstanceCreationDialog';
import {
  WorkflowEditorController,
  type WorkflowEditableConfig,
} from './editor/workflowEditorController';
import {
  beginWorkflowNodeDrag,
  projectWorkflowNodeDrag,
  type WorkflowNodeDragPreview,
  type WorkflowNodeDragState,
} from './editor/workflowNodeDrag';
import {
  HarnessDefinitionEditor,
  SearchableSingleSelect,
  type HarnessDefinitionProperty,
} from '../conversationHarnesses/HarnessDefinitionEditor';
import './workflow.css';

export interface WorkflowScreenProps {
  readonly client: WorkflowApplicationClient;
  readonly agentSessionClient?: AgentSessionClient;
  readonly targetSelector?: ComponentType<RepoBranchWorktreeTargetSelectorProps>;
  readonly workflowTypeId: string | null;
  readonly workflowInstanceId?: string | null;
  onOpenWorkflowType(workflowTypeId: string): void;
  onOpenWorkflowInstance?(workflowInstanceId: string): void;
}

type LoadState<T> =
  | { readonly kind: 'loading' }
  | { readonly kind: 'ready'; readonly value: T }
  | { readonly kind: 'failed'; readonly message: string };

interface DisplayNode {
  readonly config: WorkflowNodeConfig;
  readonly effectiveHarness: WorkflowHarnessConfig;
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

interface InstanceConnectionListState {
  readonly groupKey: string;
  readonly connectionIds: readonly string[];
  readonly title: string;
  readonly left: number;
  readonly top: number;
}

export function WorkflowScreen({
  client,
  agentSessionClient,
  targetSelector,
  workflowTypeId,
  workflowInstanceId,
  onOpenWorkflowType,
  onOpenWorkflowInstance,
}: WorkflowScreenProps) {
  const persistentClient = workflowPersistenceCoordinator(client);
  return workflowInstanceId ? (
    <WorkflowInstanceView
      client={persistentClient}
      agentSessionClient={agentSessionClient}
      workflowInstanceId={workflowInstanceId}
    />
  ) : workflowTypeId ? (
    <WorkflowTypeEditor
      client={persistentClient}
      workflowTypeId={workflowTypeId}
      onOpenWorkflowInstance={onOpenWorkflowInstance ?? (() => undefined)}
      targetSelector={targetSelector}
    />
  ) : (
    <WorkflowLanding
      client={persistentClient}
      onOpenWorkflowType={onOpenWorkflowType}
      onOpenWorkflowInstance={onOpenWorkflowInstance ?? (() => undefined)}
      targetSelector={targetSelector}
    />
  );
}

function WorkflowLanding({
  client,
  targetSelector,
  onOpenWorkflowType,
  onOpenWorkflowInstance,
}: Pick<WorkflowScreenProps, 'client' | 'targetSelector' | 'onOpenWorkflowType'> & {
  readonly onOpenWorkflowInstance: (workflowInstanceId: string) => void;
}) {
  const [tab, setTab] = useState<'instances' | 'types'>('instances');
  const [types, setTypes] = useState<LoadState<readonly WorkflowTypeSummary[]>>({
    kind: 'loading',
  });
  const [instances, setInstances] = useState<LoadState<readonly WorkflowInstanceSummary[]>>({
    kind: 'loading',
  });
  const [creationOpen, setCreationOpen] = useState(false);
  const [newName, setNewName] = useState('');
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void client.listWorkflowTypes().then(
      (value) => current && setTypes({ kind: 'ready', value }),
      (error: unknown) => current && setTypes({ kind: 'failed', message: errorMessage(error) }),
    );
    void client.listWorkflowInstances().then(
      (value) => current && setInstances({ kind: 'ready', value }),
      (error: unknown) => current && setInstances({ kind: 'failed', message: errorMessage(error) }),
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
        <button
          className="workflow-primary-button"
          type="button"
          disabled={!targetSelector}
          onClick={() => setCreationOpen(true)}
        >
          <Plus size={16} aria-hidden="true" />
          Create Workflow instance
        </button>
      </header>

      <div className="workflow-tabs" role="tablist" aria-label="Workflow lists">
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'instances'}
          className={tab === 'instances' ? 'active' : undefined}
          onClick={() => setTab('instances')}
        >
          Workflow instances
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

      {tab === 'instances' ? (
        <section className="workflow-type-list" role="tabpanel">
          {instances.kind === 'loading' ? (
            <p className="workflow-status">Loading Workflow instances…</p>
          ) : instances.kind === 'failed' ? (
            <p className="workflow-error" role="alert">
              {instances.message}
            </p>
          ) : instances.value.length === 0 ? (
            <div className="workflow-empty">
              <GitBranch aria-hidden="true" />
              <h2>No Workflow instances</h2>
              <p>Create one from an activated Workflow type.</p>
            </div>
          ) : (
            <ul className="workflow-type-cards" aria-label="Workflow instances">
              {instances.value.map((instance) => (
                <li key={instance.id}>
                  <button type="button" onClick={() => onOpenWorkflowInstance(instance.id)}>
                    <span>
                      <strong>{instance.name}</strong>
                      <small>{instance.workflowTypeName}</small>
                    </span>
                    <span className="workflow-draft-badge">
                      {instance.sessionCount === 0
                        ? 'Ready to begin'
                        : `${instance.sessionCount} ${instance.sessionCount === 1 ? 'Session' : 'Sessions'}`}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
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
      {creationOpen && targetSelector ? (
        <WorkflowInstanceCreationDialog
          workflowTypes={types.kind === 'ready' ? types.value : []}
          TargetSelector={targetSelector}
          onClose={() => setCreationOpen(false)}
          onSubmit={async (input) => {
            const instance = await client.createWorkflowInstance(input);
            setCreationOpen(false);
            onOpenWorkflowInstance(instance.summary.id);
          }}
        />
      ) : null}
    </main>
  );
}

function WorkflowTypeEditor({
  client,
  targetSelector,
  workflowTypeId,
  onOpenWorkflowInstance,
}: Pick<WorkflowScreenProps, 'client' | 'targetSelector'> & {
  readonly workflowTypeId: string;
  readonly onOpenWorkflowInstance: (workflowInstanceId: string) => void;
}) {
  const [load, setLoad] = useState<LoadState<WorkflowDefinition>>({ kind: 'loading' });
  const [nodeBrush, setNodeBrush] = useState(false);
  const [copyBrush, setCopyBrush] = useState(false);
  const [copySourceId, setCopySourceId] = useState<string | null>(null);
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
  const [roles, setRoles] = useState<readonly WorkflowRole[]>([]);
  const [mcpComponents, setMcpComponents] = useState<readonly WorkflowMcpComponent[]>([]);
  const [roleCatalogOpen, setRoleCatalogOpen] = useState(false);
  const [roleCatalogRoleId, setRoleCatalogRoleId] = useState<string | null>(null);
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
  const suppressNodeClickRef = useRef<string | null>(null);
  const nodeDragRef = useRef<WorkflowNodeDragState | null>(null);
  const [nodeDragPreview, setNodeDragPreview] = useState<WorkflowNodeDragPreview | null>(null);
  const definitionRef = useRef<WorkflowDefinition | null>(null);
  const editorControllerRef = useRef<WorkflowEditorController | null>(null);
  if (!editorControllerRef.current) editorControllerRef.current = new WorkflowEditorController();
  const mountedRef = useRef(true);
  const [saving, setSaving] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [creationOpen, setCreationOpen] = useState(false);

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
    editorControllerRef.current?.reset();
    replaceWorkingNodes(new Map());
    replaceWorkingConnections(new Map());
    workingRevisionsRef.current.clear();
    connectionRevisionsRef.current.clear();
    setSelectedNodeId(null);
    setSelectedConnection(null);
    setConnectionList(null);
    void Promise.all([
      client.loadWorkflowType(workflowTypeId),
      client.listRoles(),
      client.listWorkflowMcpComponents(),
    ]).then(
      ([value, loadedRoles, loadedMcpComponents]) => {
        if (!current) return;
        setRoles(loadedRoles);
        setMcpComponents(loadedMcpComponents);
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

  useEffect(() => {
    const handleHistoryShortcut = (event: globalThis.KeyboardEvent) => {
      if (
        !(event.ctrlKey || event.metaKey) ||
        event.altKey ||
        isEditableHistoryTarget(event.target)
      )
        return;
      const key = event.key.toLowerCase();
      const undo = key === 'z' && !event.shiftKey;
      const redo = key === 'y' || (key === 'z' && event.shiftKey);
      if (!undo && !redo) return;
      event.preventDefault();
      const operation = undo
        ? editorControllerRef.current?.undo()
        : editorControllerRef.current?.redo();
      void operation?.catch((error: unknown) => setActionError(errorMessage(error)));
    };
    window.addEventListener('keydown', handleHistoryShortcut);
    return () => window.removeEventListener('keydown', handleHistoryShortcut);
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
  const displayNodes = mergeDisplayNodes(definition, workingNodes, roles);
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

  const updateWorkingNode = (node: WorkflowNodeConfig) =>
    editorControllerRef.current?.changeElement({ kind: 'node', id: node.id }, node);

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

  const updateWorkingConnection = (connection: WorkflowConnectionConfig) =>
    editorControllerRef.current?.changeElement(
      { kind: 'connection', id: connection.id },
      connection,
    );

  const placeNodeAt = (positionX: number, positionY: number) => {
    if ((!nodeBrush && !copyBrush) || saving || selectedNodeId) return;
    const source = copySourceId
      ? displayNodes.find((node) => node.config.id === copySourceId)
      : undefined;
    if (copyBrush && !source) return;
    const id = globalThis.crypto?.randomUUID?.() ?? `node-${Date.now()}`;
    const node: WorkflowNodeConfig = {
      id,
      name: source ? `${source.config.name || 'Node'} copy` : '',
      harnessName: source?.effectiveHarness.identity.name ?? '',
      roleName: source?.config.roleName ?? null,
      positionX,
      positionY,
      isStartingPoint: source ? false : displayNodes.length === 0,
      harness: source
        ? cloneNodeHarness(source.config.harness, source.effectiveHarness)
        : { kind: 'standalone', config: emptyHarness() },
    };
    editorControllerRef.current?.changeElement({ kind: 'node', id }, node, { blocking: true });
    setNodeBrush(false);
    setCopyBrush(false);
    setCopySourceId(null);
    setSelectedNodeId(id);
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
    if ((!nodeBrush && !copyBrush) || saving || event.target !== event.currentTarget) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    placeNodeAt(
      Math.max(24, Math.min(event.clientX - bounds.left, bounds.width - 244)),
      Math.max(28, Math.min(event.clientY - bounds.top, bounds.height - 124)),
    );
  };

  const placeNodeWithKeyboard = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget) return;
    if (
      (!nodeBrush && !copyBrush) ||
      selectedNodeId ||
      saving ||
      (event.key !== 'Enter' && event.key !== ' ')
    )
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
      editorControllerRef.current?.reset();
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
    editorControllerRef.current?.changeElement({ kind: 'connection', id }, connection, {
      blocking: true,
    });
    setConnectionSourceId(null);
    setSelectedConnection({ id, full: true });
    setSelectedNodeId(null);
    setConnectionList(null);
  };

  const handleConnectionNodeClick = (nodeId: string) => {
    if (suppressConnectionClickRef.current) {
      suppressConnectionClickRef.current = false;
      return;
    }
    if (connectionSourceId) createConnection(connectionSourceId, nodeId);
    else setConnectionSourceId(nodeId);
  };

  const handleNodePointerDown = (
    event: ReactPointerEvent<HTMLButtonElement>,
    node: WorkflowNodeConfig,
  ) => {
    if (saving) return;
    if (connectionBrush) {
      dragReconnectConnectionIdRef.current = null;
      dragSourceRef.current = node.id;
      return;
    }
    if (nodeBrush || copyBrush || event.button > 0) return;
    nodeDragRef.current = beginWorkflowNodeDrag({
      nodeId: node.id,
      pointerId: event.pointerId,
      positionX: node.positionX,
      positionY: node.positionY,
      clientX: event.clientX,
      clientY: event.clientY,
    });
    event.currentTarget.setPointerCapture?.(event.pointerId);
  };

  const handleNodePointerMove = (event: ReactPointerEvent<HTMLButtonElement>) => {
    const drag = nodeDragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const bounds = event.currentTarget.parentElement?.getBoundingClientRect();
    if (!bounds) return;
    setNodeDragPreview(
      projectWorkflowNodeDrag(drag, event.clientX, event.clientY, {
        width: bounds.width,
        height: bounds.height,
      }),
    );
  };

  const handleNodePointerUp = (
    event: ReactPointerEvent<HTMLButtonElement>,
    node: WorkflowNodeConfig,
  ) => {
    const reconnectConnectionId = dragReconnectConnectionIdRef.current;
    dragReconnectConnectionIdRef.current = null;
    if (connectionBrush && reconnectConnectionId) {
      const connection = displayConnections.find(
        (candidate) => candidate.config.id === reconnectConnectionId,
      );
      if (connection) {
        suppressConnectionClickRef.current = true;
        const reconnected = { ...connection.config, receiverNodeId: node.id };
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
    if (connectionBrush) {
      if (!source || source === node.id) return;
      suppressConnectionClickRef.current = true;
      createConnection(source, node.id);
      globalThis.setTimeout(() => {
        suppressConnectionClickRef.current = false;
      }, 0);
      return;
    }
    const drag = nodeDragRef.current;
    nodeDragRef.current = null;
    const bounds = event.currentTarget.parentElement?.getBoundingClientRect();
    const preview =
      drag && bounds
        ? projectWorkflowNodeDrag(drag, event.clientX, event.clientY, {
            width: bounds.width,
            height: bounds.height,
          })
        : null;
    setNodeDragPreview(null);
    event.currentTarget.releasePointerCapture?.(event.pointerId);
    if (!drag || drag.pointerId !== event.pointerId || !preview?.moved) return;
    suppressNodeClickRef.current = node.id;
    updateWorkingNode({
      ...node,
      positionX: preview.positionX,
      positionY: preview.positionY,
    });
  };

  const handleNodePointerCancel = (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (nodeDragRef.current?.pointerId !== event.pointerId) return;
    nodeDragRef.current = null;
    setNodeDragPreview(null);
    event.currentTarget.releasePointerCapture?.(event.pointerId);
  };

  const openConnectionList = (state: ConnectionListState) => {
    setSelectedNodeId(null);
    setSelectedConnection(null);
    setHoveredConnectionId(null);
    setConnectionList(state);
  };

  const deletePersistedElement = async (target: WorkflowElementRef) => {
    if (saving) return;
    setSaving(true);
    setActionError(null);
    try {
      const next =
        target.kind === 'node'
          ? await client.deleteNodeDraft(workflowTypeId, target.id)
          : await client.deleteConnectionDraft(workflowTypeId, target.id);
      definitionRef.current = next;
      setLoad({ kind: 'ready', value: next });
      if (target.kind === 'node') {
        const local = new Map(workingNodesRef.current);
        local.delete(target.id);
        replaceWorkingNodes(local);
        setSelectedNodeId(null);
      } else {
        const local = new Map(workingConnectionsRef.current);
        local.delete(target.id);
        replaceWorkingConnections(local);
        setSelectedConnection(null);
      }
    } catch (error) {
      setActionError(errorMessage(error));
      throw error;
    } finally {
      setSaving(false);
    }
  };

  const readElement = (target: WorkflowElementRef): WorkflowEditableConfig | null => {
    if (target.kind === 'node') {
      const local = workingNodesRef.current.get(target.id);
      if (local) return local;
      const element = definitionRef.current?.nodes.find((candidate) => candidate.id === target.id);
      return element?.draft ?? element?.live ?? null;
    }
    const local = workingConnectionsRef.current.get(target.id);
    if (local) return local;
    const element = definitionRef.current?.connections.find(
      (candidate) => candidate.id === target.id,
    );
    return element?.draft ?? element?.live ?? null;
  };

  editorControllerRef.current.configure({
    readElement,
    isPersisted: (target) =>
      target.kind === 'node'
        ? Boolean(definitionRef.current?.nodes.some((candidate) => candidate.id === target.id))
        : Boolean(
            definitionRef.current?.connections.some((candidate) => candidate.id === target.id),
          ),
    dependentConnections: (nodeId) => {
      const ids = new Set<string>();
      const connections: WorkflowConnectionConfig[] = [];
      for (const element of definitionRef.current?.connections ?? []) {
        const connection =
          workingConnectionsRef.current.get(element.id) ?? element.draft ?? element.live;
        if (
          connection &&
          (connection.senderNodeId === nodeId || connection.receiverNodeId === nodeId)
        ) {
          ids.add(connection.id);
          connections.push(connection);
        }
      }
      for (const connection of workingConnectionsRef.current.values())
        if (
          !ids.has(connection.id) &&
          (connection.senderNodeId === nodeId || connection.receiverNodeId === nodeId)
        )
          connections.push(connection);
      return connections;
    },
    saveElement: (target, value, options) => {
      if (target.kind === 'node') {
        const node = value as WorkflowNodeConfig;
        const next = new Map(workingNodesRef.current);
        next.set(node.id, node);
        workingRevisionsRef.current.set(
          node.id,
          (workingRevisionsRef.current.get(node.id) ?? 0) + 1,
        );
        replaceWorkingNodes(next);
        void persistNode(node, options);
      } else {
        const connection = value as WorkflowConnectionConfig;
        const next = new Map(workingConnectionsRef.current);
        next.set(connection.id, connection);
        connectionRevisionsRef.current.set(
          connection.id,
          (connectionRevisionsRef.current.get(connection.id) ?? 0) + 1,
        );
        replaceWorkingConnections(next);
        void persistConnection(connection, options);
      }
    },
    removeLocalElement: (target) => {
      setActionError(null);
      if (target.kind === 'node') {
        const next = new Map(workingNodesRef.current);
        next.delete(target.id);
        workingRevisionsRef.current.set(
          target.id,
          (workingRevisionsRef.current.get(target.id) ?? 0) + 1,
        );
        replaceWorkingNodes(next);
        setSelectedNodeId(null);
      } else {
        const next = new Map(workingConnectionsRef.current);
        next.delete(target.id);
        connectionRevisionsRef.current.set(
          target.id,
          (connectionRevisionsRef.current.get(target.id) ?? 0) + 1,
        );
        replaceWorkingConnections(next);
        setSelectedConnection(null);
      }
    },
    deletePersistedElement,
  });

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
        <button
          type="button"
          className="workflow-primary-button"
          disabled={!definition.activeRecipe || saving || !targetSelector}
          onClick={() => setCreationOpen(true)}
        >
          Create instance
        </button>
      </header>

      <div className="workflow-editor__toolbar" role="toolbar" aria-label="Workflow tools">
        <button
          type="button"
          aria-pressed={nodeBrush}
          className={nodeBrush ? 'active' : undefined}
          disabled={saving}
          onClick={() => {
            setNodeBrush((current) => !current);
            setCopyBrush(false);
            setCopySourceId(null);
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
            setCopyBrush(false);
            setCopySourceId(null);
            setConnectionSourceId(null);
          }}
        >
          <Cable size={17} aria-hidden="true" />
          Connection
        </button>
        <button
          type="button"
          aria-pressed={copyBrush}
          className={copyBrush ? 'active' : undefined}
          disabled={saving || displayNodes.length === 0}
          onClick={() => {
            setCopyBrush((current) => !current);
            setNodeBrush(false);
            setConnectionBrush(false);
            setConnectionSourceId(null);
            if (copyBrush) setCopySourceId(null);
          }}
        >
          <Copy size={17} aria-hidden="true" />
          Copy
        </button>
        <button
          type="button"
          aria-expanded={roleCatalogOpen}
          disabled={saving}
          onClick={() => {
            setRoleCatalogRoleId(null);
            setRoleCatalogOpen((current) => !current);
          }}
        >
          <Users size={17} aria-hidden="true" />
          Roles
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
          {copyBrush
            ? copySourceId
              ? 'Copy brush active · click a different source or place its copy on the canvas'
              : 'Copy brush active · select a source node'
            : connectionBrush
              ? connectionSourceId
                ? 'Connection brush active · select a target node'
                : 'Connection brush active · select a source, then a target, or drag between nodes'
              : nodeBrush
                ? 'Node brush active · click the canvas to place nodes'
                : 'Select a brush to edit the workflow'}
        </p>
      </div>

      <div
        className={`workflow-canvas${nodeBrush || connectionBrush || copyBrush ? ' has-node-brush' : ''}`}
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
            const selected = group.connections.some(
              (connection) => connection.config.id === selectedConnection?.id,
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
                aria-pressed={selected}
                className={`${highlighted ? 'is-highlighted ' : ''}${selected ? 'is-selected ' : ''}${group.dangling ? 'is-dangling' : ''}`}
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

        {displayNodes.map(({ config, effectiveHarness, element, localDraft }) => (
          <button
            key={config.id}
            type="button"
            disabled={saving}
            className={`workflow-node${config.isStartingPoint ? ' is-start' : ''}${selectedNodeId === config.id ? ' is-selected' : ''}${connectionSourceId === config.id ? ' is-connection-source' : ''}${nodeDragPreview?.nodeId === config.id ? ' is-dragging' : ''}${element?.draft === null && element.live ? ' is-deleted' : ''}`}
            style={{
              left:
                nodeDragPreview?.nodeId === config.id
                  ? nodeDragPreview.positionX
                  : config.positionX,
              top:
                nodeDragPreview?.nodeId === config.id
                  ? nodeDragPreview.positionY
                  : config.positionY,
            }}
            onPointerDown={(event) => handleNodePointerDown(event, config)}
            onPointerMove={handleNodePointerMove}
            onPointerUp={(event) => handleNodePointerUp(event, config)}
            onPointerCancel={handleNodePointerCancel}
            onClick={(event) => {
              event.stopPropagation();
              if (suppressNodeClickRef.current === config.id) {
                suppressNodeClickRef.current = null;
                return;
              }
              if (connectionBrush) {
                handleConnectionNodeClick(config.id);
                return;
              }
              if (copyBrush) {
                setCopySourceId(config.id);
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
            aria-pressed={selectedNodeId === config.id}
          >
            <span className="workflow-node__badges">
              {config.isStartingPoint ? <small>Start</small> : null}
              {localDraft || element?.hasUnpublishedChanges ? (
                <small className="is-draft">Draft</small>
              ) : null}
              {element?.draft === null && element.live ? <small>Delete</small> : null}
            </span>
            <strong>{effectiveHarness.identity.name || 'Choose a role'}</strong>
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
            effectiveHarness={selectedNode.effectiveHarness}
            roles={roles}
            mcpComponents={mcpComponents}
            persistedElement={selectedNode.element}
            locallyChanged={selectedNode.localDraft}
            busy={saving}
            error={actionError}
            onChange={updateWorkingNode}
            onBeginFieldEdit={() =>
              editorControllerRef.current?.beginFieldEdit({
                kind: 'node',
                id: selectedNode.config.id,
              })
            }
            onCommitFieldEdit={() =>
              editorControllerRef.current?.commitFieldEdit({
                kind: 'node',
                id: selectedNode.config.id,
              })
            }
            onClose={() => {
              const localDraft = workingNodesRef.current.get(selectedNode.config.id);
              if (localDraft) void persistNode(localDraft, { blocking: true, closeAfter: true });
              else setSelectedNodeId(null);
            }}
            onSave={() => void persistNode(selectedNode.config, { blocking: true })}
            onActivate={() => void activateElements([{ kind: 'node', id: selectedNode.config.id }])}
            onDelete={() =>
              void editorControllerRef.current
                ?.deleteElement({ kind: 'node', id: selectedNode.config.id })
                .catch((error: unknown) => setActionError(errorMessage(error)))
            }
            onDetach={async () => {
              setSaving(true);
              setActionError(null);
              try {
                const next = await client.detachNodeRole(workflowTypeId, selectedNode.config.id);
                definitionRef.current = next;
                setLoad({ kind: 'ready', value: next });
              } catch (error) {
                setActionError(errorMessage(error));
              } finally {
                setSaving(false);
              }
            }}
            onSaveAsRole={async (roleName) => {
              setSaving(true);
              setActionError(null);
              try {
                const next = await client.saveNodeAsRole(
                  workflowTypeId,
                  selectedNode.config.id,
                  roleName,
                );
                const nextRoles = await client.listRoles();
                setRoles(nextRoles);
                definitionRef.current = next;
                setLoad({ kind: 'ready', value: next });
              } catch (error) {
                setActionError(errorMessage(error));
              } finally {
                setSaving(false);
              }
            }}
            onEditRole={(roleId) => {
              setRoleCatalogRoleId(roleId);
              setRoleCatalogOpen(true);
            }}
          />
        ) : null}

        {roleCatalogOpen ? (
          <RoleCatalog
            client={client}
            roles={roles}
            mcpComponents={mcpComponents}
            busy={saving}
            onBusy={setSaving}
            onRoles={setRoles}
            onDefinition={(next) => {
              definitionRef.current = next;
              setLoad({ kind: 'ready', value: next });
            }}
            workflowTypeId={workflowTypeId}
            initialRoleId={roleCatalogRoleId}
            onClose={() => setRoleCatalogOpen(false)}
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
              mcpComponents={mcpComponents}
              busy={saving}
              error={actionError}
              onChange={updateWorkingConnection}
              onBeginFieldEdit={() =>
                editorControllerRef.current?.beginFieldEdit({
                  kind: 'connection',
                  id: selectedConnectionDisplay.config.id,
                })
              }
              onCommitFieldEdit={() =>
                editorControllerRef.current?.commitFieldEdit({
                  kind: 'connection',
                  id: selectedConnectionDisplay.config.id,
                })
              }
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
              onDelete={() =>
                void editorControllerRef.current
                  ?.deleteElement({
                    kind: 'connection',
                    id: selectedConnectionDisplay.config.id,
                  })
                  .catch((error: unknown) => setActionError(errorMessage(error)))
              }
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
      {creationOpen && targetSelector ? (
        <WorkflowInstanceCreationDialog
          workflowTypes={[definition.workflowType]}
          TargetSelector={targetSelector}
          onClose={() => setCreationOpen(false)}
          onSubmit={async (input) => {
            const instance = await client.createWorkflowInstance(input);
            setCreationOpen(false);
            onOpenWorkflowInstance(instance.summary.id);
          }}
        />
      ) : null}
    </main>
  );
}

function WorkflowInstanceView({
  client,
  agentSessionClient,
  workflowInstanceId,
}: {
  readonly client: WorkflowApplicationClient;
  readonly agentSessionClient?: AgentSessionClient;
  readonly workflowInstanceId: string;
}) {
  const [load, setLoad] = useState<LoadState<WorkflowInstance>>({ kind: 'loading' });
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [connectionList, setConnectionList] = useState<InstanceConnectionListState | null>(null);
  const [selectedConnectionId, setSelectedConnectionId] = useState<string | null>(null);
  const [hoveredConnectionId, setHoveredConnectionId] = useState<string | null>(null);
  const nodeTriggers = useRef(new Map<string, HTMLButtonElement>());
  const connectionTriggers = useRef(new Map<string, SVGGElement>());
  const reloadInstance = useCallback(async () => {
    try {
      const value = await client.loadWorkflowInstance(workflowInstanceId);
      setLoad({ kind: 'ready', value });
    } catch (error) {
      setLoad({ kind: 'failed', message: errorMessage(error) });
    }
  }, [client, workflowInstanceId]);
  useEffect(() => {
    let current = true;
    void client.loadWorkflowInstance(workflowInstanceId).then(
      (value) => current && setLoad({ kind: 'ready', value }),
      (error: unknown) => current && setLoad({ kind: 'failed', message: errorMessage(error) }),
    );
    return () => {
      current = false;
    };
  }, [client, workflowInstanceId]);

  const loadedInstance = load.kind === 'ready' ? load.value : null;
  const selectedNode = selectedNodeId
    ? (loadedInstance?.recipe.nodes.find((node) => node.id === selectedNodeId) ?? null)
    : null;
  const selectedNodeIdForClient = selectedNode?.id ?? null;
  const workflowBoundAgentSessionClient = useMemo(
    () =>
      selectedNodeIdForClient && agentSessionClient
        ? createWorkflowAgentSessionClient(agentSessionClient, client, {
            workflowInstanceId,
            nodeId: selectedNodeIdForClient,
          })
        : null,
    [agentSessionClient, client, selectedNodeIdForClient, workflowInstanceId],
  );

  if (load.kind === 'loading')
    return (
      <main className="workflow-screen workflow-status" aria-label="Workflow instance">
        Loading Workflow instance…
      </main>
    );
  if (load.kind === 'failed')
    return (
      <main className="workflow-screen workflow-error" aria-label="Workflow instance">
        {load.message}
      </main>
    );

  const instance = load.value;
  const connectionGroups = groupRecipeConnections(instance.recipe.connections);
  const selectedConnection = selectedConnectionId
    ? (instance.recipe.connections.find((connection) => connection.id === selectedConnectionId) ??
      null)
    : null;
  const closeSelectedNode = () => {
    const trigger = selectedNodeId ? nodeTriggers.current.get(selectedNodeId) : undefined;
    setSelectedNodeId(null);
    trigger?.focus();
  };
  const closeConnectionDetails = () => {
    const group = selectedConnectionId
      ? connectionGroups.find((candidate) =>
          candidate.connections.some((connection) => connection.id === selectedConnectionId),
        )
      : null;
    setSelectedConnectionId(null);
    if (group) connectionTriggers.current.get(group.key)?.focus();
  };
  const closeCanvasOverlays = () => {
    if (selectedNodeId) closeSelectedNode();
    else if (selectedConnectionId) closeConnectionDetails();
    else if (connectionList) {
      const trigger = connectionTriggers.current.get(connectionList.groupKey);
      setConnectionList(null);
      setHoveredConnectionId(null);
      trigger?.focus();
    }
  };
  return (
    <main
      className="workflow-screen workflow-editor workflow-instance"
      aria-label={`Workflow instance ${instance.summary.name}`}
    >
      <header className="workflow-editor__header">
        <div>
          <p className="eyebrow">
            {instance.summary.workflowTypeName} · Recipe {instance.recipe.ordinal}
          </p>
          <h1>{instance.summary.name}</h1>
        </div>
        <div className="workflow-editor__summary" aria-label="Workflow instance summary">
          <span>{instance.sessions.length === 0 ? 'Ready to begin' : 'In progress'}</span>
          <span>{instance.summary.activeSessionCount} active</span>
          <span>{instance.summary.idleSessionCount} idle</span>
        </div>
      </header>
      <section className="workflow-instance__target" aria-label="Workflow target">
        <div>
          <strong>{instance.target.repository.name}</strong>
          <span>{instance.target.branch.name}</span>
        </div>
        <dl>
          <div>
            <dt>Repository</dt>
            <dd>{instance.target.repository.name}</dd>
          </div>
          <div>
            <dt>Branch</dt>
            <dd>{instance.target.branch.name}</dd>
          </div>
          <div>
            <dt>Worktree</dt>
            <dd>{instance.target.worktree.path}</dd>
          </div>
        </dl>
      </section>
      <div
        className="workflow-canvas workflow-instance__canvas"
        aria-label="Workflow instance graph"
        onClick={closeCanvasOverlays}
      >
        <svg className="workflow-canvas__connections" aria-label="Workflow instance connections">
          <defs>
            <marker
              id="workflow-instance-arrow"
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
            const sender = instance.recipe.nodes.find((node) => node.id === group.senderNodeId);
            const receiver = instance.recipe.nodes.find((node) => node.id === group.receiverNodeId);
            if (!sender || !receiver) return null;
            const label = `${group.connections.length} connection${group.connections.length === 1 ? '' : 's'} from ${sender.name} to ${receiver.name}`;
            const highlighted = group.connections.some(
              (connection) => connection.id === hoveredConnectionId,
            );
            const openList = () => {
              setSelectedNodeId(null);
              setSelectedConnectionId(null);
              setConnectionList({
                groupKey: group.key,
                connectionIds: group.connections.map((connection) => connection.id),
                title: label,
                left: (sender.positionX + receiver.positionX) / 2 + 84,
                top: (sender.positionY + receiver.positionY) / 2 + 56,
              });
            };
            return (
              <g
                key={group.key}
                ref={(element) => {
                  if (element) connectionTriggers.current.set(group.key, element);
                  else connectionTriggers.current.delete(group.key);
                }}
                role="button"
                tabIndex={0}
                aria-label={label}
                className={highlighted ? 'is-highlighted' : undefined}
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
                  x1={sender.positionX + 210}
                  y1={sender.positionY + 56}
                  x2={receiver.positionX}
                  y2={receiver.positionY + 56}
                  markerEnd="url(#workflow-instance-arrow)"
                />
                <line
                  className="workflow-connection__hitbox"
                  x1={sender.positionX + 210}
                  y1={sender.positionY + 56}
                  x2={receiver.positionX}
                  y2={receiver.positionY + 56}
                />
                {group.connections.length > 1 ? (
                  <text
                    x={(sender.positionX + 210 + receiver.positionX) / 2}
                    y={(sender.positionY + receiver.positionY) / 2 + 48}
                  >
                    {group.connections.length}
                  </text>
                ) : null}
              </g>
            );
          })}
        </svg>
        {instance.recipe.nodes.map((node) => {
          const sessions = instance.sessions.filter((session) => session.nodeId === node.id);
          const active = sessions.filter((session) => session.activity === 'active').length;
          return (
            <button
              type="button"
              key={node.id}
              ref={(element) => {
                if (element) nodeTriggers.current.set(node.id, element);
                else nodeTriggers.current.delete(node.id);
              }}
              className={`workflow-node workflow-instance-node${node.isStartingPoint ? ' is-start' : ''}`}
              style={{ left: node.positionX, top: node.positionY }}
              aria-label={`Open ${node.harness.identity.name || 'Harness'} node ${node.name}`}
              onClick={(event) => {
                event.stopPropagation();
                setConnectionList(null);
                setSelectedConnectionId(null);
                setSelectedNodeId(node.id);
              }}
            >
              <span className="workflow-node__badges">
                {node.isStartingPoint ? <small>Start</small> : null}
              </span>
              <strong>{node.harness.identity.name || 'Harness'}</strong>
              <span>{node.name}</span>
              <small>
                {sessions.length} {sessions.length === 1 ? 'Session' : 'Sessions'} · {active} active
                · {sessions.length - active} idle
              </small>
            </button>
          );
        })}
        {selectedNode ? (
          <WorkflowInstanceNodePopup
            key={selectedNode.id}
            node={selectedNode}
            sessions={instance.sessions.filter((session) => session.nodeId === selectedNode.id)}
            agentSessionClient={workflowBoundAgentSessionClient ?? undefined}
            allowEmptySession={selectedNode.isStartingPoint && instance.sessions.length === 0}
            onSessionCreated={() => reloadInstance()}
            onClose={closeSelectedNode}
          />
        ) : null}
        {connectionList ? (
          <WorkflowInstanceConnectionList
            state={connectionList}
            connections={instance.recipe.connections}
            nodes={instance.recipe.nodes}
            onHover={setHoveredConnectionId}
            onClose={() => {
              const trigger = connectionTriggers.current.get(connectionList.groupKey);
              setConnectionList(null);
              setHoveredConnectionId(null);
              trigger?.focus();
            }}
            onOpen={(connectionId) => {
              setConnectionList(null);
              setHoveredConnectionId(null);
              setSelectedConnectionId(connectionId);
            }}
          />
        ) : null}
        {selectedConnection ? (
          <WorkflowInstanceConnectionPopup
            key={selectedConnection.id}
            connection={selectedConnection}
            activations={instance.connectionActivations
              .filter((activation) => activation.connectionId === selectedConnection.id)
              .sort(
                (left, right) =>
                  right.requestedAt.localeCompare(left.requestedAt) ||
                  right.id.localeCompare(left.id),
              )}
            agentSessionClient={agentSessionClient}
            onClose={closeConnectionDetails}
          />
        ) : null}
      </div>
    </main>
  );
}

function WorkflowInstanceNodePopup({
  node,
  sessions,
  agentSessionClient,
  allowEmptySession,
  onSessionCreated,
  onClose,
}: {
  readonly node: WorkflowInstance['recipe']['nodes'][number];
  readonly sessions: WorkflowInstance['sessions'];
  readonly agentSessionClient?: AgentSessionClient;
  readonly allowEmptySession: boolean;
  readonly onSessionCreated: (sessionId: string) => void | Promise<void>;
  readonly onClose: () => void;
}) {
  const [view, setView] = useState<'configuration' | 'sessions'>('configuration');
  const popupRef = useRef<HTMLElement>(null);
  const orderedSessions = [...sessions].sort((left, right) =>
    right.associatedAt.localeCompare(left.associatedAt),
  );
  const activeCount = sessions.filter((session) => session.activity === 'active').length;
  useEffect(() => {
    const popup = popupRef.current;
    if (!popup) return;
    if (view === 'configuration') popup.focus();
    else popup.querySelector<HTMLElement>(workflowPopupFocusableSelector)?.focus();
  }, [view]);
  return (
    <aside
      ref={popupRef}
      className={`workflow-instance-node-popup${view === 'sessions' ? ' is-sessions' : ''}`}
      role="dialog"
      aria-label={`${node.harness.identity.name || 'Harness'} node details`}
      tabIndex={-1}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => containWorkflowPopupFocus(event, popupRef.current, onClose)}
    >
      {view === 'configuration' ? (
        <>
          <header>
            <div>
              <p className="eyebrow">Harness</p>
              <h2>{node.harness.identity.name || 'Harness'}</h2>
              <p>{node.name}</p>
            </div>
            <button type="button" aria-label="Close node details" onClick={onClose}>
              <X size={16} aria-hidden="true" />
            </button>
          </header>
          <section
            className="workflow-instance-node-popup__activity"
            aria-label="Node Session activity"
          >
            <strong>
              {sessions.length} {sessions.length === 1 ? 'Session' : 'Sessions'}
            </strong>
            <span>{activeCount} active</span>
            <span>{sessions.length - activeCount} idle</span>
          </section>
          <dl className="workflow-instance-node-popup__configuration">
            <div>
              <dt>Role identity</dt>
              <dd>{node.harness.runtime.authoritySummary || 'Not defined'}</dd>
            </div>
            <div>
              <dt>Capabilities</dt>
              <dd>
                {node.harness.skills.items.length} Skills ·{' '}
                {node.harness.tools.mcpServers?.length ?? 0} MCP Servers ·{' '}
                {node.harness.hooks.length} Hooks
              </dd>
            </div>
            <div>
              <dt>Runtime</dt>
              <dd>{node.harness.runtime.defaultModel || 'Harness default'}</dd>
            </div>
          </dl>
          <footer>
            <button
              className="workflow-primary-button"
              type="button"
              disabled={!agentSessionClient || (sessions.length === 0 && !allowEmptySession)}
              onClick={() => setView('sessions')}
            >
              Agent Sessions
            </button>
          </footer>
        </>
      ) : agentSessionClient ? (
        <WorkflowAgentSessionPane
          node={{
            id: node.id,
            name: node.name,
            harnessName: node.harness.identity.name || 'Harness',
          }}
          sessions={orderedSessions}
          client={agentSessionClient}
          allowEmptySession={allowEmptySession}
          onSessionCreated={onSessionCreated}
          onReturn={() => setView('configuration')}
          onClose={onClose}
        />
      ) : null}
    </aside>
  );
}

function WorkflowInstanceConnectionList({
  state,
  connections,
  nodes,
  onHover,
  onClose,
  onOpen,
}: {
  readonly state: InstanceConnectionListState;
  readonly connections: WorkflowInstance['recipe']['connections'];
  readonly nodes: WorkflowInstance['recipe']['nodes'];
  onHover(id: string | null): void;
  onClose(): void;
  onOpen(id: string): void;
}) {
  const popupRef = useRef<HTMLElement>(null);
  const listed = state.connectionIds.flatMap((id) => {
    const connection = connections.find((candidate) => candidate.id === id);
    return connection ? [connection] : [];
  });
  useEffect(() => {
    popupRef.current?.querySelector<HTMLElement>('li > button')?.focus();
  }, []);
  return (
    <section
      ref={popupRef}
      className="workflow-connection-list workflow-instance-connection-list"
      role="dialog"
      aria-label={state.title}
      style={{ left: state.left, top: state.top }}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => containWorkflowPopupFocus(event, popupRef.current, onClose)}
    >
      <header>
        <strong>{state.title}</strong>
        <button type="button" aria-label="Close connection list" onClick={onClose}>
          <X size={15} aria-hidden="true" />
        </button>
      </header>
      <ul>
        {listed.map((connection) => {
          const receiver = nodes.find((node) => node.id === connection.receiverNodeId);
          return (
            <li key={connection.id}>
              <button
                type="button"
                onMouseEnter={() => onHover(connection.id)}
                onMouseLeave={(event) => {
                  if (document.activeElement !== event.currentTarget) onHover(null);
                }}
                onFocus={() => onHover(connection.id)}
                onBlur={() => onHover(null)}
                onClick={() => onOpen(connection.id)}
              >
                <span>{connection.name || 'Unnamed connection'}</span>
                <small>To {receiver?.name || 'receiver'}</small>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

function WorkflowInstanceConnectionPopup({
  connection,
  activations,
  agentSessionClient,
  onClose,
}: {
  readonly connection: WorkflowInstance['recipe']['connections'][number];
  readonly activations: WorkflowInstance['connectionActivations'];
  readonly agentSessionClient?: AgentSessionClient;
  readonly onClose: () => void;
}) {
  const [activationIndex, setActivationIndex] = useState(0);
  const [showAll, setShowAll] = useState(false);
  const [returnFocusTarget, setReturnFocusTarget] = useState<'source' | 'target' | null>(null);
  const [inspection, setInspection] = useState<{
    readonly sessionId: string;
    readonly invocationId: string;
    readonly label: string;
    readonly returnTo: 'source' | 'target';
  } | null>(null);
  const popupRef = useRef<HTMLElement>(null);
  const selected = activations[activationIndex] ?? null;
  useEffect(() => {
    popupRef.current?.focus();
  }, []);
  useEffect(() => {
    if (inspection || !returnFocusTarget) return;
    const timeout = window.setTimeout(() => {
      popupRef.current
        ?.querySelector<HTMLElement>(`[data-activation-endpoint="${returnFocusTarget}"]`)
        ?.focus();
      setReturnFocusTarget(null);
    }, 0);
    return () => window.clearTimeout(timeout);
  }, [inspection, returnFocusTarget]);

  return (
    <aside
      ref={popupRef}
      className={`workflow-instance-node-popup workflow-instance-connection-popup${inspection ? ' is-sessions' : ''}`}
      role="dialog"
      aria-label={`${connection.name || 'Connection'} activity`}
      tabIndex={-1}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => containWorkflowPopupFocus(event, popupRef.current, onClose)}
    >
      {inspection && agentSessionClient ? (
        <WorkflowConnectionTurnInspection
          endpoint={inspection}
          client={agentSessionClient}
          onReturn={() => {
            setReturnFocusTarget(inspection.returnTo);
            setInspection(null);
          }}
          onClose={onClose}
        />
      ) : (
        <>
          <header>
            <div>
              <p className="eyebrow">Connection</p>
              <h2>{connection.name || 'Unnamed connection'}</h2>
              <p>Runtime activation history</p>
            </div>
            <button type="button" aria-label="Close connection activity" onClick={onClose}>
              <X size={16} aria-hidden="true" />
            </button>
          </header>
          {selected ? (
            <>
              <div className="workflow-instance-connection-popup__activation-nav">
                <button
                  type="button"
                  disabled={activationIndex === 0}
                  aria-label="Show newer activation"
                  onClick={() => setActivationIndex((current) => Math.max(0, current - 1))}
                >
                  <ChevronLeft size={15} aria-hidden="true" />
                  Newer
                </button>
                <strong>
                  Activation {activationIndex + 1} of {activations.length}
                </strong>
                <button
                  type="button"
                  disabled={activationIndex >= activations.length - 1}
                  aria-label="Show older activation"
                  onClick={() =>
                    setActivationIndex((current) => Math.min(activations.length - 1, current + 1))
                  }
                >
                  Older
                  <ChevronRight size={15} aria-hidden="true" />
                </button>
              </div>
              <dl className="workflow-instance-node-popup__configuration">
                <div>
                  <dt>Status</dt>
                  <dd>{connectionActivationStatusLabel(selected.status)}</dd>
                </div>
                <div>
                  <dt>Requested</dt>
                  <dd>
                    <time dateTime={selected.requestedAt}>
                      {new Date(selected.requestedAt).toLocaleString()}
                    </time>
                  </dd>
                </div>
                <div>
                  <dt>Resolved file</dt>
                  <dd>{selected.resolvedFilePath || 'Not resolved'}</dd>
                </div>
                <div>
                  <dt>Delivery</dt>
                  <dd>{selected.deliveryKind}</dd>
                </div>
                <div>
                  <dt>Context</dt>
                  <dd>
                    {[selected.sessionMode, selected.contextInheritance, selected.compression]
                      .filter(Boolean)
                      .join(' Â· ') || 'Not recorded'}
                  </dd>
                </div>
              </dl>
              <section
                className="workflow-instance-connection-popup__endpoints"
                aria-label="Activation Agent Sessions"
              >
                <button
                  type="button"
                  data-activation-endpoint="source"
                  aria-label="Sending Agent Session"
                  disabled={!agentSessionClient}
                  onClick={() => {
                    setReturnFocusTarget(null);
                    setInspection({
                      sessionId: selected.sourceSessionId,
                      invocationId: selected.sourceInvocationId,
                      label: 'Sending Agent Session',
                      returnTo: 'source',
                    });
                  }}
                >
                  <span>Sending Agent Session</span>
                  <small>
                    {selected.sourceSessionId} Â· {selected.sourceInvocationId}
                  </small>
                </button>
                <button
                  type="button"
                  data-activation-endpoint="target"
                  aria-label="Receiving Agent Session"
                  disabled={
                    !agentSessionClient || !selected.targetSessionId || !selected.targetInvocationId
                  }
                  onClick={() => {
                    if (!selected.targetSessionId || !selected.targetInvocationId) return;
                    setReturnFocusTarget(null);
                    setInspection({
                      sessionId: selected.targetSessionId,
                      invocationId: selected.targetInvocationId,
                      label: 'Receiving Agent Session',
                      returnTo: 'target',
                    });
                  }}
                >
                  <span>Receiving Agent Session</span>
                  <small>
                    {selected.targetSessionId && selected.targetInvocationId
                      ? `${selected.targetSessionId} Â· ${selected.targetInvocationId}`
                      : 'No target recorded'}
                  </small>
                </button>
              </section>
              <button
                className="workflow-list-secondary workflow-instance-connection-popup__all-toggle"
                type="button"
                aria-expanded={showAll}
                onClick={() => setShowAll((current) => !current)}
              >
                All activations
              </button>
              {showAll ? (
                <ol className="workflow-instance-connection-popup__all">
                  {activations.map((activation, index) => (
                    <li key={activation.id}>
                      <button
                        type="button"
                        className={index === activationIndex ? 'is-selected' : undefined}
                        aria-pressed={index === activationIndex}
                        onClick={() => setActivationIndex(index)}
                      >
                        <span>
                          {connectionActivationStatusLabel(activation.status)} Â·{' '}
                          {new Date(activation.requestedAt).toLocaleString()}
                        </span>
                        <small>{activation.resolvedFilePath || 'No resolved file'}</small>
                      </button>
                    </li>
                  ))}
                </ol>
              ) : null}
            </>
          ) : (
            <p className="workflow-instance-node-popup__empty">
              This connection has not fired in this workflow instance.
            </p>
          )}
        </>
      )}
    </aside>
  );
}

function WorkflowConnectionTurnInspection({
  endpoint,
  client,
  onReturn,
  onClose,
}: {
  readonly endpoint: {
    readonly sessionId: string;
    readonly invocationId: string;
    readonly label: string;
    readonly returnTo: 'source' | 'target';
  };
  readonly client: AgentSessionClient;
  readonly onReturn: () => void;
  readonly onClose: () => void;
}) {
  const controller = useAgentSession(client, { selectedSessionId: endpoint.sessionId });
  return (
    <div className="workflow-instance-node-popup__full-session">
      <header className="workflow-instance-node-popup__session-toolbar">
        <button type="button" autoFocus onClick={onReturn}>
          <ArrowLeft size={15} aria-hidden="true" />
          Return to activation
        </button>
        <div>
          <strong>{endpoint.label}</strong>
          <span>Exact recorded turn</span>
        </div>
        <button type="button" aria-label="Close connection activity" onClick={onClose}>
          <X size={16} aria-hidden="true" />
        </button>
      </header>
      <AgentSessionWorkspace
        controller={controller}
        inspection={{ sessionId: endpoint.sessionId, invocationId: endpoint.invocationId }}
      />
    </div>
  );
}

function containWorkflowPopupFocus(
  event: KeyboardEvent<HTMLElement>,
  popup: HTMLElement | null,
  onClose: () => void,
) {
  if (event.key === 'Escape') {
    event.preventDefault();
    event.stopPropagation();
    onClose();
    return;
  }
  if (event.key !== 'Tab' || !popup) return;
  const focusable = [...popup.querySelectorAll<HTMLElement>(workflowPopupFocusableSelector)];
  if (focusable.length === 0) {
    event.preventDefault();
    popup.focus();
    return;
  }
  const first = focusable[0];
  const last = focusable.at(-1)!;
  const active = document.activeElement;
  if (event.shiftKey && (active === first || active === popup)) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && (active === last || !popup.contains(active))) {
    event.preventDefault();
    first.focus();
  }
}

const workflowPopupFocusableSelector = [
  'button:not([disabled])',
  'a[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

function NodeConfiguration({
  node,
  effectiveHarness,
  roles,
  mcpComponents,
  persistedElement,
  locallyChanged,
  busy,
  error,
  onChange,
  onBeginFieldEdit,
  onCommitFieldEdit,
  onClose,
  onSave,
  onActivate,
  onDelete,
  onDetach,
  onSaveAsRole,
  onEditRole,
}: {
  readonly node: WorkflowNodeConfig;
  readonly effectiveHarness: WorkflowHarnessConfig;
  readonly roles: readonly WorkflowRole[];
  readonly mcpComponents: readonly WorkflowMcpComponent[];
  readonly persistedElement?: WorkflowNodeElement;
  readonly locallyChanged: boolean;
  readonly busy: boolean;
  readonly error: string | null;
  onChange(node: WorkflowNodeConfig): void;
  onBeginFieldEdit(): void;
  onCommitFieldEdit(): void;
  onClose(): void;
  onSave(): void;
  onActivate(): void;
  onDelete(): void;
  onDetach(): void;
  onSaveAsRole(roleName: string): void;
  onEditRole(roleId: string): void;
}) {
  const [newRoleName, setNewRoleName] = useState('');
  const pendingDeletion = Boolean(persistedElement?.live && !persistedElement.draft);
  const boundRoleId = node.harness?.kind === 'role' ? node.harness.roleId : null;
  const valid = Boolean(node.name.trim() && effectiveHarness.identity.name.trim());
  const catalogs = workflowHarnessCatalogs(mcpComponents);
  const canActivate = Boolean(
    (valid || pendingDeletion) && persistedElement?.hasUnpublishedChanges && !locallyChanged,
  );

  return (
    <section
      className="workflow-node-config"
      role="dialog"
      aria-label={node.name ? `Configure ${node.name}` : 'Configure new node'}
      onClick={(event) => event.stopPropagation()}
      onFocusCapture={(event) => {
        if (isEditableHistoryTarget(event.target)) onBeginFieldEdit();
      }}
      onBlurCapture={(event) => {
        if (isEditableHistoryTarget(event.target)) onCommitFieldEdit();
      }}
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

      <SearchableSingleSelect
        label="Harness source"
        options={[
          ...(roles.length
            ? [
                {
                  value: 'existing_role' as const,
                  label: 'Existing role',
                  description: 'Inherit with field overrides.',
                },
              ]
            : []),
          {
            value: 'from_scratch' as const,
            label: 'From scratch',
            description: 'Use a standalone Harness definition.',
          },
        ]}
        value={node.harness?.kind === 'role' ? 'existing_role' : 'from_scratch'}
        editable={!busy && !pendingDeletion && !(node.harness?.kind === 'role' && locallyChanged)}
        onChange={(source) => {
          if (source === 'existing_role') {
            const role = roles[0];
            if (role) onChange(bindNodeToRole(node, role));
          } else if (source === 'from_scratch' && node.harness?.kind === 'role') {
            onDetach();
          }
        }}
      />
      {!roles.length ? <small>Create a saved Role before selecting one.</small> : null}
      {node.harness?.kind === 'role' && locallyChanged ? (
        <small>Save the node draft before changing its Harness source.</small>
      ) : null}

      {node.harness?.kind === 'role' ? (
        <SearchableSingleSelect
          label="Saved Role"
          options={roles.map((role) => ({ value: role.id, label: role.name }))}
          value={node.harness.roleId}
          editable={!busy && !pendingDeletion}
          unavailableReason="Create a saved Role first."
          onChange={(roleId) => {
            const role = roles.find((candidate) => candidate.id === roleId);
            if (role) onChange(bindNodeToRole(node, role));
          }}
        />
      ) : null}

      <label>
        Node name
        <input
          value={node.name}
          placeholder="For example, Review proposed architecture"
          disabled={busy || pendingDeletion}
          onChange={(event) => onChange({ ...node, name: event.currentTarget.value })}
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
              isStartingPoint: event.currentTarget.checked,
            })
          }
        />
        Starting node
        <small>The activated workflow has exactly one starting node.</small>
      </label>

      {!pendingDeletion ? (
        <HarnessDefinitionEditor
          configuration={effectiveHarness}
          catalogs={catalogs}
          editable={!busy}
          mcpComponents={mcpComponents}
          provenance={definitionProvenance(node)}
          onResetProperty={(property) => onChange(removeHarnessPropertyOverride(node, property))}
          onChange={(configuration) =>
            onChange(updateNodeHarnessDefinition(node, effectiveHarness, configuration))
          }
        />
      ) : null}

      {!pendingDeletion ? (
        <section className="workflow-role-actions" aria-label="Role actions">
          {boundRoleId ? (
            <div>
              <button type="button" disabled={busy} onClick={() => onEditRole(boundRoleId)}>
                Edit saved Role
              </button>
              <button type="button" disabled={busy} onClick={onDetach}>
                Detach from Role
              </button>
            </div>
          ) : null}
          <label>
            Save configuration as a new Role
            <span>
              <input
                value={newRoleName}
                placeholder="New Role name"
                disabled={busy}
                onChange={(event) => setNewRoleName(event.currentTarget.value)}
              />
              <button
                type="button"
                disabled={busy || !newRoleName.trim() || locallyChanged}
                onClick={() => onSaveAsRole(newRoleName.trim())}
              >
                Save as Role
              </button>
            </span>
            {locallyChanged ? <small>Save the node draft before creating a Role.</small> : null}
          </label>
        </section>
      ) : null}

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
              <button type="button" onClick={onDelete} disabled={busy}>
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

function RoleCatalog({
  client,
  roles,
  mcpComponents,
  busy,
  workflowTypeId,
  initialRoleId,
  onBusy,
  onRoles,
  onDefinition,
  onClose,
}: {
  readonly client: WorkflowApplicationClient;
  readonly roles: readonly WorkflowRole[];
  readonly mcpComponents: readonly WorkflowMcpComponent[];
  readonly busy: boolean;
  readonly workflowTypeId: string;
  readonly initialRoleId: string | null;
  onBusy(value: boolean): void;
  onRoles(roles: readonly WorkflowRole[]): void;
  onDefinition(definition: WorkflowDefinition): void;
  onClose(): void;
}) {
  const [selectedId, setSelectedId] = useState<string | null>(
    initialRoleId ?? roles[0]?.id ?? null,
  );
  const selected = roles.find((role) => role.id === selectedId);
  const [draftName, setDraftName] = useState(selected?.name ?? '');
  const [draftHarness, setDraftHarness] = useState<WorkflowHarnessConfig>(
    selected?.harness ?? emptyHarness(),
  );
  const [creating, setCreating] = useState(roles.length === 0);
  const [error, setError] = useState<string | null>(null);
  const catalogs = workflowHarnessCatalogs(mcpComponents);

  const choose = (role: WorkflowRole) => {
    setSelectedId(role.id);
    setDraftName(role.name);
    setDraftHarness(role.harness);
    setCreating(false);
    setError(null);
  };
  const startNew = () => {
    setSelectedId(null);
    setDraftName('');
    setDraftHarness(emptyHarness());
    setCreating(true);
    setError(null);
  };
  const save = async () => {
    if (!draftName.trim() || !draftHarness.identity.name.trim() || busy) return;
    onBusy(true);
    setError(null);
    try {
      const saved = creating
        ? await client.createRole({ name: draftName.trim(), harness: draftHarness })
        : await client.updateRole({
            roleId: selectedId!,
            name: draftName.trim(),
            harness: draftHarness,
          });
      const nextRoles = await client.listRoles();
      onRoles(nextRoles);
      choose(saved);
      onDefinition(await client.loadWorkflowType(workflowTypeId));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      onBusy(false);
    }
  };

  return (
    <section
      className="workflow-role-catalog"
      role="dialog"
      aria-label="Saved Roles"
      onClick={(event) => event.stopPropagation()}
    >
      <header>
        <div>
          <p className="eyebrow">Harness catalog</p>
          <h2>Saved Roles</h2>
        </div>
        <button type="button" aria-label="Close saved Roles" onClick={onClose}>
          <X size={18} aria-hidden="true" />
        </button>
      </header>
      <div className="workflow-role-catalog__body">
        <nav aria-label="Saved Role list">
          <button type="button" className={creating ? 'active' : undefined} onClick={startNew}>
            <Plus size={14} aria-hidden="true" /> New Role
          </button>
          <SearchableSingleSelect
            label="Saved Role"
            options={roles.map((role) => ({ value: role.id, label: role.name }))}
            value={creating ? null : selectedId}
            editable={!busy}
            unavailableReason="No saved Roles yet."
            onChange={(roleId) => {
              const role = roles.find((candidate) => candidate.id === roleId);
              if (role) choose(role);
            }}
          />
        </nav>
        <div className="workflow-role-editor">
          <label>
            Role name
            <input
              aria-label="Role name"
              value={draftName}
              disabled={busy}
              onChange={(event) => setDraftName(event.currentTarget.value)}
            />
          </label>
          <HarnessDefinitionEditor
            configuration={draftHarness}
            catalogs={catalogs}
            editable={!busy}
            mcpComponents={mcpComponents}
            onChange={setDraftHarness}
          />
          {error ? (
            <p className="workflow-error" role="alert">
              {error}
            </p>
          ) : null}
          <button
            type="button"
            className="workflow-primary-button"
            disabled={busy || !draftName.trim() || !draftHarness.identity.name.trim()}
            onClick={() => void save()}
          >
            {busy ? 'Saving…' : creating ? 'Create Role' : 'Save Role'}
          </button>
        </div>
      </div>
    </section>
  );
}

function bindNodeToRole(node: WorkflowNodeConfig, role: WorkflowRole): WorkflowNodeConfig {
  return {
    ...node,
    harnessName: role.harness.identity.name,
    roleName: role.name,
    harness: { kind: 'role', roleId: role.id, overrides: {} },
  };
}

function definitionProvenance(
  node: WorkflowNodeConfig,
): Record<HarnessDefinitionProperty, 'inherited' | 'overridden' | 'instance'> {
  const properties: readonly HarnessDefinitionProperty[] = [
    'identityName',
    'permittedAgentNames',
    'visualIdentity',
    'promptPrefixContent',
    'skillDiscoveryPolicy',
    'skillItems',
    'toolDiscoveryPolicy',
    'toolItems',
    'mcpServers',
    'runtimeModelPolicyMode',
    'runtimeModels',
    'runtimeDefaultModel',
    'runtimeDefaultReasoning',
    'runtimeSandbox',
    'runtimeAuthoritySummary',
    'hookItems',
  ];
  if (node.harness?.kind !== 'role')
    return Object.fromEntries(properties.map((property) => [property, 'instance'])) as Record<
      HarnessDefinitionProperty,
      'instance'
    >;
  const overrides = node.harness.overrides;
  return Object.fromEntries(
    properties.map((property) => [
      property,
      propertyIsOverridden(overrides, property) ? 'overridden' : 'inherited',
    ]),
  ) as Record<HarnessDefinitionProperty, 'inherited' | 'overridden'>;
}

function removeHarnessPropertyOverride(
  node: WorkflowNodeConfig,
  property: HarnessDefinitionProperty,
): WorkflowNodeConfig {
  if (node.harness?.kind !== 'role') return node;
  const overrides = { ...node.harness.overrides };
  delete (overrides as Record<string, unknown>)[property];
  deleteLegacyAlias(overrides, property);
  return { ...node, harness: { ...node.harness, overrides } };
}

function updateNodeHarnessDefinition(
  node: WorkflowNodeConfig,
  effective: WorkflowHarnessConfig,
  next: WorkflowHarnessConfig,
): WorkflowNodeConfig {
  if (node.harness?.kind !== 'role')
    return {
      ...node,
      harnessName: next.identity.name,
      roleName: null,
      harness: { kind: 'standalone', config: next },
    };
  const overrides = { ...node.harness.overrides };
  const properties: readonly HarnessDefinitionProperty[] = [
    'identityName',
    'permittedAgentNames',
    'visualIdentity',
    'promptPrefixContent',
    'skillDiscoveryPolicy',
    'skillItems',
    'toolDiscoveryPolicy',
    'toolItems',
    'mcpServers',
    'runtimeModelPolicyMode',
    'runtimeModels',
    'runtimeDefaultModel',
    'runtimeDefaultReasoning',
    'runtimeSandbox',
    'runtimeAuthoritySummary',
    'hookItems',
  ];
  for (const property of properties) {
    const previousValue = harnessProperty(effective, property);
    const nextValue = harnessProperty(next, property);
    if (JSON.stringify(previousValue) !== JSON.stringify(nextValue)) {
      Object.assign(overrides, { [property]: nextValue });
      deleteLegacyAlias(overrides, property);
    }
  }
  return {
    ...node,
    harnessName: next.identity.name,
    harness: { ...node.harness, overrides },
  };
}

function harnessProperty(
  harness: WorkflowHarnessConfig,
  property: HarnessDefinitionProperty,
): unknown {
  switch (property) {
    case 'identityName':
      return harness.identity.name;
    case 'permittedAgentNames':
      return harness.identity.permittedAgentNames;
    case 'visualIdentity':
      return harness.identity.visualIdentity;
    case 'promptPrefixContent':
      return harness.promptPrefix.content;
    case 'skillDiscoveryPolicy':
      return harness.skills.availableDiscoveryPolicy;
    case 'skillItems':
      return harness.skills.items;
    case 'toolDiscoveryPolicy':
      return harness.tools.availableDiscoveryPolicy;
    case 'toolItems':
      return harness.tools.items;
    case 'mcpServers':
      return harness.tools.mcpServers ?? [];
    case 'runtimeModelPolicyMode':
      return harness.runtime.modelPolicyMode;
    case 'runtimeModels':
      return harness.runtime.models;
    case 'runtimeDefaultModel':
      return harness.runtime.defaultModel;
    case 'runtimeDefaultReasoning':
      return harness.runtime.defaultReasoning;
    case 'runtimeSandbox':
      return harness.runtime.sandbox;
    case 'runtimeAuthoritySummary':
      return harness.runtime.authoritySummary;
    case 'hookItems':
      return harness.hooks;
  }
}

function propertyIsOverridden(
  overrides: WorkflowHarnessOverrides,
  property: HarnessDefinitionProperty,
): boolean {
  if (Object.prototype.hasOwnProperty.call(overrides, property)) return true;
  switch (property) {
    case 'identityName':
      return overrides.harnessName !== undefined;
    case 'promptPrefixContent':
      return overrides.instructions !== undefined;
    case 'skillItems':
      return overrides.skills !== undefined;
    case 'runtimeModels':
    case 'runtimeDefaultModel':
    case 'runtimeDefaultReasoning':
      return overrides.runtime !== undefined;
    case 'runtimeAuthoritySummary':
      return overrides.roleIdentity !== undefined;
    case 'hookItems':
      return overrides.hooks !== undefined;
    default:
      return false;
  }
}

function deleteLegacyAlias(
  overrides: WorkflowHarnessOverrides,
  property: HarnessDefinitionProperty,
) {
  const mutable = overrides as {
    harnessName?: unknown;
    roleIdentity?: unknown;
    instructions?: unknown;
    skills?: unknown;
    hooks?: unknown;
    runtime?: unknown;
  };
  if (property === 'identityName') delete mutable.harnessName;
  if (property === 'promptPrefixContent') delete mutable.instructions;
  if (property === 'skillItems') delete mutable.skills;
  if (property === 'runtimeAuthoritySummary') delete mutable.roleIdentity;
  if (
    property === 'runtimeModels' ||
    property === 'runtimeDefaultModel' ||
    property === 'runtimeDefaultReasoning'
  )
    delete mutable.runtime;
  if (property === 'hookItems') delete mutable.hooks;
}

function emptyHarness(): WorkflowHarnessConfig {
  return {
    identity: {
      name: '',
      machineKey: '',
      permittedAgentNames: null,
      visualIdentity: null,
    },
    promptPrefix: {
      content: '',
      initialDelivery: 'prepend',
      contextCompressionDelivery: 'deferred',
    },
    skills: { availableDiscoveryPolicy: 'whitelist', items: [] },
    tools: {
      availableDiscoveryPolicy: 'whitelist',
      items: [],
      schemaBoundary: 'Tool schemas remain runtime-owned.',
      mcpServers: [],
    },
    runtime: {
      modelPolicyMode: 'revision_owned',
      models: [
        {
          modelId: 'gpt-5.6-terra',
          allowed: true,
          minReasoning: 'low',
          maxReasoning: 'xhigh',
        },
        {
          modelId: 'gpt-5.6-sol',
          allowed: true,
          minReasoning: 'medium',
          maxReasoning: 'xhigh',
        },
      ],
      defaultModel: null,
      defaultReasoning: null,
      sandbox: 'workspace_write',
      sandboxOptions: ['read_only', 'workspace_write', 'danger_full_access'],
      approvalPolicy: 'never',
      approvalPolicyOptions: ['never'],
      authoritySummary: '',
    },
    hooks: [],
    updatePolicy: {
      status: 'not_configured',
      reason: 'Session replacement applies activated Workflow Harness changes.',
    },
  };
}

function workflowHarnessCatalogs(
  mcpComponents: readonly WorkflowMcpComponent[],
): HarnessConfigurationCatalogs {
  const toolNames = [...new Set(mcpComponents.map((component) => component.toolName))];
  return {
    agentNames: {
      source: 'not_connected',
      items: [],
      reason: 'Agent name catalog unavailable.',
    },
    agentVisualIdentities: {
      source: 'not_connected',
      items: [],
      reason: 'Visual identity catalog unavailable.',
    },
    skills: {
      source: 'not_connected',
      items: [],
      reason: 'Skill catalog unavailable.',
    },
    tools: {
      source: mcpComponents.length ? 'workflow_mcp_component_catalog' : 'not_connected',
      items: toolNames.map((name) => ({
        name,
        description: mcpComponents.find((component) => component.toolName === name)?.title ?? '',
      })),
      reason: mcpComponents.length
        ? 'Application-owned Workflow MCP components.'
        : 'Workflow MCP component catalog unavailable.',
    },
    models: {
      source: 'workflow_runtime_catalog',
      items: [
        {
          id: 'gpt-5.6-terra',
          label: 'GPT-5.6 Terra',
          reasoningLevels: ['low', 'medium', 'high', 'xhigh'],
        },
        {
          id: 'gpt-5.6-sol',
          label: 'GPT-5.6 Sol',
          reasoningLevels: ['medium', 'high', 'xhigh'],
        },
      ],
      reason: 'Models currently supported by Workflow runtime launch.',
    },
  };
}

function cloneNodeHarness(
  harness: WorkflowNodeHarness | null | undefined,
  effective: WorkflowHarnessConfig,
): WorkflowNodeHarness {
  if (!harness) return { kind: 'standalone', config: structuredClone(effective) };
  return structuredClone(harness);
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
          ? connectionMechanismLabel(connection.config.mechanism)
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
  mcpComponents,
  busy,
  error,
  onChange,
  onBeginFieldEdit,
  onCommitFieldEdit,
  onClose,
  onSave,
  onActivate,
  onDelete,
}: {
  readonly connection: WorkflowConnectionConfig;
  readonly persistedElement?: WorkflowConnectionElement;
  readonly locallyChanged: boolean;
  readonly nodes: readonly DisplayNode[];
  readonly mcpComponents: readonly WorkflowMcpComponent[];
  readonly busy: boolean;
  readonly error: string | null;
  onChange(connection: WorkflowConnectionConfig): void;
  onBeginFieldEdit(): void;
  onCommitFieldEdit(): void;
  onClose(): void;
  onSave(): void;
  onActivate(): void;
  onDelete(): void;
}) {
  const pendingDeletion = Boolean(persistedElement?.live && !persistedElement.draft);
  const mechanism = connection.mechanism;
  const sender = nodes.find((node) => node.config.id === connection.senderNodeId);
  const availableMcpComponents = mcpComponents.filter(
    (component) =>
      component.participationMode === 'native' &&
      component.interfaceId === 'prompt_agent_files_and_text/v1' &&
      sender !== undefined &&
      harnessExposesMcpTool(sender.effectiveHarness, component.serverName, component.toolName),
  );
  const complete = connectionIsComplete(connection, availableMcpComponents);
  const canActivate = Boolean(
    persistedElement?.hasUnpublishedChanges && !locallyChanged && (complete || pendingDeletion),
  );
  const setMechanism = (next: WorkflowConnectionMechanism | null) =>
    onChange({ ...connection, mechanism: next });

  return (
    <section
      className="workflow-node-config workflow-connection-config"
      role="dialog"
      aria-label={`Configure ${connection.name || 'connection'}`}
      onClick={(event) => event.stopPropagation()}
      onFocusCapture={(event) => {
        if (isEditableHistoryTarget(event.target)) onBeginFieldEdit();
      }}
      onBlurCapture={(event) => {
        if (isEditableHistoryTarget(event.target)) onCommitFieldEdit();
      }}
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
              value={sender?.config.name || sender?.config.harnessName || connection.senderNodeId}
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
                    : event.currentTarget.value === 'mcp_native_prompt_agent'
                      ? newMcpNativePromptAgentMechanism()
                      : null,
                )
              }
            >
              <option value="">Not configured</option>
              <option value="turn_finished_expected_file">Turn finished · expected file</option>
              <option value="mcp_native_prompt_agent">MCP · native agent handoff</option>
            </select>
          </label>
          {mechanism?.kind === 'turn_finished_expected_file' ? (
            <TurnFinishedMechanismFields
              mechanism={mechanism}
              busy={busy}
              onChange={setMechanism}
            />
          ) : mechanism?.kind === 'mcp_native_prompt_agent' ? (
            <McpNativePromptAgentMechanismFields
              mechanism={mechanism}
              components={availableMcpComponents}
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
              <button type="button" onClick={onDelete} disabled={busy}>
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
  readonly mechanism: Extract<
    WorkflowConnectionMechanism,
    { readonly kind: 'turn_finished_expected_file' }
  >;
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

function McpNativePromptAgentMechanismFields({
  mechanism,
  components,
  busy,
  onChange,
}: {
  readonly mechanism: Extract<
    WorkflowConnectionMechanism,
    { readonly kind: 'mcp_native_prompt_agent' }
  >;
  readonly components: readonly WorkflowMcpComponent[];
  readonly busy: boolean;
  onChange(mechanism: WorkflowConnectionMechanism): void;
}) {
  const selectedKey = mcpComponentKey(mechanism.serverName, mechanism.toolName);
  const selectedAvailable = components.some(
    (component) =>
      component.serverName === mechanism.serverName && component.toolName === mechanism.toolName,
  );
  return (
    <fieldset className="workflow-mechanism-fields" disabled={busy}>
      <legend>Native MCP handoff</legend>
      <label>
        MCP component
        <select
          value={selectedAvailable ? selectedKey : ''}
          onChange={(event) => {
            const component = components.find(
              (candidate) =>
                mcpComponentKey(candidate.serverName, candidate.toolName) ===
                event.currentTarget.value,
            );
            onChange({
              ...mechanism,
              serverName: component?.serverName ?? '',
              toolName: component?.toolName ?? '',
            });
          }}
        >
          <option value="">
            {components.length === 0
              ? 'No compatible component exposed by sender Harness'
              : 'Choose a component'}
          </option>
          {components.map((component) => (
            <option
              key={mcpComponentKey(component.serverName, component.toolName)}
              value={mcpComponentKey(component.serverName, component.toolName)}
            >
              {component.title} · {component.serverName}/{component.toolName}
            </option>
          ))}
        </select>
      </label>
      {mechanism.serverName && mechanism.toolName && !selectedAvailable ? (
        <p className="workflow-draft-note">
          The configured component is not exposed by the sender Harness.
        </p>
      ) : null}
      <label>
        Connection warning
        <textarea
          value={mechanism.warningText ?? ''}
          placeholder="Use the default workflow warning"
          onChange={(event) =>
            onChange({
              ...mechanism,
              warningText: event.currentTarget.value || null,
            })
          }
        />
      </label>
      <p>Leave the warning blank to use the application default.</p>
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
  roles: readonly WorkflowRole[],
): DisplayNode[] {
  const nodes: DisplayNode[] = definition.nodes.flatMap((element) => {
    const persisted = element.draft ?? element.live;
    const config = workingNodes.get(element.id) ?? persisted;
    const locallyChanged = workingNodes.has(element.id);
    const effectiveHarness = locallyChanged
      ? resolveNodeHarness(config!, roles)
      : element.draft
        ? (element.draftEffectiveHarness ?? resolveNodeHarness(config!, roles))
        : (element.liveEffectiveHarness ?? resolveNodeHarness(config!, roles));
    return config ? [{ config, effectiveHarness, element, localDraft: locallyChanged }] : [];
  });
  for (const [id, config] of workingNodes) {
    if (!definition.nodes.some((element) => element.id === id))
      nodes.push({
        config,
        effectiveHarness: resolveNodeHarness(config, roles),
        element: undefined,
        localDraft: true,
      });
  }
  return nodes;
}

function resolveNodeHarness(
  node: WorkflowNodeConfig,
  roles: readonly WorkflowRole[],
): WorkflowHarnessConfig {
  if (node.harness?.kind === 'standalone') return node.harness.config;
  if (node.harness?.kind === 'role') {
    const binding = node.harness;
    const role = roles.find((candidate) => candidate.id === binding.roleId);
    if (role) {
      let resolved: WorkflowHarnessConfig = structuredClone(role.harness);
      const overrides = binding.overrides;
      if (overrides.harnessName !== undefined)
        resolved = {
          ...resolved,
          identity: { ...resolved.identity, name: overrides.harnessName ?? '' },
        };
      if (overrides.instructions !== undefined)
        resolved = {
          ...resolved,
          promptPrefix: {
            ...resolved.promptPrefix,
            content: overrides.instructions ?? '',
          },
        };
      if (overrides.skills)
        resolved = {
          ...resolved,
          skills: {
            ...resolved.skills,
            items: overrides.skills.map((name) => ({
              name,
              path: name,
              purpose: '',
              useWhen: '',
              policy: 'available',
            })),
          },
        };
      if (overrides.mcpServers)
        resolved = {
          ...resolved,
          tools: { ...resolved.tools, mcpServers: overrides.mcpServers },
        };
      if (overrides.hooks)
        resolved = {
          ...resolved,
          hooks: overrides.hooks.map((name) => ({
            name,
            status: 'exposed',
            detail: '',
          })),
        };
      if (overrides.runtime) resolved = applyLegacyRuntimeOverride(resolved, overrides.runtime);
      if (overrides.roleIdentity !== undefined)
        resolved = {
          ...resolved,
          runtime: {
            ...resolved.runtime,
            authoritySummary: overrides.roleIdentity ?? '',
          },
        };
      if (overrides.identityName !== undefined)
        resolved = {
          ...resolved,
          identity: { ...resolved.identity, name: overrides.identityName ?? '' },
        };
      if (overrides.identityMachineKey !== undefined)
        resolved = {
          ...resolved,
          identity: { ...resolved.identity, machineKey: overrides.identityMachineKey ?? '' },
        };
      if (Object.prototype.hasOwnProperty.call(overrides, 'permittedAgentNames'))
        resolved = {
          ...resolved,
          identity: {
            ...resolved.identity,
            permittedAgentNames: overrides.permittedAgentNames ?? null,
          },
        };
      if (Object.prototype.hasOwnProperty.call(overrides, 'visualIdentity'))
        resolved = {
          ...resolved,
          identity: { ...resolved.identity, visualIdentity: overrides.visualIdentity ?? null },
        };
      if (overrides.promptPrefixContent !== undefined)
        resolved = {
          ...resolved,
          promptPrefix: { ...resolved.promptPrefix, content: overrides.promptPrefixContent ?? '' },
        };
      if (overrides.skillDiscoveryPolicy)
        resolved = {
          ...resolved,
          skills: {
            ...resolved.skills,
            availableDiscoveryPolicy: overrides.skillDiscoveryPolicy,
          },
        };
      if (overrides.skillItems)
        resolved = { ...resolved, skills: { ...resolved.skills, items: overrides.skillItems } };
      if (overrides.toolDiscoveryPolicy)
        resolved = {
          ...resolved,
          tools: { ...resolved.tools, availableDiscoveryPolicy: overrides.toolDiscoveryPolicy },
        };
      if (overrides.toolItems)
        resolved = { ...resolved, tools: { ...resolved.tools, items: overrides.toolItems } };
      if (overrides.toolSchemaBoundary !== undefined)
        resolved = {
          ...resolved,
          tools: { ...resolved.tools, schemaBoundary: overrides.toolSchemaBoundary ?? '' },
        };
      if (overrides.runtimeModelPolicyMode)
        resolved = {
          ...resolved,
          runtime: { ...resolved.runtime, modelPolicyMode: overrides.runtimeModelPolicyMode },
        };
      if (overrides.runtimeModels)
        resolved = {
          ...resolved,
          runtime: { ...resolved.runtime, models: overrides.runtimeModels },
        };
      if (Object.prototype.hasOwnProperty.call(overrides, 'runtimeDefaultModel'))
        resolved = {
          ...resolved,
          runtime: { ...resolved.runtime, defaultModel: overrides.runtimeDefaultModel ?? null },
        };
      if (Object.prototype.hasOwnProperty.call(overrides, 'runtimeDefaultReasoning'))
        resolved = {
          ...resolved,
          runtime: {
            ...resolved.runtime,
            defaultReasoning: overrides.runtimeDefaultReasoning ?? null,
          },
        };
      if (overrides.runtimeSandbox)
        resolved = {
          ...resolved,
          runtime: { ...resolved.runtime, sandbox: overrides.runtimeSandbox },
        };
      if (overrides.runtimeSandboxOptions)
        resolved = {
          ...resolved,
          runtime: { ...resolved.runtime, sandboxOptions: overrides.runtimeSandboxOptions },
        };
      if (overrides.runtimeApprovalPolicy)
        resolved = {
          ...resolved,
          runtime: { ...resolved.runtime, approvalPolicy: overrides.runtimeApprovalPolicy },
        };
      if (overrides.runtimeApprovalPolicyOptions)
        resolved = {
          ...resolved,
          runtime: {
            ...resolved.runtime,
            approvalPolicyOptions: overrides.runtimeApprovalPolicyOptions,
          },
        };
      if (overrides.runtimeAuthoritySummary !== undefined)
        resolved = {
          ...resolved,
          runtime: {
            ...resolved.runtime,
            authoritySummary: overrides.runtimeAuthoritySummary ?? '',
          },
        };
      if (overrides.hookItems) resolved = { ...resolved, hooks: overrides.hookItems };
      if (overrides.updatePolicy) resolved = { ...resolved, updatePolicy: overrides.updatePolicy };
      return resolved;
    }
  }
  const empty = emptyHarness();
  return {
    ...empty,
    identity: {
      ...empty.identity,
      name: node.harnessName,
      machineKey: node.id,
    },
    runtime: {
      ...empty.runtime,
      authoritySummary: node.roleName ?? '',
    },
  };
}

function applyLegacyRuntimeOverride(
  harness: WorkflowHarnessConfig,
  runtime: NonNullable<WorkflowHarnessOverrides['runtime']>,
): WorkflowHarnessConfig {
  const reasoning = ['low', 'medium', 'high', 'xhigh'].includes(runtime.reasoningEffort)
    ? (runtime.reasoningEffort as WorkflowHarnessConfig['runtime']['defaultReasoning'])
    : null;
  return {
    ...harness,
    runtime: {
      ...harness.runtime,
      models: runtime.model
        ? [
            {
              modelId: runtime.model,
              allowed: true,
              minReasoning: 'low',
              maxReasoning: 'xhigh',
            },
          ]
        : [],
      defaultModel: runtime.model || null,
      defaultReasoning: reasoning,
    },
  };
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

function groupRecipeConnections(connections: WorkflowInstance['recipe']['connections']) {
  const groups = new Map<
    string,
    {
      readonly key: string;
      readonly senderNodeId: string;
      readonly receiverNodeId: string | null;
      connections: WorkflowInstance['recipe']['connections'][number][];
    }
  >();
  for (const connection of connections) {
    const key = `${connection.senderNodeId}->${connection.receiverNodeId}`;
    const existing = groups.get(key);
    if (existing) existing.connections.push(connection);
    else
      groups.set(key, {
        key,
        senderNodeId: connection.senderNodeId,
        receiverNodeId: connection.receiverNodeId,
        connections: [connection],
      });
  }
  return Array.from(groups.values());
}

function connectionActivationStatusLabel(
  status: WorkflowInstance['connectionActivations'][number]['status'],
): string {
  switch (status) {
    case 'requested':
      return 'Requested';
    case 'resolved':
      return 'File resolved';
    case 'associated':
      return 'Session associated';
    case 'launch_requested':
      return 'Launch requested';
    case 'launch_accepted':
      return 'Launch accepted';
    case 'failed':
      return 'Failed';
  }
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

function newMcpNativePromptAgentMechanism(): WorkflowConnectionMechanism {
  return {
    kind: 'mcp_native_prompt_agent',
    serverName: '',
    toolName: '',
    warningText: null,
  };
}

function connectionMechanismLabel(mechanism: WorkflowConnectionMechanism): string {
  return mechanism.kind === 'turn_finished_expected_file'
    ? 'Turn finished · expected file'
    : 'MCP · native agent handoff';
}

function mcpComponentKey(serverName: string, toolName: string): string {
  return JSON.stringify([serverName, toolName]);
}

function harnessExposesMcpTool(
  harness: WorkflowHarnessConfig,
  serverName: string,
  toolName: string,
): boolean {
  const server = harness.tools.mcpServers?.find((candidate) => candidate.serverName === serverName);
  return Boolean(
    server &&
    (server.access.kind === 'entire_server' || server.access.toolNames.includes(toolName)),
  );
}

function connectionIsComplete(
  connection: WorkflowConnectionConfig,
  availableMcpComponents: readonly WorkflowMcpComponent[],
): boolean {
  const mechanism = connection.mechanism;
  if (!connection.name.trim() || !connection.receiverNodeId || !mechanism) return false;
  if (mechanism.kind === 'mcp_native_prompt_agent')
    return availableMcpComponents.some(
      (component) =>
        component.serverName === mechanism.serverName && component.toolName === mechanism.toolName,
    );
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

function isEditableHistoryTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.matches('input, textarea, select') || target.isContentEditable)
  );
}
