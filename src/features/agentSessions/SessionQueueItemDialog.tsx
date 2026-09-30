import { AlertCircle, Check, Circle, LoaderCircle, X } from 'lucide-react';
import { useState, type FormEvent } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import type {
  ProjectedQueueEntry,
  SessionQueueEntryViewDto,
} from '../../application/agentSessions/sessionQueue';
import type { SandboxModeDto } from '../../application/executionConfiguration';
import type { AgentSessionWorkspaceController } from './useAgentSession';
import {
  queueEntryState,
  queueEntryTitle,
  queueTargetLabel,
  targetActionLabel,
} from './sessionQueueLabels';

export function SessionQueueItemDialog({
  projection,
  index,
  count,
  controller,
  onChooseTarget,
  onMove,
  onClose,
}: {
  readonly projection: ProjectedQueueEntry;
  readonly index: number;
  readonly count: number;
  readonly controller: AgentSessionWorkspaceController;
  readonly onChooseTarget: (item: SessionQueueEntryViewDto) => void;
  readonly onMove: (entryId: string, offset: number) => void;
  readonly onClose: () => void;
}) {
  const item = projection.item;
  const compaction = item.entry.item.kind === 'compaction';
  const editable = item.entry.state === 'pending' || item.entry.state === 'blocked';
  const blocked = item.entry.state === 'blocked' || item.targetOperation?.phase === 'blocked';
  const finish = (action: (() => void) | undefined) => {
    action?.();
    onClose();
  };

  return (
    <ModalDialog
      labelledBy="session-queue-item-title"
      onClose={onClose}
      className="session-queue-editor"
    >
      <header>
        <div>
          <p>
            {compaction ? 'Queued command' : item.message ? 'Queued message' : 'Target operation'}
          </p>
          <h2 id="session-queue-item-title">{queueEntryTitle(item)}</h2>
        </div>
        <button type="button" onClick={onClose} aria-label="Close queue item">
          <X size={18} />
        </button>
      </header>
      {compaction ? (
        <CompactionDetails item={item} target={projection.target} />
      ) : item.message ? (
        <MessageDetails
          item={item}
          target={projection.target}
          controller={controller}
          onClose={onClose}
        />
      ) : (
        <TargetDetails item={item} controller={controller} />
      )}
      <div className="session-queue-editor__controls" aria-label="Queue item controls">
        <button
          type="button"
          disabled={!editable || index === 0}
          onClick={() => onMove(item.entry.id, -1)}
        >
          Move up
        </button>
        <button
          type="button"
          disabled={!editable || index === count - 1}
          onClick={() => onMove(item.entry.id, 1)}
        >
          Move down
        </button>
        {blocked && (
          <button type="button" onClick={() => void controller.retryQueueEntry?.(item.entry.id)}>
            Retry
          </button>
        )}
        {item.targetOperation && editable && (
          <button type="button" onClick={() => finish(() => onChooseTarget(item))}>
            Choose destination…
          </button>
        )}
        {item.message && !compaction && editable && (
          <button type="button" onClick={() => finish(() => onChooseTarget(item))}>
            Change destination…
          </button>
        )}
        {editable && (
          <button
            type="button"
            className="is-danger"
            onClick={() => finish(() => void controller.cancelQueueEntry?.(item.entry.id))}
          >
            Remove
          </button>
        )}
      </div>
    </ModalDialog>
  );
}

function CompactionDetails({
  item,
  target,
}: {
  readonly item: SessionQueueEntryViewDto;
  readonly target: ProjectedQueueEntry['target'];
}) {
  const destination = queueTargetLabel(target);
  return (
    <div className="session-queue-editor__target">
      <dl>
        <dt>Command</dt>
        <dd>/compact</dd>
        <dt>Destination</dt>
        <dd>
          {destination.device} / {destination.workspace}
        </dd>
        <dt>Status</dt>
        <dd>{queueEntryState(item)}</dd>
      </dl>
      {item.entry.blockedReason && (
        <p className="session-queue-editor__error" role="alert">
          <AlertCircle size={15} />
          {item.entry.blockedReason}
        </p>
      )}
    </div>
  );
}

