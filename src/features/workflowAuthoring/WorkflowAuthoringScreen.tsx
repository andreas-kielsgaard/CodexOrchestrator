import { GitBranch, Plus, RefreshCw, Save } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType } from 'react';
import { DraftWorkspace } from '../../components/draftWorkspace';
import { useDraftCloseWarning } from '../../components/useDraftCloseWarning';
import type {
  WorkflowInstanceClient,
  WorkflowRecipeInstance,
} from '../../application/workflowInstances';
import type { RepoBranchWorktreeTargetSelectorProps } from '../../application/worktreeTargets';
import type { AgentSessionClient } from '../../application/agentSessions';
import type { AgentSessionProfileClient } from '../../application/agentSessionProfiles';
import type { SessionEventQueryClient } from '../../application/sessionEvents';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { IdentityManagementClient } from '../../application/identities';
import type {
  WorkflowAuthoringClient,
  OtpPackageDto,
  WorkflowRecipeDraftDto,
  WorkflowRecipeStateDto,
  WorkflowRecipeSummaryDto,
} from '../../application/workflowAuthoring';
import {
  runtimeProfileViewModel,
  type AgentIdentityOption,
  type CapabilityProfileOption,
  type RuntimeProfileViewModel,
} from '../executionConfiguration';
import { offeredOutputs, offeredActions } from './otpPresentation';
import { WorkflowConnectionEditor } from './WorkflowConnectionEditor';
import { WorkflowNodeEditor } from './WorkflowNodeEditor';
import { WorkflowCanvas } from './WorkflowCanvas';
import { RecipeInstanceCreationDialog } from './RecipeInstanceCreationDialog';
import { WorkflowInstancePanel } from './WorkflowInstancePanel';
import { errorMessage, defaultsWithinCapabilities } from './workflowAuthoringPresentation';
import type { WorkflowEditorSelection } from './workflowAuthoringTypes';
import './workflowAuthoring.css';

export interface WorkflowAuthoringScreenProps {
  readonly client: WorkflowAuthoringClient;
  readonly executionConfigurationClient: ExecutionConfigurationClient;
  readonly identityClient?: IdentityManagementClient;
  readonly workspace?: DraftWorkspace<WorkflowRecipeDraftDto>;
  readonly instanceClient?: WorkflowInstanceClient;
  readonly targetSelector?: ComponentType<RepoBranchWorktreeTargetSelectorProps>;
  readonly sessionClient?: AgentSessionClient;
  readonly profileClient?: AgentSessionProfileClient;
  readonly queryClient?: SessionEventQueryClient;
  readonly recipeId?: string | null;
  readonly instanceId?: string | null;
  readonly onOpenRecipe?: (id: string) => void;
  readonly onOpenInstance?: (id: string) => void;
}

