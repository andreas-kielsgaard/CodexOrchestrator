import { useEffect, useState } from 'react';
import type {
  ProductDecisionClient,
  ProductDecisionCurrent,
  ProductDecisionEvidenceDestination,
  ProductDecisionPublishTarget,
  ProductDecisionVersion,
} from '../../application/productDecisions';
import { productDecisionCommandErrorCode } from '../../application/productDecisions';

export interface ProductiveProductDecisionsPanelProps {
  readonly epicId: string;
  readonly client: ProductDecisionClient;
  readonly onOpenEvidence?: (destination: ProductDecisionEvidenceDestination) => void;
  readonly onPublish?: (target: ProductDecisionPublishTarget) => void;
}

type ProductiveLoad =
  | { readonly kind: 'loading' }
  | { readonly kind: 'available'; readonly decisions: readonly ProductDecisionCurrent[] }
  | { readonly kind: 'unavailable'; readonly reason: string };

type EditState = {
  readonly decisionId: string;
  readonly expectedCurrentVersion: number;
  readonly idempotencyKey: string;
  readonly humanInteractionId: string;
  title: string;
  statement: string;
  intent: string;
};

export function ProductiveProductDecisionsPanel({
  epicId,
  client,
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

  const reload = () => {
    setEdit(null);
    setMessage(null);
    setHistoryByDecision({});
    setLoad({ kind: 'loading' });
    void client.loadCurrent(epicId).then(
      (decisions) => setLoad({ kind: 'available', decisions }),
      () => setLoad({ kind: 'unavailable', reason: 'Productive decisions could not be loaded.' }),
    );
  };

  useEffect(() => {
    reload();
    return undefined;
    // The client is an injected boundary and must not be recreated by this view.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, epicId]);

  const beginEdit = (decision: ProductDecisionCurrent) => {
    const version = decision.currentVersion;
    setMessage(null);
    setEdit({
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
    if (!edit || load.kind !== 'available') return;
    const decision = load.decisions.find(({ decisionId }) => decisionId === edit.decisionId);
    if (!decision) return;
    setSaving(true);
    setMessage(null);
    try {
      const accepted = await client.acceptVersion({
        decisionId: decision.decisionId,
        epicId,
        expectedCurrentVersion: edit.expectedCurrentVersion,
        idempotencyKey: edit.idempotencyKey,
        title: edit.title.trim(),
        statement: edit.statement.trim(),
        intent: edit.intent.trim(),
        acceptanceProvenance: {
          kind: 'manual_human_application',
          humanInteractionOrigin: {
            kind: 'human_interaction',
            opaqueId: edit.humanInteractionId,
          },
        },
        currentActionableEvidence: decision.currentVersion.currentActionableEvidence,
        historicalUnresolvedEvidence: decision.currentVersion.historicalUnresolvedEvidence,
      });
      const next: ProductDecisionCurrent = {
        ...decision,
        currentVersion: accepted,
      };
      setLoad({
        kind: 'available',
        decisions: load.decisions.map((item) =>
          item.decisionId === decision.decisionId ? next : item,
        ),
      });
      setHistoryByDecision((current) => ({
        ...current,
        [decision.decisionId]: appendVersion(
          current[decision.decisionId] ?? [decision.currentVersion],
          accepted,
        ),
      }));
      setEdit(null);
      setMessage(`Accepted Product Decision version ${accepted.version}.`);
    } catch (error) {
      const code = productDecisionCommandErrorCode(error);
      setMessage(
        code === 'revision_conflict' || code === 'idempotency_conflict'
          ? 'This decision changed or conflicted elsewhere. Your edits are preserved; reload to review the current version before trying again.'
          : 'The correction was not accepted. Your edits are preserved for review.',
      );
    } finally {
      setSaving(false);
    }
  };

  const loadHistory = (decisionId: string) => {
    if (historyByDecision[decisionId] || historyLoading.has(decisionId)) return;
    setHistoryLoading((current) => new Set(current).add(decisionId));
    void client.loadHistory(epicId, decisionId).then(
      (history) => {
        setHistoryByDecision((current) => ({ ...current, [decisionId]: history }));
        setHistoryLoading((current) => without(current, decisionId));
      },
      () => {
        setMessage('Version history could not be loaded.');
        setHistoryLoading((current) => without(current, decisionId));
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
        <button type="button" onClick={reload} disabled={load.kind === 'loading'}>
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
              key={decision.decisionId}
              decision={decision}
              edit={edit?.decisionId === decision.decisionId ? edit : null}
              saving={saving}
              history={historyByDecision[decision.decisionId]}
              historyLoading={historyLoading.has(decision.decisionId)}
              onBeginEdit={() => beginEdit(decision)}
              onCancel={() => {
                setEdit(null);
                setMessage(null);
              }}
              onChange={(field, value) =>
                setEdit((current) => (current ? { ...current, [field]: value } : current))
              }
              onAccept={() => void acceptEdit()}
              onLoadHistory={() => loadHistory(decision.decisionId)}
              onOpenEvidence={onOpenEvidence}
              onPublish={onPublish}
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
                required
              />
            </label>
            <label>
              Statement
              <textarea
                value={edit.statement}
                onChange={(event) => onChange('statement', event.target.value)}
                required
              />
            </label>
            <label>
              Intent
              <textarea
                value={edit.intent}
                onChange={(event) => onChange('intent', event.target.value)}
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
              <button type="button" onClick={onBeginEdit}>
                Edit
              </button>
              <button
                type="button"
                onClick={onLoadHistory}
                disabled={historyLoading}
                aria-expanded={history !== undefined}
              >
                {historyLoading ? 'Loading history…' : 'Version history'}
              </button>
              {onPublish && (
                <button
                  type="button"
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
          onOpenEvidence={onOpenEvidence}
        />
        {history && <VersionHistory history={history} />}
      </article>
    </li>
  );
}

function ProductiveEvidence({
  current,
  historical,
  onOpenEvidence,
}: {
  readonly current: ProductDecisionCurrent['currentVersion']['currentActionableEvidence'];
  readonly historical: ProductDecisionCurrent['currentVersion']['historicalUnresolvedEvidence'];
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
                <button type="button" onClick={() => onOpenEvidence(item.destination)}>
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
