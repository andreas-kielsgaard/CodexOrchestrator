import { useEffect, useMemo, useRef, useState } from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
import { AgentSessionWorkspace, useAgentSession } from '../agentSessions';
import type {
  ProductDecisionClient,
  ProductDecisionCorrectionClient,
  ProductDecisionCurrent,
  ProductDecisionEvidenceDestination,
  ProductDecisionPublishTarget,
  ProductDecisionVersion,
} from '../../application/productDecisions';
import { productDecisionCommandErrorCode } from '../../application/productDecisions';

export interface ProductiveProductDecisionsPanelProps {
  readonly epicId: string;
  readonly client: ProductDecisionClient;
  readonly correctionClient?: ProductDecisionCorrectionClient;
  readonly agentSessionClient?: AgentSessionClient;
  readonly onOpenEvidence?: (destination: ProductDecisionEvidenceDestination) => void;
  readonly onPublish?: (target: ProductDecisionPublishTarget) => void;
}

type ProductiveLoad =
  | { readonly kind: 'loading' }
  | { readonly kind: 'available'; readonly decisions: readonly ProductDecisionCurrent[] }
  | { readonly kind: 'unavailable'; readonly reason: string };

type EditState = {
  readonly operationToken: number;
  readonly decisionId: string;
  readonly expectedCurrentVersion: number;
  readonly idempotencyKey: string;
  readonly humanInteractionId: string;
  title: string;
  statement: string;
  intent: string;
};

type HistoryRequest = {
  readonly token: number;
  readonly epoch: number;
  readonly epicId: string;
  readonly client: ProductDecisionClient;
};

