import { GitBranch, Save } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType } from 'react';
import { DraftWorkspace } from '../../components/draftWorkspace';
import { useDraftCloseWarning } from '../../components/useDraftCloseWarning';
import type {
  WorkflowInstanceClient,
  WorkflowRecipeInstance,
} from '../../application/workflowInstances';
import type { RepoBranchWorktreeTargetSelectorProps } from '../../application/worktreeTargets';
import type { AgentSessionClient } from '../../application/agentSessions';
import type { AgentSessionProfileClient } from '../../application/agentSessions';
import type { SessionEventQueryClient } from '../../application/sessionEvents';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { IdentityManagementClient } from '../../application/identities';
import type { OtpCatalogueReader } from '../../application/otp';
import type {
  WorkflowAuthoringClient,
  WorkflowRecipeDraftDto,
  WorkflowRecipeStateDto,
  WorkflowRecipeSummaryDto,
} from '../../application/workflowAuthoring';
import { useExecutionConfigurationCatalog } from '../executionConfiguration';
import { WorkflowConnectionEditor } from './WorkflowConnectionEditor';
import { WorkflowNodeEditor } from './WorkflowNodeEditor';
import { WorkflowCanvas } from './WorkflowCanvas';
import { WorkflowNavigation, type WorkflowNavigationTab } from './WorkflowNavigation';
import { WorkflowConnectionGroupDialog } from '../workflowGraph';
import { RecipeInstanceCreationDialog } from './RecipeInstanceCreationDialog';
import { WorkflowInstanceView } from '../workflowInstances';
import { errorMessage, defaultsWithinCapabilities } from './workflowAuthoringPresentation';
import type { WorkflowEditorSelection } from './workflowAuthoringTypes';
import './workflowAuthoring.css';

export interface WorkflowAuthoringScreenProps {
  readonly client: WorkflowAuthoringClient;
  readonly executionConfigurationClient: ExecutionConfigurationClient;
  readonly identityClient?: IdentityManagementClient;
  readonly readOtpCatalogue?: OtpCatalogueReader;
  readonly workspace?: DraftWorkspace<WorkflowRecipeDraftDto>;
  readonly instanceClient?: WorkflowInstanceClient;
  readonly targetSelector?: ComponentType<RepoBranchWorktreeTargetSelectorProps>;
  readonly sessionClient?: AgentSessionClient;
  readonly profileClient?: AgentSessionProfileClient;
  readonly queryClient?: SessionEventQueryClient;
  readonly recipeId?: string | null;
  readonly sessionFocus?: { readonly nodeId: string; readonly sessionId: string };
  readonly instanceId?: string | null;
  readonly onOpenRecipe?: (id: string) => void;
  readonly onOpenInstance?: (id: string) => void;
}

