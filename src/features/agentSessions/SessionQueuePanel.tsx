import {
  AlertCircle,
  ArrowDown,
  ArrowUp,
  Check,
  Circle,
  Clock3,
  LoaderCircle,
  Pencil,
  Play,
  RefreshCw,
  Trash2,
  X,
} from 'lucide-react';
import { useMemo, useState, type FormEvent } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import {
  projectSessionQueueTargets,
  type ProjectedQueueEntry,
  type SessionQueueEntryViewDto,
  type SessionQueueViewDto,
} from '../../application/agentSessions/sessionQueue';
import type { SandboxModeDto } from '../../application/executionConfiguration';
import type {
  SessionExecutionSelectionDto,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import type { AgentSessionWorkspaceController } from './useAgentSession';
import './sessionQueue.css';

const terminalStates = new Set(['completed', 'canceled']);

function targetLabel(
  target: SessionExecutionSelectionDto | SessionExecutionTargetDto | null,
): { device: string; workspace: string } {
  if (!target) return { device: 'Current device', workspace: 'Current workspace' };
  if ('workspace' in target) {
    const workspace = target.workspace;
    if (workspace.kind === 'existing')
      return {
        device: target.execution.deviceName,
        workspace: workspace.target.path,
      };
    if (workspace.kind === 'create')
      return {
        device: target.execution.deviceName,
        workspace: `${workspace.branchRef.replace(/^refs\/heads\//, '')} · create worktree`,
      };
    return { device: target.execution.deviceName, workspace: 'Empty workspace' };
  }
  return { device: target.execution.deviceName, workspace: target.path };
}

function entryTitle(item: SessionQueueEntryViewDto) {
  if (item.message) return item.message.invocation.submittedText.split(/\r?\n/, 1)[0] || 'Message';
  return `Switch to ${item.targetOperation?.destinationSelection.execution.deviceName ?? 'destination'}`;
}

function stateLabel(item: SessionQueueEntryViewDto) {
  if (item.targetOperation) {
    return {
      pending: 'Waiting',
      staging: 'Preparing',
      staged: 'Ready',
      activating: 'Switching',
      completed: 'Completed',
      blocked: 'Blocked',
      canceled: 'Canceled',
    }[item.targetOperation.phase];
  }
  return {
    pending: 'Queued',
    executing: 'Running',
    blocked: 'Blocked',
    completed: 'Completed',
    canceled: 'Canceled',
  }[item.entry.state];
}

function StateIcon({ item }: { readonly item: SessionQueueEntryViewDto }) {
  const state = item.targetOperation?.phase ?? item.entry.state;
  if (state === 'completed') return <Check size={15} aria-hidden="true" />;
  if (state === 'blocked') return <AlertCircle size={15} aria-hidden="true" />;
  if (state === 'executing' || state === 'staging' || state === 'activating')
    return <LoaderCircle className="session-queue__spinner" size={15} aria-hidden="true" />;
  if (state === 'staged') return <Clock3 size={15} aria-hidden="true" />;
  return <Circle size={15} aria-hidden="true" />;
}

function MessageEditor({
  item,
  onClose,
  onSave,
}: {
  readonly item: SessionQueueEntryViewDto;
  readonly onClose: () => void;
  readonly onSave: (input: {
    entryId: string;
    submittedText: string;
    model: string | null;
    reasoningMode: string | null;
    sandboxMode: SandboxModeDto | null;
  }) => Promise<void> | void;
}) {
  const message = item.message!;
  const [text, setText] = useState(message.invocation.submittedText);
  const [model, setModel] = useState(message.preparation.model ?? message.invocation.requestedOptions.model ?? '');
  const [reasoning, setReasoning] = useState(message.preparation.reasoningMode ?? '');
  const [sandbox, setSandbox] = useState<SandboxModeDto | ''>(
    message.preparation.sandboxMode ?? message.invocation.requestedOptions.sandbox ?? '',
  );
  const [saving, setSaving] = useState(false);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!text.trim() || saving) return;
    setSaving(true);
    await onSave({
      entryId: item.entry.id,
      submittedText: text.trim(),
      model: model.trim() || null,
      reasoningMode: reasoning.trim() || null,
      sandboxMode: sandbox || null,
    });
    setSaving(false);
    onClose();
  };
  return (
    <ModalDialog labelledBy="queued-message-editor-title" onClose={onClose} className="session-queue-editor">
      <header>
        <div>
          <p>Queued message</p>
          <h2 id="queued-message-editor-title">Edit message</h2>
        </div>
        <button type="button" onClick={onClose} aria-label="Close message editor"><X size={18} /></button>
      </header>
      <form onSubmit={(event) => void submit(event)}>
        <label>
          <span>Message</span>
          <textarea rows={8} value={text} onChange={(event) => setText(event.target.value)} />
        </label>
        <div className="session-queue-editor__options">
          <label>
            <span>Model</span>
            <input value={model} onChange={(event) => setModel(event.target.value)} placeholder="Profile default" />
          </label>
          <label>
            <span>Reasoning</span>
            <input value={reasoning} onChange={(event) => setReasoning(event.target.value)} placeholder="Model default" />
          </label>
          <label>
            <span>Sandbox</span>
            <select value={sandbox} onChange={(event) => setSandbox(event.target.value as SandboxModeDto | '')}>
              <option value="">Profile default</option>
              <option value="read_only">Read only</option>
              <option value="workspace_write">Workspace write</option>
              <option value="danger_full_access">Full access</option>
            </select>
          </label>
        </div>
        <footer>
          <button type="button" onClick={onClose}>Cancel</button>
          <button type="submit" disabled={!text.trim() || saving}>{saving ? 'Saving…' : 'Save message'}</button>
        </footer>
      </form>
    </ModalDialog>
  );
}

