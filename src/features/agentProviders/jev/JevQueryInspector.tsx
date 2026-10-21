import { useEffect, useState } from 'react';
import type {
  AppliedJevQuery,
  JevAnswer,
  JevQueryClient,
  JevQueryDetail,
  JevQuerySummary,
} from '../../../application/agentSessions/jevQueries';
import './jevQueryInspector.css';

export function StructuredValue({ value }: { value: unknown }) {
  if (value === null || value === undefined) return <span className="jev-muted">None</span>;
  if (typeof value !== 'object') return <pre className="jev-value">{String(value)}</pre>;
  const entries = Array.isArray(value)
    ? value.map((v, i) => [String(i + 1), v] as const)
    : Object.entries(value);
  return (
    <dl className="jev-tree">
      {entries.map(([key, item]) => (
        <div key={key}>
          <dt>{key}</dt>
          <dd>
            {typeof item === 'object' && item !== null ? (
              <details>
                <summary>
                  {Array.isArray(item)
                    ? `${item.length} items`
                    : `${Object.keys(item).length} fields`}
                </summary>
                <StructuredValue value={item} />
              </details>
            ) : (
              <StructuredValue value={item} />
            )}
          </dd>
        </div>
      ))}
    </dl>
  );
}
const percent = (value: number) => `${(value * 100).toFixed(1)}%`;
function answerLabel(answer?: JevAnswer) {
  return !answer
    ? 'Awaiting result'
    : answer.type === 'choice'
      ? answer.choice
      : answer.type === 'score'
        ? String(answer.score)
        : `Noul ${answer.noul.toFixed(3)}`;
}
function Answer({ answer }: { answer?: JevAnswer }) {
  if (!answer) return <p className="jev-muted">No response yet.</p>;
  return (
    <div className="jev-answer">
      <strong>{answerLabel(answer)}</strong>
      {answer.type === 'noul' ? (
        <p>Noul value: {answer.noul.toFixed(3)}</p>
      ) : (
        <>
          <p>Confidence: {percent(answer.confidence)}</p>
          <details>
            <summary>Probability distribution</summary>
            <table>
              <thead>
                <tr>
                  <th>Option</th>
                  <th>Probability</th>
                </tr>
              </thead>
              <tbody>
                {Object.entries(answer.probabilities)
                  .sort((a, b) => b[1] - a[1])
                  .map(([key, probability]) => (
                    <tr key={key}>
                      <td>{key}</td>
                      <td>
                        <meter
                          min={0}
                          max={1}
                          value={probability}
                          aria-label={`${key} probability`}
                        />{' '}
                        {percent(probability)}
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </details>
          {answer.type === 'score' && (
            <details>
              <summary>Score legend</summary>
              <StructuredValue value={answer.legend} />
            </details>
          )}
        </>
      )}
    </div>
  );
}
export function JevQueryInspector({
  client,
  sessionId,
  applications = [],
  onShowRelease,
  questionSubject,
  active = true,
}: {
  client: JevQueryClient;
  sessionId: string;
  applications?: readonly AppliedJevQuery[];
  onShowRelease?(requestId: string): void;
  questionSubject?(detail: JevQueryDetail, questionId: string): string | undefined;
  active?: boolean;
}) {
  const [queries, setQueries] = useState<JevQuerySummary[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [detail, setDetail] = useState<JevQueryDetail | null>(null);
  const [previous, setPrevious] = useState<JevQueryDetail | null>(null);
  const [compare, setCompare] = useState(false);
  const [error, setError] = useState('');
  const [refresh, setRefresh] = useState(0);
  const selectedQuery = queries.find((q) => q.invocationId === selected);
  const previousId =
    queries[queries.findIndex((q) => q.invocationId === selected) + 1]?.invocationId;
  useEffect(() => {
    setSelected(null);
    setDetail(null);
    setQueries([]);
    setCompare(false);
  }, [sessionId]);
  useEffect(() => {
    if (!active) return;
    let alive = true;
    let pending = false;
    const read = async () => {
      if (pending) return;
      pending = true;
      try {
        const result = await client.list(sessionId);
        if (alive) {
          setQueries(result);
          setSelected((current) => current ?? result[0]?.invocationId ?? null);
          setError('');
        }
      } catch (error) {
        if (alive) setError(String(error));
      } finally {
        pending = false;
      }
    };
    void read();
    const timer = setInterval(() => void read(), 2000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [client, sessionId, refresh, active]);
  useEffect(() => {
    let alive = true;
    setDetail(null);
    setPrevious(null);
    if (selected)
      void client
        .load(sessionId, selected)
        .then((result) => {
          if (alive) setDetail(result);
        })
        .catch((error) => {
          if (alive) setError(String(error));
        });
    return () => {
      alive = false;
    };
  }, [client, sessionId, selected, selectedQuery?.status, selectedQuery?.completedAt, refresh]);
  useEffect(() => {
    let alive = true;
    setPrevious(null);
    if (compare && previousId)
      void client
        .load(sessionId, previousId)
        .then((result) => {
          if (alive) setPrevious(result);
        })
        .catch((error) => {
          if (alive) setError(String(error));
        });
    return () => {
      alive = false;
    };
  }, [client, sessionId, compare, previousId]);
  const application = applications.find((a) => a.invocationId === selected);
  return (
    <section className="jev-queries" aria-label="JEV node queries">
      <header>
        <div>
          <h2>JEV queries</h2>
          <p>{queries.length} independent evaluations · latest first</p>
        </div>
        <button onClick={() => setRefresh((r) => r + 1)}>Refresh queries</button>
      </header>
      {error && <p role="alert">{error}</p>}
      <div className="jev-query-workspace">
        <ol className="jev-query-list" aria-label="Query history">
          {queries.map((query) => (
            <li key={query.invocationId}>
              <button
                aria-pressed={selected === query.invocationId}
                onClick={() => setSelected(query.invocationId)}
              >
                <span>
                  <time dateTime={query.createdAt}>
                    {new Date(query.createdAt).toLocaleTimeString()}
                  </time>{' '}
                  · {query.status}
                </span>
                <strong>{query.inputPreview}</strong>
                <small>
                  {query.questionCount} questions
                  {query.startedAt && query.completedAt
                    ? ` · ${((Date.parse(query.completedAt) - Date.parse(query.startedAt)) / 1000).toFixed(2)}s`
                    : ''}
                </small>
                <span className="jev-query-summary">{[...new Set(query.answers)].join(' · ')}</span>
              </button>
            </li>
          ))}
        </ol>
        {!queries.length && <p>No queries recorded for this node yet.</p>}
        {selected && !detail && <p role="status">Loading query…</p>}
        {detail && (
          <article className="jev-query-detail" aria-label="Selected JEV query">
            <header>
              <div>
                <h3>Evaluation</h3>
                <p>{selectedQuery?.inputPreview}</p>
                <small>
                  {detail.invocation.id} · {detail.invocation.status}
                </small>
              </div>
              <label>
                <input
                  type="checkbox"
                  checked={compare}
                  disabled={!previousId}
                  onChange={(e) => setCompare(e.target.checked)}
                />{' '}
                Compare previous evaluation
              </label>
            </header>
            <section className="jev-applied">
              <h3>Applied decision</h3>
              {application ? (
                <>
                  <table>
                    <thead>
                      <tr>
                        <th>Sentence / revision</th>
                        <th>Application</th>
                        <th>Effective relevance</th>
                        <th>Readiness / reaction</th>
                      </tr>
                    </thead>
                    <tbody>
                      {application.decisions.map((d) => (
                        <tr key={d.sentenceId}>
                          <td>
                            {d.sentenceId.slice(0, 8)} · r{d.revision}
                          </td>
                          <td>
                            {d.applied
                              ? d.buffered
                                ? 'Buffered'
                                : 'Not accumulated'
                              : `Superseded by r${d.currentRevision}`}
                          </td>
                          <td>
                            {d.relevance ?? 'Not applied'}
                            {d.relevanceRetained && <strong> · retained earlier relevance</strong>}
                          </td>
                          <td>
                            {d.completion} · {d.reaction}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {application.releaseId ? (
                    <p>
                      Released request: {application.releaseId}{' '}
                      {onShowRelease && (
                        <button onClick={() => onShowRelease(application.releaseId!)}>
                          Show recorded route
                        </button>
                      )}
                    </p>
                  ) : (
                    <p>No request release recorded for this evaluation.</p>
                  )}
                </>
              ) : (
                <p>
                  No applied-decision evidence is available in this view. The provider result alone
                  does not establish delivery.
                </p>
              )}
            </section>
            <details>
              <summary>Exact submitted context</summary>
              <StructuredValue value={detail.evaluation.state} />
            </details>
            {compare && previous && (
              <div className="jev-comparison">
                <p>
                  Compared with {new Date(previous.invocation.createdAt).toLocaleString()} ·{' '}
                  {previous.invocation.id} · {previous.invocation.status}. Independent evaluation;
                  matching question identities only.
                </p>
                <details>
                  <summary>Previous submitted context</summary>
                  <StructuredValue value={previous.evaluation.state} />
                </details>
              </div>
            )}
            <div className="jev-questions">
              {Object.entries(detail.evaluation.questions).map(([id, question]) => (
                <section key={id} aria-label={`Question ${id}`}>
                  <header>
                    <h3>{id.split(':')[0]}</h3>
                    <small>
                      {question.type} · {id.includes(':') ? id.slice(id.indexOf(':') + 1) : id}
                    </small>
                  </header>
                  {questionSubject?.(detail, id) && <p>{questionSubject(detail, id)}</p>}
                  <div className="jev-question-columns">
                    <div>
                      <details>
                        <summary>Instructions</summary>
                        <StructuredValue value={question.instructions} />
                      </details>
                      <details>
                        <summary>Criteria</summary>
                        <StructuredValue value={question.criteria} />
                      </details>
                    </div>
                    <Answer answer={detail.result?.answers[id]} />
                  </div>
                  {compare && previous && (
                    <p className="jev-comparison">
                      Previous:{' '}
                      {!previous.evaluation.questions[id]
                        ? 'No corresponding question in the previous evaluation'
                        : previous.result && !previous.result.answers[id]
                          ? 'No recorded answer'
                          : answerLabel(previous.result?.answers[id])}
                      {previous.result?.answers[id] &&
                      answerLabel(previous.result.answers[id]) !==
                        answerLabel(detail.result?.answers[id])
                        ? ' → changed'
                        : ''}
                    </p>
                  )}
                </section>
              ))}
            </div>
            <footer>
              <p>
                Model:{' '}
                {detail.result?.model ??
                  detail.invocation.effectiveOptions?.model ??
                  detail.invocation.requestedOptions.model}
              </p>
              <p>
                Submitted: {new Date(detail.invocation.createdAt).toLocaleString()} · Started:{' '}
                {detail.invocation.startedAt
                  ? new Date(detail.invocation.startedAt).toLocaleTimeString()
                  : 'Pending'}{' '}
                · Finished:{' '}
                {detail.invocation.completedAt
                  ? new Date(detail.invocation.completedAt).toLocaleTimeString()
                  : 'Pending'}
              </p>
              {detail.result && (
                <p>
                  Tokens: {detail.result.usage.input_tokens} input ·{' '}
                  {detail.result.usage.output_tokens} output
                </p>
              )}
              {detail.invocation.runtimeError && (
                <p role="alert">{detail.invocation.runtimeError.message}</p>
              )}
              <details>
                <summary>Raw query and response</summary>
                <pre>
                  {JSON.stringify(
                    { evaluation: detail.evaluation, result: detail.result },
                    null,
                    2,
                  )}
                </pre>
              </details>
            </footer>
          </article>
        )}
      </div>
    </section>
  );
}