export function WorkflowAuthoringScreen({
  client,
  executionConfigurationClient,
  identityClient,
  readOtpCatalogue,
  workspace: providedWorkspace,
  instanceClient,
  targetSelector: TargetSelector,
  sessionClient,
  profileClient,
  queryClient,
  recipeId,
  instanceId,
  sessionFocus,
  onOpenRecipe,
  onOpenInstance,
}: WorkflowAuthoringScreenProps) {
  const localWorkspace = useMemo(() => new DraftWorkspace<WorkflowRecipeDraftDto>(), []);
  const workspace = providedWorkspace ?? localWorkspace;
  const selectedRef = useRef<string | null>(recipeId ?? workspace.selectedKey);
  const openTicket = useRef(0);
  const nameInput = useRef<HTMLInputElement>(null);
  const [summaries, setSummaries] = useState<readonly WorkflowRecipeSummaryDto[]>([]);
  const [state, setState] = useState<WorkflowRecipeStateDto | null>(null);
  const [draft, setDraft] = useState<WorkflowRecipeDraftDto | null>(null);
  const [instances, setInstances] = useState<readonly WorkflowRecipeInstance[]>([]);
  const [creatingInstance, setCreatingInstance] = useState(false);
  const [localInstanceId, setLocalInstanceId] = useState<string | null>(null);
  const selectedInstanceId = instanceId === undefined ? localInstanceId : instanceId;
  const editDraft = (next: WorkflowRecipeDraftDto) => {
    workspace.edit(next.recipeId, next);
    setDraft(next);
  };
  useDraftCloseWarning(() => workspace.dirty());
  const {
    runtime,
    profiles,
    profileValues,
    modelCatalogues,
    identities,
    error: catalogError,
  } = useExecutionConfigurationCatalog(
    executionConfigurationClient,
    identityClient,
    readOtpCatalogue,
    Boolean(draft),
  );
  const [selection, setSelection] = useState<WorkflowEditorSelection>({
    kind: 'node',
    id: null,
  });
  const [connectionGroupIds, setConnectionGroupIds] = useState<readonly string[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadSummaries = useCallback(async () => {
    const next = await client.listRecipes();
    setSummaries(next);
    return next;
  }, [client]);

  const openRecipe = useCallback(
    async (recipeId: string) => {
      const ticket = ++openTicket.current;
      selectedRef.current = recipeId;
      workspace.selectedKey = recipeId;
      setBusy(true);
      setError(null);
      try {
        const next = await client.loadRecipe(recipeId);
        if (ticket !== openTicket.current) return;
        setState(next);
        setDraft(workspace.load(recipeId, next.draft));
        setSelection({
          kind: 'node',
          id: null,
        });
      } catch (caught) {
        if (ticket === openTicket.current) setError(errorMessage(caught));
      } finally {
        if (ticket === openTicket.current) setBusy(false);
      }
    },
    [client, workspace],
  );

  useEffect(() => {
    let active = true;
    setBusy(true);
    setError(null);
    void client
      .openWorkspace(selectedRef.current)
      .then((workspaceState) => {
        if (!active) return;
        setSummaries(workspaceState.summaries);
        if (!workspaceState.selected) return;
        const selected = workspaceState.selected;
        selectedRef.current = selected.draft.recipeId;
        workspace.selectedKey = selected.draft.recipeId;
        setState(selected);
        setDraft(workspace.load(selected.draft.recipeId, selected.draft));
        setSelection({ kind: 'node', id: null });
      })
      .catch((caught) => active && setError(errorMessage(caught)))
      .finally(() => active && setBusy(false));
    return () => {
      active = false;
      openTicket.current += 1;
    };
  }, [client, workspace]);

  useEffect(() => {
    if (recipeId && recipeId !== selectedRef.current) void openRecipe(recipeId);
  }, [recipeId, openRecipe]);
  const loadInstances = useCallback(async () => {
    if (instanceClient) setInstances(await instanceClient.list());
  }, [instanceClient]);
  const instancesLoaded = useRef(false);
  const handleNavigationTabChange = useCallback(
    (tab: WorkflowNavigationTab) => {
      if (tab !== 'instances' || instancesLoaded.current) return;
      instancesLoaded.current = true;
      void loadInstances().catch((cause) => {
        instancesLoaded.current = false;
        setError(errorMessage(cause));
      });
    },
    [loadInstances],
  );
  useEffect(() => {
    if (selectedInstanceId) handleNavigationTabChange('instances');
  }, [handleNavigationTabChange, selectedInstanceId]);

  const createRecipe = async () => {
    setBusy(true);
    setError(null);
    try {
      const created = await client.createRecipe();
      setState(created);
      selectedRef.current = created.draft.recipeId;
      workspace.selectedKey = created.draft.recipeId;
      setDraft(workspace.load(created.draft.recipeId, created.draft));
      setLocalInstanceId(null);
      onOpenRecipe?.(created.draft.recipeId);
      setSelection({ kind: 'node', id: null });
      await loadSummaries();
      requestAnimationFrame(() => nameInput.current?.focus());
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  };

  const saveDraft = async () => {
    if (!draft || busy) return;
    setBusy(true);
    setError(null);
    try {
      const saved = await client.saveDraft(draft);
      const working = workspace.acceptSave(
        draft.recipeId,
        draft,
        saved.draft,
        (current, baseline) => ({ ...current, revision: baseline.revision }),
      );
      if (selectedRef.current === draft.recipeId) {
        setState(saved);
        setDraft(working);
      }
      await loadSummaries();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  };

  const activate = async () => {
    if (!draft || busy) return;
    setBusy(true);
    setError(null);
    try {
      const activated = await client.activateRecipe(
        draft.recipeId,
        workspace.baseline(draft.recipeId)?.revision ?? draft.revision,
      );
      if (selectedRef.current === draft.recipeId) setState(activated);
      await loadSummaries();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  };

  const selectedNode =
    draft && selection.kind === 'node'
      ? (draft.nodes.find((node) => node.nodeId === selection.id) ?? null)
      : null;
  const selectedConnection =
    draft && selection.kind === 'connection'
      ? (draft.connections.find((connection) => connection.connectionId === selection.id) ?? null)
      : null;

  return (
    <main className="workflow-authoring-screen">
      <WorkflowNavigation
        summaries={summaries}
        instances={instances}
        selectedRecipeId={draft?.recipeId ?? null}
        selectedInstanceId={selectedInstanceId ?? null}
        busy={busy}
        canCreateInstance={Boolean(instanceClient && TargetSelector)}
        onTabChange={handleNavigationTabChange}
        onCreateDesign={() => void createRecipe()}
        onCreateInstance={() => setCreatingInstance(true)}
        onOpenRecipe={(id) => {
          setLocalInstanceId(null);
          onOpenRecipe?.(id);
          void openRecipe(id);
        }}
        onOpenInstance={(id) => {
          setLocalInstanceId(id);
          onOpenInstance?.(id);
        }}
      />

      <section className="workflow-authoring-screen__main">
        {error || catalogError ? (
          <div className="workflow-authoring-screen__error" role="alert">
            {error ?? catalogError}
          </div>
        ) : null}
        {selectedInstanceId && instanceClient ? (
          <WorkflowInstanceView
            key={JSON.stringify([
              selectedInstanceId,
              sessionFocus?.nodeId,
              sessionFocus?.sessionId,
            ])}
            instanceId={selectedInstanceId}
            sessionFocus={sessionFocus}
            client={instanceClient}
            capabilityProfiles={profileValues}
            identities={identities}
            sessionClient={sessionClient}
            profileClient={profileClient}
            queryClient={queryClient}
          />
        ) : !draft ? (
          <div className="workflow-authoring-screen__empty">
            <GitBranch size={28} aria-hidden="true" />
            <h2>Create a Workflow recipe</h2>
            <p>A Workflow compiles its nodes and connections into managed Session Events.</p>
          </div>
        ) : (
          <>
            <header className="workflow-authoring-screen__toolbar">
              <div>
                <p>Workflow recipe · draft revision {draft.revision}</p>
                <input
                  ref={nameInput}
                  aria-label="Workflow name"
                  value={draft.name}
                  onChange={(event) => editDraft({ ...draft, name: event.currentTarget.value })}
                />
              </div>
              <div>
                <span>{state?.active ? `Active v${state.active.revision}` : 'Not active'}</span>
                {workspace.dirty(draft.recipeId) ? (
                  <small>Unsaved edits · activation uses the saved draft</small>
                ) : null}
                <button type="button" disabled={busy} onClick={() => void saveDraft()}>
                  <Save size={15} aria-hidden="true" /> Save draft
                </button>
                <button
                  className="is-primary"
                  type="button"
                  disabled={busy}
                  onClick={() => void activate()}
                >
                  Activate saved draft
                </button>
              </div>
            </header>

            <div className="workflow-authoring-screen__body">
              <WorkflowCanvas
                key={draft.recipeId}
                nodes={draft.nodes.map((node) => ({
                  id: node.nodeId,
                  name: node.name,
                  x: node.positionX,
                  y: node.positionY,
                  entry: draft.entryNodeIds.includes(node.nodeId),
                }))}
                connections={draft.connections.map((connection) => ({
                  id: connection.connectionId,
                  name: connection.name,
                  source: connection.sourceNodeId,
                  destination: connection.destinationNodeId,
                }))}
                selection={selection}
                canAdd={profiles.length > 0}
                canUndo={workspace.canUndo(draft.recipeId)}
                canRedo={workspace.canRedo(draft.recipeId)}
                onUndo={() => {
                  const previous = workspace.restore(draft.recipeId, 'undo', (value, saved) => ({
                    ...value,
                    revision: saved.revision,
                  }));
                  if (previous) setDraft(previous);
                }}
                onRedo={() => {
                  const next = workspace.restore(draft.recipeId, 'redo', (value, saved) => ({
                    ...value,
                    revision: saved.revision,
                  }));
                  if (next) setDraft(next);
                }}
                onSelect={setSelection}
                onConnectionGroup={(ids) => {
                  if (ids.length === 1) setSelection({ kind: 'connection', id: ids[0] });
                  else setConnectionGroupIds(ids);
                }}
                onPlace={(x, y, copyFrom) => {
                  const source = draft.nodes.find((node) => node.nodeId === copyFrom);
                  const profile = source
                    ? profileValues.get(source.capabilityProfileId)
                    : profiles[0]
                      ? profileValues.get(profiles[0].id)
                      : undefined;
                  if (!source && !profile) return;
                  const nodeId = `node-${crypto.randomUUID()}`;
                  const node = source
                    ? {
                        ...structuredClone(source),
                        nodeId,
                        name: `${source.name} copy`,
                        positionX: x,
                        positionY: y,
                      }
                    : {
                        nodeId,
                        name: `Node ${draft.nodes.length + 1}`,
                        positionX: x,
                        positionY: y,
                        capabilityProfileId: profile!.capabilityProfileId,
                        nodeProfile: {
                          contractVersion: 1 as const,
                          allowedCapabilities: profile!.allowedCapabilities,
                          pinnedDefaults: defaultsWithinCapabilities(
                            profile!.allowedCapabilities,
                            runtime?.lockedSelections,
                          ),
                        },
                        initialPrompt: null,
                        agentIdentityId: null,
                      };
                  editDraft({
                    ...draft,
                    nodes: [...draft.nodes, node],
                  });
                  setSelection({ kind: 'node', id: nodeId });
                }}
                onConnect={(sourceNodeId, destinationNodeId) => {
                  const connectionId = `connection-${crypto.randomUUID()}`;
                  editDraft({
                    ...draft,
                    connections: [
                      ...draft.connections,
                      {
                        connectionId,
                        name: `${draft.nodes.find((node) => node.nodeId === sourceNodeId)?.name} → ${draft.nodes.find((node) => node.nodeId === destinationNodeId)?.name}`,
                        sourceNodeId,
                        destinationNodeId,
                        trigger: {
                          capability: {
                            package: 'workflow',
                            tool: 'on_invocation_completed',
                          },
                          output: 'completed',
                        },
                        promptInputs: [{ kind: 'output_field', field: 'output' }],
                        promptText: '',
                        action: {
                          package: 'workflow',
                          tool: 'prompt_agent',
                        },
                        configuration: {},
                      },
                    ],
                  });
                  setSelection({ kind: 'connection', id: connectionId });
                }}
                onMove={(id, x, y) =>
                  editDraft({
                    ...draft,
                    nodes: draft.nodes.map((node) =>
                      node.nodeId === id ? { ...node, positionX: x, positionY: y } : node,
                    ),
                  })
                }
                onEntryNode={(id) =>
                  editDraft({
                    ...draft,
                    entryNodeIds: draft.entryNodeIds.includes(id)
                      ? draft.entryNodeIds.filter((nodeId) => nodeId !== id)
                      : [...draft.entryNodeIds, id],
                  })
                }
                onRemove={() => {
                  const nodes = draft.nodes.filter(
                    (node) => selection.kind !== 'node' || node.nodeId !== selection.id,
                  );
                  editDraft({
                    ...draft,
                    nodes,
                    entryNodeIds: draft.entryNodeIds.filter((id) =>
                      nodes.some((node) => node.nodeId === id),
                    ),
                    connections: draft.connections.filter((edge) =>
                      selection.kind === 'connection'
                        ? edge.connectionId !== selection.id
                        : edge.sourceNodeId !== selection.id &&
                          edge.destinationNodeId !== selection.id,
                    ),
                  });
                  setSelection({ kind: 'node', id: null });
                }}
              />
              {selection.id ? (
                <section
                  className="workflow-authoring-screen__editor"
                  aria-label="Selected flow element"
                  onKeyDown={(event) => {
                    if (event.key === 'Escape') setSelection({ kind: 'node', id: null });
                  }}
                >
                  <button
                    type="button"
                    className="workflow-authoring-screen__close-editor"
                    onClick={() => setSelection({ kind: 'node', id: null })}
                  >
                    Close editor
                  </button>
                  {selectedNode && runtime ? (
                    <WorkflowNodeEditor
                      node={selectedNode}
                      draft={draft}
                      runtime={runtime}
                      profiles={profiles}
                      profileValues={profileValues}
                      modelCatalogues={modelCatalogues}
                      identities={identities}
                      onChange={(node) =>
                        editDraft({
                          ...draft,
                          nodes: draft.nodes.map((candidate) =>
                            candidate.nodeId === node.nodeId ? node : candidate,
                          ),
                        })
                      }
                      onCopy={(sourceNodeId) => {
                        const source = draft.nodes.find((node) => node.nodeId === sourceNodeId);
                        if (!source) return;
                        editDraft({
                          ...draft,
                          nodes: draft.nodes.map((candidate) =>
                            candidate.nodeId === selectedNode.nodeId
                              ? {
                                  ...candidate,
                                  capabilityProfileId: source.capabilityProfileId,
                                  nodeProfile: source.nodeProfile,
                                  initialPrompt: source.initialPrompt,
                                  agentIdentityId: source.agentIdentityId,
                                }
                              : candidate,
                          ),
                        });
                      }}
                    />
                  ) : selectedConnection ? (
                    <WorkflowConnectionEditor
                      connection={selectedConnection}
                      nodes={draft.nodes}
                      packages={runtime?.catalogs.otpPackages ?? []}
                      onChange={(connection) =>
                        editDraft({
                          ...draft,
                          connections: draft.connections.map((candidate) =>
                            candidate.connectionId === connection.connectionId
                              ? connection
                              : candidate,
                          ),
                        })
                      }
                    />
                  ) : (
                    <div className="workflow-authoring-screen__empty">
                      <h2>Select or add a node</h2>
                      <p>Node state is embedded in this recipe and can be copied before editing.</p>
                    </div>
                  )}
                </section>
              ) : null}
              {connectionGroupIds ? (
                <WorkflowConnectionGroupDialog
                  connections={draft.connections
                    .filter((connection) => connectionGroupIds.includes(connection.connectionId))
                    .map((connection) => ({
                      id: connection.connectionId,
                      name: connection.name,
                      source: connection.sourceNodeId,
                      destination: connection.destinationNodeId,
                    }))}
                  nodes={draft.nodes.map((node) => ({
                    id: node.nodeId,
                    name: node.name,
                    x: node.positionX,
                    y: node.positionY,
                  }))}
                  summary={(connection) => {
                    const source = draft.connections.find(
                      (candidate) => candidate.connectionId === connection.id,
                    );
                    return source
                      ? `${source.trigger.capability.tool} → ${source.action.tool}`
                      : null;
                  }}
                  onClose={() => setConnectionGroupIds(null)}
                  onSelect={(id) => {
                    setConnectionGroupIds(null);
                    setSelection({ kind: 'connection', id });
                  }}
                />
              ) : null}
            </div>
          </>
        )}
      </section>
      {creatingInstance && instanceClient && TargetSelector ? (
        <RecipeInstanceCreationDialog
          recipes={summaries}
          TargetSelector={TargetSelector}
          onClose={() => setCreatingInstance(false)}
          onSubmit={async (input) => {
            const instance = await instanceClient.create(input);
            setCreatingInstance(false);
            await loadInstances();
            setLocalInstanceId(instance.id);
            onOpenInstance?.(instance.id);
          }}
        />
      ) : null}
    </main>
  );
}