function TargetOperationEditor({
  item,
  onClose,
  onChooseTarget,
}: {
  readonly item: SessionQueueEntryViewDto;
  readonly onClose: () => void;
  readonly onChooseTarget: (item: SessionQueueEntryViewDto) => void;
}) {
  const operation = item.targetOperation!;
  const target = targetLabel(operation.destinationSelection);
  return (
    <ModalDialog labelledBy="target-operation-editor-title" onClose={onClose} className="session-queue-editor">
      <header>
        <div>
          <p>Target operation</p>
          <h2 id="target-operation-editor-title">Edit destination</h2>
        </div>
        <button type="button" onClick={onClose} aria-label="Close target operation editor"><X size={18} /></button>
      </header>
      <dl>
        <dt>Device</dt><dd>{target.device}</dd>
        <dt>Workspace</dt><dd>{target.workspace}</dd>
        <dt>Status</dt><dd>{stateLabel(item)}</dd>
      </dl>
      <p>Choosing another destination resets this operation’s preparation and starts the new preparation early.</p>
      <footer>
        <button type="button" onClick={onClose}>Cancel</button>
        <button type="button" onClick={() => { onClose(); onChooseTarget(item); }}>Choose another destination…</button>
      </footer>
    </ModalDialog>
  );
}

export function SessionQueuePanel({
  queue,
  controller,
  onChooseTarget,
}: {
  readonly queue: SessionQueueViewDto;
  readonly controller: AgentSessionWorkspaceController;
  readonly onChooseTarget: (entry: SessionQueueEntryViewDto) => void;
}) {
  const projected = useMemo(
    () => projectSessionQueueTargets(queue).filter(({ item }) => !terminalStates.has(item.entry.state)),
    [queue],
  );
  const [editing, setEditing] = useState<SessionQueueEntryViewDto | null>(null);
  const [dragged, setDragged] = useState<string | null>(null);
  if (!projected.length) return null;
  const ids = projected.map(({ item }) => item.entry.id);
  const executingIndex = projected.findIndex(
    ({ item }) => item.entry.state === 'executing' || item.targetOperation?.phase === 'activating',
  );
  const minimumMovableIndex = executingIndex < 0 ? 0 : executingIndex + 1;
  const move = (entryId: string, offset: number) => {
    const from = ids.indexOf(entryId);
    const to = from + offset;
    if (
      from < minimumMovableIndex ||
      to < minimumMovableIndex ||
      to >= ids.length ||
      projected[from].item.entry.state === 'executing'
    )
      return;
    const next = [...ids];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    void controller.reorderQueue?.(next);
  };
  const dropBefore = (targetId: string) => {
    if (!dragged || dragged === targetId) return;
    const from = ids.indexOf(dragged);
    const to = ids.indexOf(targetId);
    if (
      from < minimumMovableIndex ||
      to < minimumMovableIndex ||
      projected[from].item.entry.state === 'executing'
    )
      return;
    const next = [...ids];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    setDragged(null);
    void controller.reorderQueue?.(next);
  };
  return (
    <>
      <section className="session-queue" aria-label="Session queue">
        <header>
          <div><h3>Queue</h3><span>{projected.length}</span></div>
          <p>Messages use the destination configured above them.</p>
        </header>
        <ol>
          {projected.map((projection, index) => (
            <QueueCard
              key={projection.item.entry.id}
              projection={projection}
              index={index}
              count={projected.length}
              canMove={projection.item.entry.state !== 'executing'}
              minimumMovableIndex={minimumMovableIndex}
              onMove={move}
              onEdit={() => setEditing(projection.item)}
              onDragStart={() => setDragged(projection.item.entry.id)}
              onDrop={() => dropBefore(projection.item.entry.id)}
              onStart={() => void controller.startQueueEntry?.(projection.item.entry.id)}
              onRetry={() => void controller.retryQueueEntry?.(projection.item.entry.id)}
              onCancel={() => void controller.cancelQueueEntry?.(projection.item.entry.id)}
            />
          ))}
        </ol>
      </section>
      {editing?.message && (
        <MessageEditor
          item={editing}
          onClose={() => setEditing(null)}
          onSave={(input) => controller.updateQueuedMessage?.(input)}
        />
      )}
      {editing?.targetOperation && (
        <TargetOperationEditor
          item={editing}
          onClose={() => setEditing(null)}
          onChooseTarget={onChooseTarget}
        />
      )}
    </>
  );
}

