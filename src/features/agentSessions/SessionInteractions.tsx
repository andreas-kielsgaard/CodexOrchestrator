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
  const [answers, setAnswers] = useState<Record<string, QuestionAnswer>>({});
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
        <QuestionField
          key={question.id}
          question={question}
          answer={answers[question.id] ?? EMPTY_ANSWER}
          disabled={!pending || busy}
          onChange={(answer) => setAnswers({ ...answers, [question.id]: answer })}
        />
      ))}
      {request.content.questions?.length ? (
        <button
          type="button"
          disabled={
            !pending ||
            busy ||
            request.content.questions.some(
              (question) => answerValues(question, answers[question.id]).length === 0,
            )
          }
          onClick={() =>
            void respond({
              kind: 'answer',
              answers: Object.fromEntries(
                (request.content.questions ?? []).map((question) => [
                  question.id,
                  answerValues(question, answers[question.id]),
                ]),
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

type RuntimeQuestionDto = NonNullable<SessionInteractionDto['content']['questions']>[number];

/** Offered options the user selected, and a typed answer where the question accepts one. */
interface QuestionAnswer {
  readonly selected: readonly string[];
  readonly typed: string;
}

const EMPTY_ANSWER: QuestionAnswer = { selected: [], typed: '' };

/** A typed answer replaces the selection of a single-answer question and adds to a multi-select. */
function answerValues(question: RuntimeQuestionDto, answer: QuestionAnswer | undefined): string[] {
  const typed = answer?.typed.trim() ?? '';
  const selected = answer?.selected ?? [];
  if (!question.multiSelect) return typed ? [typed] : selected.slice(0, 1);
  return typed ? [...selected, typed] : [...selected];
}

function QuestionField({
  question,
  answer,
  disabled,
  onChange,
}: {
  question: RuntimeQuestionDto;
  answer: QuestionAnswer;
  disabled: boolean;
  onChange: (answer: QuestionAnswer) => void;
}) {
  const options = question.options ?? [];
  const typedInput = (label: string | undefined) => (
    <input
      type={question.isSecret ? 'password' : 'text'}
      aria-label={label}
      placeholder={label}
      disabled={disabled}
      value={answer.typed}
      onChange={(event) => onChange({ ...answer, typed: event.target.value })}
    />
  );
  if (options.length === 0) {
    return (
      <label>
        {question.header && <small>{question.header}</small>}
        <span>{question.question}</span>
        {typedInput(undefined)}
      </label>
    );
  }
  return (
    <fieldset className="session-runtime-question">
      <legend>
        {question.header && <small>{question.header}</small>}
        <span>{question.question}</span>
      </legend>
      {question.multiSelect ? (
        options.map((option) => (
          <label key={option.label}>
            <input
              type="checkbox"
              disabled={disabled}
              checked={answer.selected.includes(option.label)}
              onChange={(event) =>
                onChange({
                  ...answer,
                  selected: event.target.checked
                    ? [...answer.selected, option.label]
                    : answer.selected.filter((label) => label !== option.label),
                })
              }
            />
            {option.label}
            {option.description && <small> — {option.description}</small>}
          </label>
        ))
      ) : (
        <select
          aria-label={question.question}
          disabled={disabled}
          value={answer.selected[0] ?? ''}
          onChange={(event) =>
            onChange({ ...answer, selected: event.target.value ? [event.target.value] : [] })
          }
        >
          <option value="">Choose an answer</option>
          {options.map((option) => (
            <option key={option.label} value={option.label}>
              {option.description ? `${option.label} — ${option.description}` : option.label}
            </option>
          ))}
        </select>
      )}
      {question.isOther && typedInput('Or type an answer')}
    </fieldset>
  );
}