export function ProductiveProductDecisionsPanel({
  epicId,
  client,
  correctionClient,
  agentSessionClient,
  onOpenEvidence,
  onPublish,
}: ProductiveProductDecisionsPanelProps) {
  const [load, setLoad] = useState<ProductiveLoad>({ kind: 'loading' });
  const [edit, setEdit] = useState<EditState | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [historyByDecision, setHistoryByDecision] = useState<
    Readonly<Record<string, readonly ProductDecisionVersion[]>>
  >({});
  const [historyLoading, setHistoryLoading] = useState<ReadonlySet<string>>(() => new Set());

  const mountedRef = useRef(false);
  const clientRef = useRef(client);
  const contextEpochRef = useRef(0);
  const requestTokenRef = useRef(0);
  const operationTokenRef = useRef(0);
  const loadRef = useRef<ProductiveLoad>({ kind: 'loading' });
  const editRef = useRef<EditState | null>(null);
  const savingRef = useRef(false);
  const historyByDecisionRef = useRef<Readonly<Record<string, readonly ProductDecisionVersion[]>>>(
    {},
  );
  const historyLoadingRef = useRef<ReadonlySet<string>>(new Set());
  const historyRequestsRef = useRef(new Map<string, HistoryRequest>());

  const setCurrentLoad = (next: ProductiveLoad) => {
    loadRef.current = next;
    setLoad(next);
  };

  const setCurrentEdit = (next: EditState | null) => {
    editRef.current = next;
    setEdit(next);
  };

  const setCurrentSaving = (next: boolean) => {
    savingRef.current = next;
    setSaving(next);
  };

  const setCurrentHistory = (next: Readonly<Record<string, readonly ProductDecisionVersion[]>>) => {
    historyByDecisionRef.current = next;
    setHistoryByDecision(next);
  };

  const addHistoryLoading = (decisionId: string) => {
    const next = new Set(historyLoadingRef.current);
    next.add(decisionId);
    historyLoadingRef.current = next;
    setHistoryLoading(next);
  };

  const removeHistoryLoading = (decisionId: string) => {
    const next = without(historyLoadingRef.current, decisionId);
    historyLoadingRef.current = next;
    setHistoryLoading(next);
  };

  const startCurrentLoad = (allowDuringAcceptance: boolean) => {
    if (!allowDuringAcceptance && savingRef.current) return;
    const epoch = ++contextEpochRef.current;
    const requestToken = ++requestTokenRef.current;
    const requestClient = client;
    const requestEpicId = epicId;
    setCurrentSaving(false);
    setCurrentEdit(null);
    setMessage(null);
    historyRequestsRef.current.clear();
    historyLoadingRef.current = new Set();
    setHistoryLoading(new Set());
    setCurrentHistory({});
    setCurrentLoad({ kind: 'loading' });
    void requestClient.loadCurrent(requestEpicId).then(
      (decisions) => {
        if (
          !mountedRef.current ||
          contextEpochRef.current !== epoch ||
          requestTokenRef.current !== requestToken ||
          client !== requestClient ||
          epicId !== requestEpicId
        ) {
          return;
        }
        setCurrentLoad({ kind: 'available', decisions });
      },
      () => {
        if (
          !mountedRef.current ||
          contextEpochRef.current !== epoch ||
          requestTokenRef.current !== requestToken ||
          client !== requestClient ||
          epicId !== requestEpicId
        ) {
          return;
        }
        setCurrentLoad({
          kind: 'unavailable',
          reason: 'Productive decisions could not be loaded.',
        });
      },
    );
  };

  const reload = () => startCurrentLoad(false);

  const isCurrentHistoryRequest = (current: HistoryRequest | undefined, expected: HistoryRequest) =>
    mountedRef.current &&
    current === expected &&
    contextEpochRef.current === expected.epoch &&
    client === expected.client &&
    epicId === expected.epicId;

  useEffect(() => {
    mountedRef.current = true;
    clientRef.current = client;
    startCurrentLoad(true);
    const historyRequests = historyRequestsRef.current;
    return () => {
      mountedRef.current = false;
      contextEpochRef.current += 1;
      requestTokenRef.current += 1;
      historyRequests.clear();
    };
    // The client is an injected boundary and must not be recreated by this view.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, epicId]);

  const beginEdit = (decision: ProductDecisionCurrent) => {
    if (savingRef.current) return;
    const version = decision.currentVersion;
    setMessage(null);
    setCurrentEdit({
      operationToken: ++operationTokenRef.current,
      decisionId: decision.decisionId,
      expectedCurrentVersion: version.version,
      idempotencyKey: `product-decision-accept:${decision.decisionId}:${opaqueId()}`,
      humanInteractionId: opaqueId(),
      title: version.title,
      statement: version.statement,
      intent: version.intent,
    });
  };

  const acceptEdit = async () => {
    const activeEdit = editRef.current;
    const activeLoad = loadRef.current;
    if (!activeEdit || activeLoad.kind !== 'available' || savingRef.current) return;
    const decision = activeLoad.decisions.find(
      ({ decisionId }) => decisionId === activeEdit.decisionId,
    );
    if (!decision) return;
    const epoch = contextEpochRef.current;
    const requestClient = client;
    const requestEpicId = epicId;
    const displayedVersion = decision.currentVersion.version;
    const displayedVersionId = decision.currentVersion.versionId;
    const operationToken = activeEdit.operationToken;
    const isCurrentOperation = () =>
      mountedRef.current &&
      contextEpochRef.current === epoch &&
      requestClient === client &&
      requestEpicId === epicId &&
      editRef.current?.operationToken === operationToken &&
      loadRef.current.kind === 'available' &&
      loadRef.current.decisions.some(
        (item) =>
          item.decisionId === decision.decisionId &&
          item.currentVersion.version === displayedVersion &&
          item.currentVersion.versionId === displayedVersionId,
      );
    historyRequestsRef.current.clear();
    historyLoadingRef.current = new Set();
    setHistoryLoading(new Set());
    setCurrentSaving(true);
    setMessage(null);
    try {
      const accepted = await requestClient.acceptVersion({
        decisionId: decision.decisionId,
        epicId: requestEpicId,
        expectedCurrentVersion: activeEdit.expectedCurrentVersion,
        idempotencyKey: activeEdit.idempotencyKey,
        title: activeEdit.title.trim(),
        statement: activeEdit.statement.trim(),
        intent: activeEdit.intent.trim(),
        acceptanceProvenance: {
          kind: 'manual_human_application',
          humanInteractionOrigin: {
            kind: 'human_interaction',
            opaqueId: activeEdit.humanInteractionId,
          },
        },
        currentActionableEvidence: decision.currentVersion.currentActionableEvidence,
        historicalUnresolvedEvidence: decision.currentVersion.historicalUnresolvedEvidence,
      });
      if (!isCurrentOperation()) return;
      const currentLoad = loadRef.current;
      if (currentLoad.kind !== 'available') return;
      const next: ProductDecisionCurrent = {
        ...decision,
        currentVersion: accepted,
      };
      setCurrentLoad({
        kind: 'available',
        decisions: currentLoad.decisions.map((item) =>
          item.decisionId === decision.decisionId ? next : item,
        ),
      });
      setCurrentHistory({
        ...historyByDecisionRef.current,
        [decision.decisionId]: appendVersion(
          historyByDecisionRef.current[decision.decisionId] ?? [decision.currentVersion],
          accepted,
        ),
      });
      setCurrentSaving(false);
      setCurrentEdit(null);
      setMessage(`Accepted Product Decision version ${accepted.version}.`);
    } catch (error) {
      if (!isCurrentOperation()) return;
      const code = productDecisionCommandErrorCode(error);
      setMessage(
        code === 'revision_conflict' || code === 'idempotency_conflict'
          ? 'This decision changed or conflicted elsewhere. Your edits are preserved; reload to review the current version before trying again.'
          : 'The correction was not accepted. Your edits are preserved for review.',
      );
      setCurrentSaving(false);
    }
  };

  const loadHistory = (decisionId: string) => {
    if (
      savingRef.current ||
      historyByDecisionRef.current[decisionId] ||
      historyLoadingRef.current.has(decisionId)
    ) {
      return;
    }
    const request = {
      token: ++requestTokenRef.current,
      epoch: contextEpochRef.current,
      epicId,
      client,
    } satisfies HistoryRequest;
    historyRequestsRef.current.set(decisionId, request);
    addHistoryLoading(decisionId);
    void request.client.loadHistory(request.epicId, decisionId).then(
      (history) => {
        if (!isCurrentHistoryRequest(historyRequestsRef.current.get(decisionId), request)) return;
        historyRequestsRef.current.delete(decisionId);
        setCurrentHistory({ ...historyByDecisionRef.current, [decisionId]: history });
        removeHistoryLoading(decisionId);
      },
      () => {
        if (!isCurrentHistoryRequest(historyRequestsRef.current.get(decisionId), request)) return;
        historyRequestsRef.current.delete(decisionId);
        setMessage('Version history could not be loaded.');
        removeHistoryLoading(decisionId);
      },
    );
  };

  return (
    <section className="product-decisions__productive" aria-label="Productive Product Decisions">
      <header className="product-decisions__productive-header">
        <div>
          <p className="eyebrow">Productive durable decisions</p>
          <h3>Current / official</h3>
          <p>
            These versions are stored for this Epic. <strong>Not applied</strong> means no publish
            or application effect has occurred.
          </p>
        </div>
        <button type="button" onClick={reload} disabled={saving || load.kind === 'loading'}>
          Reload
        </button>
      </header>
      {message && (
        <p className="product-decisions__command-message" role="status">
          {message}
        </p>
      )}
      {load.kind === 'loading' && <p role="status">Loading productive decisions.</p>}
      {load.kind === 'unavailable' && <p role="alert">{load.reason}</p>}
      {load.kind === 'available' && load.decisions.length === 0 && (
        <p>No productive decisions have been accepted for this Epic.</p>
      )}
      {load.kind === 'available' && load.decisions.length > 0 && (
        <ul className="product-decisions__productive-list">
          {load.decisions.map((decision) => (
            <ProductiveDecisionCard
              key={`${decision.epicId}:${decision.decisionId}:${decision.currentVersion.versionId}`}
              decision={decision}
              edit={edit?.decisionId === decision.decisionId ? edit : null}
              saving={saving}
              history={historyByDecision[decision.decisionId]}
              historyLoading={historyLoading.has(decision.decisionId)}
              onBeginEdit={() => beginEdit(decision)}
              onCancel={() => {
                if (savingRef.current) return;
                setCurrentEdit(null);
                setMessage(null);
              }}
              onChange={(field, value) =>
                !savingRef.current &&
                setCurrentEdit(editRef.current ? { ...editRef.current, [field]: value } : null)
              }
              onAccept={() => void acceptEdit()}
              onLoadHistory={() => loadHistory(decision.decisionId)}
              onOpenEvidence={onOpenEvidence}
              onPublish={onPublish}
              correctionClient={correctionClient}
              agentSessionClient={agentSessionClient}
              onAgentBusyChange={setCurrentSaving}
              contextEpoch={contextEpochRef.current}
              onAgentAccepted={(accepted, operationEpoch) => {
                const currentLoad = loadRef.current;
                if (
                  currentLoad.kind !== 'available' ||
                  clientRef.current !== client ||
                  contextEpochRef.current !== operationEpoch
                )
                  return;
                if (
                  !currentLoad.decisions.some(
                    (item) =>
                      item.epicId === decision.epicId &&
                      item.decisionId === decision.decisionId &&
                      item.currentVersion.version === decision.currentVersion.version &&
                      item.currentVersion.versionId === decision.currentVersion.versionId,
                  )
                ) {
                  return;
                }
                setCurrentLoad({
                  kind: 'available',
                  decisions: currentLoad.decisions.map((item) =>
                    item.epicId === decision.epicId && item.decisionId === decision.decisionId
                      ? { ...item, currentVersion: accepted }
                      : item,
                  ),
                });
                setMessage(`Accepted Product Decision version ${accepted.version}.`);
              }}
            />
          ))}
        </ul>
      )}
    </section>
  );
}

function ProductiveDecisionCard({
  decision,
  edit,
  saving,
  history,
  historyLoading,
  onBeginEdit,
  onCancel,
  onChange,
  onAccept,
  onLoadHistory,
  onOpenEvidence,
  onPublish,
  correctionClient,
  agentSessionClient,
  onAgentAccepted,
  onAgentBusyChange,
  contextEpoch,
}: {
  readonly decision: ProductDecisionCurrent;
  readonly edit: EditState | null;
  readonly saving: boolean;
  readonly history?: readonly ProductDecisionVersion[];
  readonly historyLoading: boolean;
  readonly onBeginEdit: () => void;
  readonly onCancel: () => void;
  readonly onChange: (field: 'title' | 'statement' | 'intent', value: string) => void;
  readonly onAccept: () => void;
  readonly onLoadHistory: () => void;
  readonly onOpenEvidence?: (destination: ProductDecisionEvidenceDestination) => void;
  readonly onPublish?: (target: ProductDecisionPublishTarget) => void;
  readonly correctionClient?: ProductDecisionCorrectionClient;
  readonly agentSessionClient?: AgentSessionClient;
  readonly onAgentAccepted: (version: ProductDecisionVersion, operationEpoch: number) => void;
  readonly onAgentBusyChange: (busy: boolean) => void;
  readonly contextEpoch: number;
}) {
  const version = decision.currentVersion;
  return (
    <li>
      <article
        className="product-decisions__productive-card"
        aria-label={`${version.title} current decision`}
      >
        <div className="product-decisions__productive-status">
          <span>Version {version.version}</span>
          <span>Not applied</span>
        </div>
        {edit ? (
          <form
            onSubmit={(event) => {
              event.preventDefault();
              onAccept();
            }}
          >
            <label>
              Title
              <input
                value={edit.title}
                onChange={(event) => onChange('title', event.target.value)}
                disabled={saving}
                required
              />
            </label>
            <label>
              Statement
              <textarea
                value={edit.statement}
                onChange={(event) => onChange('statement', event.target.value)}
                disabled={saving}
                required
              />
            </label>
            <label>
              Intent
              <textarea
                value={edit.intent}
                onChange={(event) => onChange('intent', event.target.value)}
                disabled={saving}
                required
              />
            </label>
            <div className="product-decisions__form-actions">
              <button type="submit" disabled={saving}>
                {saving ? 'Accepting…' : 'Accept correction'}
              </button>
              <button type="button" onClick={onCancel} disabled={saving}>
                Cancel
              </button>
            </div>
            <p className="product-decisions__tentative-note">
              Tentative only. No version is saved until Accept correction succeeds.
            </p>
          </form>
        ) : (
          <>
            <h4>{version.title}</h4>
            <p className="product-decisions__statement">{version.statement}</p>
            <p>
              <strong>Intent:</strong> {version.intent}
            </p>
            <div className="product-decisions__card-actions">
              <button type="button" onClick={onBeginEdit} disabled={saving}>
                Edit
              </button>
              {correctionClient && agentSessionClient && (
                <AgentAssistedCorrection
                  decision={decision}
                  client={correctionClient}
                  agentSessionClient={agentSessionClient}
                  disabled={saving}
                  onAccepted={onAgentAccepted}
                  onBusyChange={onAgentBusyChange}
                  contextEpoch={contextEpoch}
                />
              )}
              <button
                type="button"
                onClick={onLoadHistory}
                disabled={historyLoading || saving}
                aria-expanded={history !== undefined}
              >
                {historyLoading ? 'Loading history…' : 'Version history'}
              </button>
              {onPublish && (
                <button
                  type="button"
                  disabled={saving}
                  onClick={() =>
                    onPublish({
                      epicId: decision.epicId,
                      decisionId: decision.decisionId,
                      versionId: version.versionId,
                      version: version.version,
                    })
                  }
                >
                  Publish
                </button>
              )}
            </div>
          </>
        )}
        <ProductiveEvidence
          current={version.currentActionableEvidence}
          historical={version.historicalUnresolvedEvidence}
          busy={saving}
          onOpenEvidence={onOpenEvidence}
        />
        {history && <VersionHistory history={history} />}
      </article>
    </li>
  );
}

function AgentAssistedCorrection({
  decision,
  client,
  agentSessionClient,
  disabled,
  onAccepted,
  onBusyChange,
  contextEpoch,
}: {
  readonly decision: ProductDecisionCurrent;
  readonly client: ProductDecisionCorrectionClient;
  readonly agentSessionClient: AgentSessionClient;
  readonly disabled: boolean;
  readonly onAccepted: (version: ProductDecisionVersion, operationEpoch: number) => void;
  readonly onBusyChange: (busy: boolean) => void;
  readonly contextEpoch: number;
}) {
  const boundary = {
    contextEpoch,
    client,
    epicId: decision.epicId,
    decisionId: decision.decisionId,
    version: decision.currentVersion.version,
    versionId: decision.currentVersion.versionId,
  };
  const activeBoundaryRef = useRef(boundary);
  activeBoundaryRef.current = boundary;
  const [conversation, setConversation] = useState<Awaited<
    ReturnType<typeof client.startConversation>
  > | null>(null);
  const [opening, setOpening] = useState(false);
  const [proposal, setProposal] = useState<Awaited<ReturnType<typeof client.saveProposal>> | null>(
    null,
  );
  const [draft, setDraft] = useState({
    title: decision.currentVersion.title,
    statement: decision.currentVersion.statement,
    intent: decision.currentVersion.intent,
  });
  const [message, setMessage] = useState<string | null>(null);
  const [accepting, setAccepting] = useState(false);
  const isCurrentBoundary = (expected: typeof boundary) => {
    const current = activeBoundaryRef.current;
    return (
      current.contextEpoch === expected.contextEpoch &&
      current.client === expected.client &&
      current.epicId === expected.epicId &&
      current.decisionId === expected.decisionId &&
      current.version === expected.version &&
      current.versionId === expected.versionId
    );
  };

  useEffect(() => {
    setConversation(null);
    setProposal(null);
    setDraft({
      title: decision.currentVersion.title,
      statement: decision.currentVersion.statement,
      intent: decision.currentVersion.intent,
    });
    setMessage(null);
    setOpening(false);
    setAccepting(false);
  }, [
    contextEpoch,
    decision.currentVersion.intent,
    decision.currentVersion.statement,
    decision.currentVersion.title,
    decision.currentVersion.version,
    decision.currentVersion.versionId,
  ]);

  const open = async () => {
    if (opening || disabled) return;
    const operationBoundary = boundary;
    setOpening(true);
    onBusyChange(true);
    setMessage(null);
    try {
      const opened = await client.startConversation({
        epicId: decision.epicId,
        decisionId: decision.decisionId,
        baseVersion: decision.currentVersion.version,
      });
      if (!isCurrentBoundary(operationBoundary)) return;
      setConversation(opened);
      setProposal(opened.latestProposal ?? null);
    } catch (error) {
      if (!isCurrentBoundary(operationBoundary)) return;
      setMessage(
        productDecisionCommandErrorCode(error) === 'revision_conflict'
          ? 'This decision changed. Reload before starting a new correction discussion.'
          : 'The decision-bound correction conversation could not be opened.',
      );
    } finally {
      if (!isCurrentBoundary(operationBoundary)) return;
      setOpening(false);
      onBusyChange(false);
    }
  };
  if (!conversation) {
    return (
      <span>
        <button type="button" onClick={() => void open()} disabled={disabled || opening}>
          {opening ? 'Opening correction discussion…' : 'Discuss correction with agent'}
        </button>
        {message && <small role="alert">{message}</small>}
      </span>
    );
  }
  return (
    <section className="product-decisions__agent-correction" aria-label="Agent-assisted correction">
      <p className="product-decisions__tentative-note">
        This discussion is bound to the displayed decision and version. Agent material is
        proposal-only.
      </p>
      <CorrectionConversation
        conversation={conversation}
        correctionClient={client}
        agentSessionClient={agentSessionClient}
        draft={draft}
        onDraftChange={setDraft}
        onSavedProposal={setProposal}
        disabled={disabled || accepting}
        onMessage={setMessage}
        onBusyChange={onBusyChange}
        boundary={boundary}
        isCurrentBoundary={isCurrentBoundary}
      />
      {proposal && (
        <section aria-label="Proposed corrected decision">
          <h5>Proposed corrected decision</h5>
          <p>
            <strong>{proposal.title}</strong>
          </p>
          <p>{proposal.statement}</p>
          <p>
            <strong>Intent:</strong> {proposal.intent}
          </p>
          <button
            type="button"
            disabled={disabled || accepting}
            onClick={() =>
              void (async () => {
                const operationBoundary = boundary;
                setAccepting(true);
                onBusyChange(true);
                setMessage(null);
                try {
                  const accepted = await client.acceptProposal({
                    proposalId: proposal.proposalId,
                  });
                  if (!isCurrentBoundary(operationBoundary)) return;
                  onAccepted(accepted, operationBoundary.contextEpoch);
                  setMessage(`Accepted Product Decision version ${accepted.version}.`);
                } catch (error) {
                  if (!isCurrentBoundary(operationBoundary)) return;
                  setMessage(
                    productDecisionCommandErrorCode(error) === 'revision_conflict'
                      ? 'The current decision changed. This proposal remains available for review; reload and start a correction from the newer version.'
                      : 'The proposal was not accepted. It remains available for explicit review.',
                  );
                } finally {
                  if (!isCurrentBoundary(operationBoundary)) return;
                  setAccepting(false);
                  onBusyChange(false);
                }
              })()
            }
          >
            {accepting ? 'Accepting…' : 'Accept proposed correction'}
          </button>
        </section>
      )}
      {message && <p role="status">{message}</p>}
    </section>
  );
}

function CorrectionConversation({
  conversation,
  correctionClient,
  agentSessionClient,
  draft,
  onDraftChange,
  onSavedProposal,
  disabled,
  onMessage,
  onBusyChange,
  boundary,
  isCurrentBoundary,
}: {
  readonly conversation: Awaited<ReturnType<ProductDecisionCorrectionClient['startConversation']>>;
  readonly correctionClient: ProductDecisionCorrectionClient;
  readonly agentSessionClient: AgentSessionClient;
  readonly draft: { title: string; statement: string; intent: string };
  readonly onDraftChange: (value: { title: string; statement: string; intent: string }) => void;
  readonly onSavedProposal: (
    proposal: Awaited<ReturnType<ProductDecisionCorrectionClient['saveProposal']>>,
  ) => void;
  readonly disabled: boolean;
  readonly onMessage: (message: string | null) => void;
  readonly onBusyChange: (busy: boolean) => void;
  readonly boundary: {
    readonly contextEpoch: number;
    readonly client: ProductDecisionCorrectionClient;
    readonly epicId: string;
    readonly decisionId: string;
    readonly version: number;
    readonly versionId: string;
  };
  readonly isCurrentBoundary: (expected: {
    readonly contextEpoch: number;
    readonly client: ProductDecisionCorrectionClient;
    readonly epicId: string;
    readonly decisionId: string;
    readonly version: number;
    readonly versionId: string;
  }) => boolean;
}) {
  const boundClient = useMemo<AgentSessionClient>(
    () => ({
      ...agentSessionClient,
      sendMessage: ({ submittedText }) =>
        correctionClient.sendMessage({
          correctionId: conversation.correctionId,
          submittedText,
        }),
    }),
    [agentSessionClient, conversation.correctionId, correctionClient],
  );
  const session = useAgentSession(boundClient, { selectedSessionId: conversation.sessionId });
  const latestResponse = [...(session.transcript?.invocations ?? [])]
    .reverse()
    .find((invocation) => invocation.finalResponse)?.finalResponse;
  const save = async () => {
    if (!latestResponse || disabled) {
      onMessage('Wait for an exact final Agent Session response before saving a proposal.');
      return;
    }
    const operationBoundary = boundary;
    const isCurrentSave = () => isCurrentBoundary(operationBoundary);
    onBusyChange(true);
    try {
      const saved = await correctionClient.saveProposal({
        correctionId: conversation.correctionId,
        ...draft,
        proposalPassage: {
          kind: 'agent_session_passage',
          sessionId: latestResponse.anchor.sessionId,
          invocationId: latestResponse.anchor.invocationId,
          passage: { kind: 'final_response', runtimeEventId: latestResponse.eventId },
        },
      });
      if (!isCurrentSave()) return;
      onSavedProposal(saved);
      onMessage(null);
    } catch {
      if (!isCurrentSave()) return;
      onMessage('The proposal could not be retained. No Product Decision version was created.');
    } finally {
      if (isCurrentSave()) onBusyChange(false);
    }
  };
  return (
    <>
      <AgentSessionWorkspace
        controller={session}
        presentation={{
          showHeader: false,
          ariaLabel: 'Product Decision correction conversation',
          emptyState: {
            heading: 'Correction discussion',
            guidance: 'Discuss the displayed decision only.',
          },
        }}
      />
      <fieldset disabled={disabled}>
        <legend>Review proposed correction</legend>
        <label>
          Title
          <input
            value={draft.title}
            onChange={(event) => onDraftChange({ ...draft, title: event.target.value })}
          />
        </label>
        <label>
          Statement
          <textarea
            value={draft.statement}
            onChange={(event) => onDraftChange({ ...draft, statement: event.target.value })}
          />
        </label>
        <label>
          Intent
          <textarea
            value={draft.intent}
            onChange={(event) => onDraftChange({ ...draft, intent: event.target.value })}
          />
        </label>
        <button type="button" onClick={() => void save()} disabled={!latestResponse}>
          Save proposal for explicit review
        </button>
      </fieldset>
    </>
  );
}

function ProductiveEvidence({
  current,
  historical,
  busy,
  onOpenEvidence,
}: {
  readonly current: ProductDecisionCurrent['currentVersion']['currentActionableEvidence'];
  readonly historical: ProductDecisionCurrent['currentVersion']['historicalUnresolvedEvidence'];
  readonly busy: boolean;
  readonly onOpenEvidence?: (destination: ProductDecisionEvidenceDestination) => void;
}) {
  return (
    <section className="product-decisions__productive-evidence" aria-label="Decision evidence">
      <h5>Evidence retained with this version</h5>
      <h6>Current actionable evidence</h6>
      <ul>
        {current.length ? (
          current.map((item) => (
            <li key={item.evidenceId}>
              <strong>{item.evidenceId}</strong>
              {onOpenEvidence ? (
                <button
                  type="button"
                  onClick={() => onOpenEvidence(item.destination)}
                  disabled={busy}
                >
                  Open supporting Agent Session passage
                </button>
              ) : (
                <small>Exact supporting destination retained.</small>
              )}
            </li>
          ))
        ) : (
          <li>None retained.</li>
        )}
      </ul>
      <h6>Historical unresolved evidence</h6>
      <ul>
        {historical.length ? (
          historical.map((item) => (
            <li key={item.evidenceId}>
              <strong>{item.label}</strong>
              <small>
                Retained for history only; no current action or navigation is available.
              </small>
            </li>
          ))
        ) : (
          <li>None retained.</li>
        )}
      </ul>
    </section>
  );
}

function VersionHistory({ history }: { readonly history: readonly ProductDecisionVersion[] }) {
  return (
    <section className="product-decisions__history" aria-label="Immutable version history">
      <h5>Immutable version history</h5>
      <ol>
        {history.map((version) => (
          <li key={version.versionId}>
            <strong>
              Version {version.version}: {version.title}
            </strong>
            <p>{version.statement}</p>
            <small>Accepted {version.acceptedAt}. History is read-only.</small>
          </li>
        ))}
      </ol>
    </section>
  );
}

function appendVersion(
  history: readonly ProductDecisionVersion[],
  version: ProductDecisionVersion,
): readonly ProductDecisionVersion[] {
  return history.some(({ versionId }) => versionId === version.versionId)
    ? history
    : [...history, version];
}

function without(values: ReadonlySet<string>, value: string): ReadonlySet<string> {
  const next = new Set(values);
  next.delete(value);
  return next;
}

function opaqueId(): string {
  return (
    globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`
  );
}