function QueueCard({
  projection,
  index,
  count,
  canMove,
  minimumMovableIndex,
  onMove,
  onEdit,
  onDragStart,
  onDrop,
  onStart,
  onRetry,
  onCancel,
}: {
  readonly projection: ProjectedQueueEntry;
  readonly index: number;
  readonly count: number;
  readonly canMove: boolean;
  readonly minimumMovableIndex: number;
  readonly onMove: (id: string, offset: number) => void;
  readonly onEdit: () => void;
  readonly onDragStart: () => void;
  readonly onDrop: () => void;
  readonly onStart: () => void;
  readonly onRetry: () => void;
  readonly onCancel: () => void;
}) {
  const { item } = projection;
  const target = targetLabel(projection.target);
  const blocked = item.entry.state === 'blocked' || item.targetOperation?.phase === 'blocked';
  const editable = item.entry.state !== 'executing';
  return (
    <li
      className={`session-queue__entry session-queue__entry--${item.entry.state}`}
      draggable={canMove}
      onDragStart={onDragStart}
      onDragOver={(event) => event.preventDefault()}
      onDrop={onDrop}
    >
      <button type="button" className="session-queue__card" onClick={onEdit} disabled={!editable}>
        <span className="session-queue__state"><StateIcon item={item} />{stateLabel(item)}</span>
        <strong>{entryTitle(item)}</strong>
        {item.message && <small>Will run on {target.device} · {target.workspace}</small>}
        {item.message && (item.message.preparation.model || item.message.preparation.reasoningMode) && (
          <small>{[item.message.preparation.model, item.message.preparation.reasoningMode].filter(Boolean).join(' · ')}</small>
        )}
        {item.targetOperation && (
          <ol className="session-queue__steps">
            {item.targetOperation.actions.map((action) => (
              <li key={action.kind} className={`is-${action.status}`}>
                <span>{action.status === 'completed' ? <Check size={12} /> : <Circle size={12} />}</span>
                <span>{action.kind.replaceAll('_', ' ')}</span>
              </li>
            ))}
          </ol>
        )}
        {(item.entry.blockedReason || item.targetOperation?.error) && (
          <small className="session-queue__error">{item.entry.blockedReason ?? item.targetOperation?.error}</small>
        )}
      </button>
      <div className="session-queue__actions">
        <button type="button" onClick={() => onMove(item.entry.id, -1)} disabled={!canMove || index <= minimumMovableIndex} aria-label="Move queue item up"><ArrowUp size={14} /></button>
        <button type="button" onClick={() => onMove(item.entry.id, 1)} disabled={!canMove || index === count - 1} aria-label="Move queue item down"><ArrowDown size={14} /></button>
        <button type="button" onClick={onEdit} disabled={!editable} aria-label="Edit queue item"><Pencil size={14} /></button>
        {item.targetOperation?.phase === 'pending' && <button type="button" onClick={onStart} aria-label="Start target preparation"><Play size={14} /></button>}
        {blocked && <button type="button" onClick={onRetry} aria-label="Retry queue item"><RefreshCw size={14} /></button>}
        {editable && <button type="button" onClick={onCancel} aria-label="Cancel queue item"><Trash2 size={14} /></button>}
      </div>
    </li>
  );
}