function MessageDetails({
  item,
  target,
  controller,
  onClose,
}: {
  readonly item: SessionQueueEntryViewDto;
  readonly target: ProjectedQueueEntry['target'];
  readonly controller: AgentSessionWorkspaceController;
  readonly onClose: () => void;
}) {
  const message = item.message!;
  const destination = queueTargetLabel(target);
  const [text, setText] = useState(message.invocation.submittedText);
  const [model, setModel] = useState(
    message.preparation.model ?? message.invocation.requestedOptions.model ?? '',
  );
  const [reasoning, setReasoning] = useState(message.preparation.reasoningMode ?? '');
  const [sandbox, setSandbox] = useState<SandboxModeDto | ''>(
    message.preparation.sandboxMode ?? message.invocation.requestedOptions.sandbox ?? '',
  );
  const [saving, setSaving] = useState(false);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!text.trim() || saving) return;
    setSaving(true);
    await controller.updateQueuedMessage?.({
      entryId: item.entry.id,
      submittedText: text.trim(),
      model: model.trim() || null,
      reasoningMode: reasoning.trim() || null,
      sandboxMode: sandbox || null,
    });
    onClose();
  };
  return (
    <form onSubmit={(event) => void submit(event)}>
      <p className="session-queue-editor__destination">
        Will run on {destination.device} · {destination.workspace}
      </p>
      <label>
        <span>Message</span>
        <textarea rows={8} value={text} onChange={(event) => setText(event.target.value)} />
      </label>
      <div className="session-queue-editor__options">
        <label>
          <span>Model</span>
          <input
            value={model}
            onChange={(event) => setModel(event.target.value)}
            placeholder="Profile default"
          />
        </label>
        <label>
          <span>Reasoning</span>
          <input
            value={reasoning}
            onChange={(event) => setReasoning(event.target.value)}
            placeholder="Model default"
          />
        </label>
        <label>
          <span>Sandbox</span>
          <select
            value={sandbox}
            onChange={(event) => setSandbox(event.target.value as SandboxModeDto | '')}
          >
            <option value="">Profile default</option>
            <option value="read_only">Read only</option>
            <option value="workspace_write">Workspace write</option>
            <option value="danger_full_access">Full access</option>
          </select>
        </label>
      </div>
      <footer>
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button type="submit" disabled={!text.trim() || saving}>
          {saving ? 'Saving…' : 'Save message'}
        </button>
      </footer>
    </form>
  );
}

function TargetDetails({
  item,
  controller,
}: {
  readonly item: SessionQueueEntryViewDto;
  readonly controller: AgentSessionWorkspaceController;
}) {
  const operation = item.targetOperation!;
  const target = queueTargetLabel(operation.request.destination);
  const error = item.entry.blockedReason ?? operation.error;
  return (
    <div className="session-queue-editor__target">
      <dl>
        <dt>Device</dt>
        <dd>{target.device}</dd>
        <dt>Workspace</dt>
        <dd>{target.workspace}</dd>
        <dt>Status</dt>
        <dd>{queueEntryState(item)}</dd>
      </dl>
      {error && (
        <p className="session-queue-editor__error" role="alert">
          <AlertCircle size={15} />
          {error}
        </p>
      )}
      <section aria-label="Projected execution plan">
        <h3>Projected when queued</h3>
        <p>
          {new Date(operation.projectedPlan.calculatedAt).toLocaleString()}
          {operation.projectedPlan.stale ? ' · Stale after editing or reordering' : ''}
        </p>
        <button
          type="button"
          className="session-queue-editor__recalculate"
          onClick={() => void controller.recalculateTargetProjection?.(item.entry.id)}
        >
          Recalculate
        </button>
        {[...operation.projectedPlan.facts.blockers, ...operation.projectedPlan.facts.unknown].map(
          (fact, index) => (
            <p key={index}>{fact}</p>
          ),
        )}
        {operation.projectedPlan.steps.length ? (
          <ol>
            {operation.projectedPlan.steps.map((step) => (
              <li key={step.kind}>{targetActionLabel(step.kind)}</li>
            ))}
          </ol>
        ) : (
          <p>No action projected.</p>
        )}
      </section>
      <h3>Actual execution</h3>
      {!operation.actions.length && <p>Evaluated when this entry reaches the front.</p>}
      <ol className="session-queue-editor__steps">
        {operation.actions.map((action) => (
          <li key={action.kind} className={`is-${action.status}`}>
            {action.status === 'completed' ? (
              <Check size={15} />
            ) : action.status === 'running' ? (
              <LoaderCircle className="session-queue__spinner" size={15} />
            ) : (
              <Circle size={15} />
            )}
            <span>
              <strong>{targetActionLabel(action.kind)}</strong>
              {(action.detail || action.error) && <small>{action.detail ?? action.error}</small>}
            </span>
          </li>
        ))}
      </ol>
    </div>
  );
}