export function WorkflowAuthoringScreen({
  client,
  executionConfigurationClient,
  identityClient,
  workspace: providedWorkspace,
  instanceClient,
  targetSelector: TargetSelector,
  sessionClient,
  profileClient,
  queryClient,
  recipeId,
  instanceId,
  onOpenRecipe,
  onOpenInstance,
}: WorkflowAuthoringScreenProps) {
  const localWorkspace = useMemo(() => new DraftWorkspace<WorkflowRecipeDraftDto>(), []);
  const workspace = providedWorkspace ?? localWorkspace;
  const selectedRef = useRef<string | null>(recipeId ?? workspace.selectedKey);
  const openTicket = useRef(0);
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
  const [runtime, setRuntime] = useState<RuntimeProfileViewModel | null>(null);
  const [profiles, setProfiles] = useState<readonly CapabilityProfileOption[]>([]);
  const [profileValues, setProfileValues] = useState<
    ReadonlyMap<string, Awaited<ReturnType<ExecutionConfigurationClient['loadCapabilityProfile']>>>
  >(new Map());
  const [identities, setIdentities] = useState<readonly AgentIdentityOption[]>([]);
  const [selection, setSelection] = useState<WorkflowEditorSelection>({
    kind: 'node',
    id: null,
  });
  const [creatingName, setCreatingName] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [packages, setPackages] = useState<readonly OtpPackageDto[]>([]);

  const loadCatalogs = useCallback(async () => {
    const [runtimeSnapshot, capabilityProfiles, identityCatalog] = await Promise.all([
      executionConfigurationClient.loadSelectedRuntimeProfile(),
      executionConfigurationClient.listCapabilityProfiles(),
      identityClient?.list() ?? Promise.resolve([]),
    ]);
    setRuntime(runtimeProfileViewModel(runtimeSnapshot));
    setProfileValues(
      new Map(capabilityProfiles.map((profile) => [profile.capabilityProfileId, profile])),
    );
    setProfiles(
      capabilityProfiles.map((profile) => ({
        id: profile.capabilityProfileId,
        label: profile.name,
        revision: profile.revision,
      })),
    );
    setIdentities(
      identityCatalog.map((identity) => ({
        id: identity.id,
        displayName: identity.displayName,
        color: identity.color,
        shape: identity.shape,
      })),
    );
  }, [executionConfigurationClient, identityClient]);

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
    void Promise.all([loadCatalogs(), loadSummaries(), client.listCapabilities()])
      .then(([, recipes, capabilities]) => {
        if (active) setPackages(capabilities);
        if (active && recipes.length) return openRecipe(selectedRef.current ?? recipes[0].recipeId);
        return undefined;
      })
      .catch((caught) => active && setError(errorMessage(caught)))
      .finally(() => active && setBusy(false));
    return () => {
      active = false;
      openTicket.current += 1;
    };
  }, [client, loadCatalogs, loadSummaries, openRecipe]);

  useEffect(() => {
    if (recipeId && recipeId !== selectedRef.current) void openRecipe(recipeId);
  }, [recipeId, openRecipe]);
  const loadInstances = useCallback(async () => {
    if (instanceClient) setInstances(await instanceClient.list());
  }, [instanceClient]);
  useEffect(() => {
    void loadInstances().catch((cause) => setError(errorMessage(cause)));
  }, [loadInstances]);

  const createRecipe = async () => {
    if (!creatingName.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const created = await client.createRecipe(creatingName.trim());
      setCreatingName('');
      setState(created);
      selectedRef.current = created.draft.recipeId;
      workspace.selectedKey = created.draft.recipeId;
      setDraft(workspace.load(created.draft.recipeId, created.draft));
      setLocalInstanceId(null);
      onOpenRecipe?.(created.draft.recipeId);
      setSelection({ kind: 'node', id: null });
      await loadSummaries();
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
      <aside className="workflow-authoring-screen__recipes">
        <header>
          <div>
            <p>Session Event recipes</p>
            <h1>Workflows</h1>
          </div>
          <button type="button" aria-label="Reload workflows" onClick={() => void loadSummaries()}>
            <RefreshCw size={16} aria-hidden="true" />
          </button>
        </header>
        <div className="workflow-authoring-screen__create">
          <input
            aria-label="New Workflow name"
            value={creatingName}
            placeholder="New Workflow name"
            onChange={(event) => setCreatingName(event.currentTarget.value)}
          />
          <button
            type="button"
            disabled={!creatingName.trim() || busy}
            onClick={() => void createRecipe()}
          >
            <Plus size={15} aria-hidden="true" />
            Create
          </button>
        </div>
        <nav aria-label="Workflow recipes">
          {summaries.map((summary) => (
            <button
              type="button"
              className={draft?.recipeId === summary.recipeId ? 'is-selected' : undefined}
              key={summary.recipeId}
              onClick={() => {
                setLocalInstanceId(null);
                onOpenRecipe?.(summary.recipeId);
                void openRecipe(summary.recipeId);
              }}
            >
              <strong>{summary.name}</strong>
              <span>Draft v{summary.draftRevision}</span>
              <small>
                {summary.activeRevision ? `Active v${summary.activeRevision}` : 'Not active'}
              </small>
            </button>
          ))}
        </nav>
        {instanceClient ? (
          <section className="workflow-instance-list">
            <h2>Instances</h2>
            <button
              type="button"
              disabled={
                !TargetSelector || !summaries.some((recipe) => recipe.activeRevision !== null)
              }
              onClick={() => setCreatingInstance(true)}
            >
              Create instance
            </button>
            {instances.map((instance) => (
              <button
                type="button"
                key={instance.id}
                aria-pressed={selectedInstanceId === instance.id}
                onClick={() => {
                  setLocalInstanceId(instance.id);
                  onOpenInstance?.(instance.id);
                }}
              >
                {instance.name}
                <small>
                  {instance.recipe.name} · v{instance.recipe.revision}
                </small>
              </button>
            ))}
          </section>
        ) : null}
      </aside>

      <section className="workflow-authoring-screen__main">
        {error ? (
          <div className="workflow-authoring-screen__error" role="alert">
            {error}
          </div>
        ) : null}
        {selectedInstanceId && instanceClient ? (
          <WorkflowInstancePanel
            key={selectedInstanceId}
            instanceId={selectedInstanceId}
            client={instanceClient}
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
                  starting: draft.startingNodeId === node.nodeId,
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
                    startingNodeId: draft.startingNodeId ?? nodeId,
                    nodes: [...draft.nodes, node],
                  });
                  setSelection({ kind: 'node', id: nodeId });
                }}
                onConnect={(sourceNodeId, destinationNodeId) => {
                  const output = offeredOutputs(packages)[0];
                  const action = offeredActions(packages)[0];
                  if (!output || !action) {
                    setError('Import a package with a trigger output and destination action.');
                    return;
                  }
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
                        trigger: output.ref,
                        promptInputs: Object.keys(output.output.schema.properties ?? {})
                          .slice(0, 1)
                          .map((field) => ({ kind: 'output_field' as const, field })),
                        promptText: '',
                        action: action.ref,
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
                onStartingNode={(id) => editDraft({ ...draft, startingNodeId: id })}
                onRemove={() => {
                  const nodes = draft.nodes.filter(
                    (node) => selection.kind !== 'node' || node.nodeId !== selection.id,
                  );
                  editDraft({
                    ...draft,
                    nodes,
                    startingNodeId: nodes.some((node) => node.nodeId === draft.startingNodeId)
                      ? draft.startingNodeId
                      : (nodes[0]?.nodeId ?? null),
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
                      packages={packages}
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
