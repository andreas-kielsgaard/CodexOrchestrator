import { GitBranch, Plus, RefreshCw, Save } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { IdentityManagementClient } from '../../application/identities';
import type {
  SessionEventDefinitionDto,
  SessionEventResultDto,
} from '../../application/sessionEvents';
import type {
  WorkflowAuthoringClient,
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
import { WorkflowConnectionEditor } from './WorkflowConnectionEditor';
import { WorkflowNodeEditor } from './WorkflowNodeEditor';
import { WorkflowOutline } from './WorkflowOutline';
import { WorkflowRunPanel } from './WorkflowRunPanel';
import { errorMessage } from './workflowAuthoringPresentation';
import type { WorkflowEditorSelection } from './workflowAuthoringTypes';
import './workflowAuthoring.css';

export interface WorkflowAuthoringScreenProps {
  readonly client: WorkflowAuthoringClient;
  readonly executionConfigurationClient: ExecutionConfigurationClient;
  readonly identityClient?: IdentityManagementClient;
}

export function WorkflowAuthoringScreen({
  client,
  executionConfigurationClient,
  identityClient,
}: WorkflowAuthoringScreenProps) {
  const [summaries, setSummaries] = useState<readonly WorkflowRecipeSummaryDto[]>([]);
  const [state, setState] = useState<WorkflowRecipeStateDto | null>(null);
  const [draft, setDraft] = useState<WorkflowRecipeDraftDto | null>(null);
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
  const [compiled, setCompiled] = useState<readonly SessionEventDefinitionDto[]>([]);
  const [eventResult, setEventResult] = useState<SessionEventResultDto | null>(null);

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
      setBusy(true);
      setError(null);
      try {
        const next = await client.loadRecipe(recipeId);
        setState(next);
        setDraft(next.draft);
        setSelection({
          kind: 'node',
          id: next.draft.startingNodeId ?? next.draft.nodes[0]?.nodeId ?? null,
        });
        setCompiled([]);
        setEventResult(null);
      } catch (caught) {
        setError(errorMessage(caught));
      } finally {
        setBusy(false);
      }
    },
    [client],
  );

  useEffect(() => {
    let active = true;
    setBusy(true);
    setError(null);
    void Promise.all([loadCatalogs(), loadSummaries()])
      .then(([, recipes]) => {
        if (active && recipes[0]) return openRecipe(recipes[0].recipeId);
        return undefined;
      })
      .catch((caught) => active && setError(errorMessage(caught)))
      .finally(() => active && setBusy(false));
    return () => {
      active = false;
    };
  }, [loadCatalogs, loadSummaries, openRecipe]);

  const createRecipe = async () => {
    if (!creatingName.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const created = await client.createRecipe(creatingName.trim());
      setCreatingName('');
      setState(created);
      setDraft(created.draft);
      setSelection({ kind: 'node', id: null });
      await loadSummaries();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  };

  const saveDraft = async () => {
    if (!draft) return;
    setBusy(true);
    setError(null);
    try {
      const saved = await client.saveDraft(draft);
      setState(saved);
      setDraft(saved.draft);
      await loadSummaries();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  };

  const activate = async () => {
    if (!draft) return;
    setBusy(true);
    setError(null);
    try {
      const activated = await client.activateRecipe(draft.recipeId);
      setState(activated);
      setDraft(activated.draft);
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
              onClick={() => void openRecipe(summary.recipeId)}
            >
              <strong>{summary.name}</strong>
              <span>Draft v{summary.draftRevision}</span>
              <small>
                {summary.activeRevision ? `Active v${summary.activeRevision}` : 'Not active'}
              </small>
            </button>
          ))}
        </nav>
      </aside>

      <section className="workflow-authoring-screen__main">
        {error ? (
          <div className="workflow-authoring-screen__error" role="alert">
            {error}
          </div>
        ) : null}
        {!draft ? (
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
                  onChange={(event) => setDraft({ ...draft, name: event.currentTarget.value })}
                />
              </div>
              <div>
                <span>{state?.active ? `Active v${state.active.revision}` : 'Not active'}</span>
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
              <WorkflowOutline
                draft={draft}
                selection={selection}
                defaultProfile={profiles[0] ? profileValues.get(profiles[0].id) : undefined}
                runtimeLockedSelections={runtime?.lockedSelections}
                onSelect={setSelection}
                onChange={setDraft}
              />
              <section className="workflow-authoring-screen__editor">
                {selectedNode && runtime ? (
                  <WorkflowNodeEditor
                    node={selectedNode}
                    draft={draft}
                    runtime={runtime}
                    profiles={profiles}
                    profileValues={profileValues}
                    identities={identities}
                    onChange={(node) =>
                      setDraft({
                        ...draft,
                        nodes: draft.nodes.map((candidate) =>
                          candidate.nodeId === node.nodeId ? node : candidate,
                        ),
                      })
                    }
                    onCopy={(sourceNodeId) => {
                      const source = draft.nodes.find((node) => node.nodeId === sourceNodeId);
                      if (!source) return;
                      setDraft({
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
                    onChange={(connection) =>
                      setDraft({
                        ...draft,
                        connections: draft.connections.map((candidate) =>
                          candidate.connectionId === connection.connectionId
                            ? connection
                            : candidate,
                        ),
                      })
                    }
                  />
                ) : selection.kind === 'run' ? (
                  <WorkflowRunPanel
                    client={client}
                    recipeId={draft.recipeId}
                    compiled={compiled}
                    eventResult={eventResult}
                    onCompiled={setCompiled}
                    onEventResult={setEventResult}
                    onError={setError}
                  />
                ) : (
                  <div className="workflow-authoring-screen__empty">
                    <h2>Select or add a node</h2>
                    <p>Node state is embedded in this recipe and can be copied before editing.</p>
                  </div>
                )}
              </section>
            </div>
          </>
        )}
      </section>
    </main>
  );
}
