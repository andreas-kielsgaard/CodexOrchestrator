import { useId, useState } from 'react';
import type { RuntimeInteractionResponseDto, SessionInteractionDto } from '../../application/agentSessions';

export function SessionInteractions({
  interactions,
  onRespond,
}: {
  interactions: readonly SessionInteractionDto[];
  onRespond?(invocationId: string, requestId: string, response: RuntimeInteractionResponseDto): Promise<void>;
}) {
  return (
    <div className="session-interactions" aria-label="Turn inputs and requests">
      {interactions.map((interaction) =>
        interaction.kind === 'steering' ? (
          <article key={interaction.id} className="session-steering-input">
            <strong>Steering · {interaction.state}</strong>
            <p>{interaction.content.text}</p>
            {interaction.result && <p role="status">{interaction.result}</p>}
          </article>
        ) : (
          <RuntimeRequest key={interaction.id} request={interaction} onRespond={onRespond} />
        ),
      )}
    </div>
  );
}

function RuntimeRequest({
  request,
  onRespond,
}: {
  request: SessionInteractionDto;
  onRespond?: (invocationId: string, requestId: string, response: RuntimeInteractionResponseDto) => Promise<void>;
}) {
  const choiceDescriptionId = useId();
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pending = request.state === 'pending' && Boolean(onRespond);
  const respond = async (response: RuntimeInteractionResponseDto) => {
    if (!pending || busy) return;
    setBusy(true);
    setError(null);
    try {
      await onRespond?.(request.invocationId, request.id, response);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  };
  return (
    <article
      className="session-runtime-request"
      aria-label="Agent request"
      data-request-id={request.id}
      tabIndex={-1}
    >
      <strong>
        {request.content.title ?? 'Agent request'} · {request.state}
      </strong>
      {request.content.command && <pre>{request.content.command}</pre>}
      {request.content.cwd && <p>{request.content.cwd}</p>}
      {request.content.grantRoot && <p>Requested write access: {request.content.grantRoot}</p>}
      {request.content.permissions != null && (
        <pre>{JSON.stringify(request.content.permissions, null, 2)}</pre>
      )}
      {request.content.url && /^https?:\/\//i.test(request.content.url) && (
        <a href={request.content.url} target="_blank" rel="noreferrer">
          Open requested page
        </a>
      )}
      {request.content.choices?.map((choice, index) => (
        <div className="session-request-choice" key={index}>
          <button
            type="button"
            aria-describedby={choice.description ? `${choiceDescriptionId}-${index}` : undefined}
            disabled={!pending || busy}
            onClick={() => void respond({ kind: 'choose', choiceId: choice.id })}
          >
            {choice.label}
          </button>
          {choice.description && <p id={`${choiceDescriptionId}-${index}`}>{choice.description}</p>}
          {choice.scope && (
            <details>
              <summary>Rule scope</summary>
              <pre>{choice.scope}</pre>
            </details>
          )}
        </div>
      ))}
      {request.content.questions?.map((question) => (
        <label key={question.id}>
          <span>{question.question}</span>
          {question.options?.length && !question.isOther ? (
            <select
              disabled={!pending || busy}
              value={answers[question.id] ?? ''}
              onChange={(event) => setAnswers({ ...answers, [question.id]: event.target.value })}
            >
              <option value="">Choose an answer</option>
              {question.options.map((option) => (
                <option key={option.label} value={option.label}>
                  {option.label} — {option.description}
                </option>
              ))}
            </select>
          ) : (
            <input
              type={question.isSecret ? 'password' : 'text'}
              disabled={!pending || busy}
              value={answers[question.id] ?? ''}
              onChange={(event) => setAnswers({ ...answers, [question.id]: event.target.value })}
            />
          )}
        </label>
      ))}
      {request.content.questions?.length ? (
        <button
          type="button"
          disabled={
            !pending || busy || request.content.questions.some((q) => !answers[q.id]?.trim())
          }
          onClick={() =>
            void respond({
              kind: 'answer',
              answers: Object.fromEntries(
                Object.entries(answers).map(([id, answer]) => [id, [answer]]),
              ),
            })
          }
        >
          Submit answers
        </button>
      ) : null}
      {error && <p role="alert">{error}</p>}
      {request.result && <p role="status">{request.result}</p>}
      {request.state === 'unsupported' && <p>This request is not supported by this integration.</p>}
    </article>
  );
}
