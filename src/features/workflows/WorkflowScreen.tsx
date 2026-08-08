import { GitBranch, Paintbrush, Plus, X } from 'lucide-react';
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
  WorkflowDefinition,
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
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [workingNodes, setWorkingNodes] = useState<ReadonlyMap<string, WorkflowNodeConfig>>(
    () => new Map(),
  );
  const workingNodesRef = useRef<ReadonlyMap<string, WorkflowNodeConfig>>(new Map());
  const workingRevisionsRef = useRef<Map<string, number>>(new Map());
  const definitionRef = useRef<WorkflowDefinition | null>(null);
  const mountedRef = useRef(true);
  const [saving, setSaving] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const replaceWorkingNodes = useCallback((next: ReadonlyMap<string, WorkflowNodeConfig>) => {
    workingNodesRef.current = next;
    setWorkingNodes(next);
  }, []);

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

  useEffect(() => {
    let current = true;
    setLoad({ kind: 'loading' });
    definitionRef.current = null;
    replaceWorkingNodes(new Map());
    workingRevisionsRef.current.clear();
    setSelectedNodeId(null);
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
  }, [client, replaceWorkingNodes, workflowTypeId]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      for (const node of workingNodesRef.current.values())
        void queueNodeSave(node).catch(() => undefined);
    };
  }, [queueNodeSave]);

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
  const selectedNode = selectedNodeId
    ? displayNodes.find((node) => node.config.id === selectedNodeId)
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
    if (selectedNodeId) {
      if (!saving && selectedNode) {
        const locallyChanged = workingNodesRef.current.get(selectedNode.config.id);
        if (locallyChanged) void persistNode(locallyChanged, { blocking: true, closeAfter: true });
        else setSelectedNodeId(null);
      }
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

  const activateNode = async (nodeId: string) => {
    if (saving) return;
    setSaving(true);
    setActionError(null);
    try {
      const changedStartNodes = definition.nodes.filter((element) => {
        if (!element.hasUnpublishedChanges || !element.draft) return false;
        return element.live?.isStartingPoint !== element.draft.isStartingPoint;
      });
      const ids = new Set([nodeId, ...changedStartNodes.map((element) => element.id)]);
      const next = await client.activateChanges(
        workflowTypeId,
        Array.from(ids, (id) => ({ kind: 'node' as const, id })),
      );
      definitionRef.current = next;
      setLoad({ kind: 'ready', value: next });
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
          onClick={() => setNodeBrush((current) => !current)}
        >
          <Paintbrush size={17} aria-hidden="true" />
          Node
        </button>
        <p id="workflow-canvas-instructions">
          {nodeBrush
            ? 'Node brush active · click the canvas to place nodes'
            : 'Select the node brush to place a node'}
        </p>
      </div>

      <div
        className={`workflow-canvas${nodeBrush ? ' has-node-brush' : ''}`}
        aria-label="Workflow canvas"
        aria-describedby="workflow-canvas-instructions"
        tabIndex={0}
        onClick={placeNode}
        onKeyDown={placeNodeWithKeyboard}
      >
        <svg className="workflow-canvas__connections" aria-hidden="true">
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
          {definition.connections.map((element) => {
            const connection = element.draft ?? element.live;
            const sender = displayNodes.find((node) => node.config.id === connection?.senderNodeId);
            const receiver = displayNodes.find(
              (node) => node.config.id === connection?.receiverNodeId,
            );
            if (!sender || !receiver) return null;
            return (
              <line
                key={element.id}
                x1={sender.config.positionX + 210}
                y1={sender.config.positionY + 46}
                x2={receiver.config.positionX}
                y2={receiver.config.positionY + 46}
                markerEnd="url(#workflow-arrow)"
              />
            );
          })}
        </svg>

        {displayNodes.map(({ config, element, localDraft }) => (
          <button
            key={config.id}
            type="button"
            disabled={saving}
            className={`workflow-node${config.isStartingPoint ? ' is-start' : ''}`}
            style={{ left: config.positionX, top: config.positionY }}
            onClick={(event) => {
              event.stopPropagation();
              setSelectedNodeId(config.id);
              setActionError(null);
            }}
            aria-label={`Configure ${config.name || 'new node'}`}
          >
            <span className="workflow-node__badges">
              {config.isStartingPoint ? <small>Start</small> : null}
              {localDraft || element?.hasUnpublishedChanges ? (
                <small className="is-draft">Draft</small>
              ) : null}
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
            onActivate={() => void activateNode(selectedNode.config.id)}
          />
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
}) {
  const valid = Boolean(node.name.trim() && node.harnessName.trim());
  const canActivate = Boolean(valid && persistedElement?.hasUnpublishedChanges && !locallyChanged);

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

      <fieldset className="workflow-role-choice" disabled={busy}>
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
          disabled={busy}
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
          disabled={busy}
          onChange={(event) =>
            onChange({ ...node, roleName: null, name: event.currentTarget.value })
          }
        />
      </label>

      <label className="workflow-start-choice">
        <input
          type="checkbox"
          checked={node.isStartingPoint}
          disabled={busy}
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
          {locallyChanged || persistedElement?.hasUnpublishedChanges
            ? 'Draft changes'
            : 'Activated'}
        </span>
        <div>
          <button type="button" onClick={onSave} disabled={!valid || busy || !locallyChanged}>
            {busy ? 'Saving…' : 'Save draft'}
          </button>
          {canActivate ? (
            <button
              className="workflow-primary-button"
              type="button"
              onClick={onActivate}
              disabled={busy}
            >
              {busy ? 'Activating…' : 'Activate node'}
            </button>
          ) : null}
        </div>
      </footer>
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

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
