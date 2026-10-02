import { AlertCircle, Circle, Clock3, CornerDownRight, Send, LoaderCircle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { presentSessionQueue } from '../../application/agentSessions/sessionQueuePresentation';
import type {
  ProjectedQueueEntry,
  SessionQueueEntryViewDto,
  SessionQueueViewDto,
} from '../../application/agentSessions/sessionQueue';
import type { AgentSessionWorkspaceController } from './useAgentSession';
import type { SessionInteractionDto } from '../../application/agentSessions';
import { SessionQueueItemDialog } from './SessionQueueItemDialog';
import { queueEntryState, queueEntryTitle } from './sessionQueueLabels';
import './sessionQueue.css';
import { SteeringEditor } from './SteeringEditor';

const terminalStates = new Set(['completed', 'canceled']);

export function SessionQueuePanel({
  queue,
  steeringEntries = [],
  controller,
  onChooseTarget,
}: {
  readonly queue: SessionQueueViewDto;
  readonly steeringEntries?: readonly SessionInteractionDto[];
  readonly controller: AgentSessionWorkspaceController;
  readonly onChooseTarget: (entry: SessionQueueEntryViewDto) => void;
}) {
  const projected = useMemo(() => presentSessionQueue(queue).futureEntries, [queue]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [steeringEdit, setSteeringEdit] = useState<SessionInteractionDto | null>(null);
  const [steeringError, setSteeringError] = useState<string | null>(null);
  const [dragged, setDragged] = useState<string | null>(null);
  if (!projected.length && !steeringEntries.length && !steeringEdit) return null;
  const visibleSteering = steeringEntries.filter((i) => i.id !== steeringEdit?.id);

  const ids = projected.map(({ item }) => item.entry.id);
  const hiddenIds = queue.entries
    .filter(({ entry }) => !terminalStates.has(entry.state) && !ids.includes(entry.id))
    .map(({ entry }) => entry.id);
  const reorder = (next: readonly string[]) =>
    void controller.reorderQueue?.([...hiddenIds, ...next]);
  const move = (entryId: string, offset: number) => {
    const from = ids.indexOf(entryId);
    const to = from + offset;
    if (from < 0 || to < 0 || to >= ids.length) return;
    const next = [...ids];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    reorder(next);
  };
  const dropBefore = (targetId: string) => {
    if (!dragged || dragged === targetId) return;
    const from = ids.indexOf(dragged);
    const to = ids.indexOf(targetId);
    if (from < 0 || to < 0) return;
    const next = [...ids];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    setDragged(null);
    reorder(next);
  };
  const editingIndex = projected.findIndex(({ item }) => item.entry.id === editingId);

  return (
    <>
      <section className="session-queue" aria-label="Queued messages, steering, and target changes">
        <ol>
          {visibleSteering.map((interaction) => (
            <SteeringLine
              interaction={interaction}
              key={interaction.id}
              onEdit={
                controller.editSteering &&
                ['queued', 'editing', 'expired'].includes(interaction.state)
                  ? async () => {
                      try {
                        setSteeringError(null);
                        setSteeringEdit(await controller.editSteering!(interaction, 'begin'));
                      } catch (cause) {
                        setSteeringError(String(cause));
                      }
                    }
                  : undefined
              }
            />
          ))}
          {projected.map((projection) => (
            <QueueLine
              key={projection.item.entry.id}
              projection={projection}
              onOpen={() => setEditingId(projection.item.entry.id)}
              onSendNow={
                controller.sendQueueEntryNow
                  ? () => controller.sendQueueEntryNow!(projection.item.entry.id)
                  : undefined
              }
              onDragStart={() => setDragged(projection.item.entry.id)}
              onDrop={() => dropBefore(projection.item.entry.id)}
            />
          ))}
        </ol>
        {projected.length + visibleSteering.length > 1 && (
          <span className="session-queue__more">
            +{projected.length + visibleSteering.length - 1} more
          </span>
        )}
      </section>
      {steeringError && <p role="alert">{steeringError}</p>}
      {steeringEdit && (
        <SteeringEditor
          interaction={steeringEdit}
          controller={controller}
          onClose={() => setSteeringEdit(null)}
        />
      )}
      {editingIndex >= 0 && (
        <SessionQueueItemDialog
          projection={projected[editingIndex]}
          index={editingIndex}
          count={projected.length}
          controller={controller}
          onChooseTarget={onChooseTarget}
          onMove={move}
          onClose={() => setEditingId(null)}
        />
      )}
    </>
  );
}

function SteeringLine({
  interaction,
  onEdit,
}: {
  readonly interaction: SessionInteractionDto;
  onEdit?: () => Promise<void>;
}) {
  return (
    <li className="is-steering" aria-label={`Steering: ${interaction.content.text ?? ''}`}>
      <button
        type="button"
        className="session-queue__line"
        disabled={!onEdit}
        onClick={() => void onEdit?.()}
        aria-label={`Edit steering: ${interaction.content.text ?? ''}`}
      >
        <span className="session-queue__state" aria-hidden="true">
          <CornerDownRight size={14} />
        </span>
        <strong>{interaction.content.text || 'Steering message'}</strong>
        <small>
          {interaction.state === 'queued'
            ? 'Steering next'
            : interaction.state === 'editing'
              ? 'Editing paused'
              : interaction.state === 'expired'
                ? 'Turn ended before delivery'
                : interaction.state === 'accepted'
                  ? 'Delivered to agent'
                  : interaction.state === 'uncertain'
                    ? 'Delivery uncertain'
                    : interaction.state === 'rejected'
                      ? 'Delivery failed'
                      : 'Delivering steering'}
        </small>
      </button>
    </li>
  );
}

function QueueLine({
  projection,
  onOpen,
  onSendNow,
  onDragStart,
  onDrop,
}: {
  readonly projection: ProjectedQueueEntry;
  readonly onOpen: () => void;
  readonly onSendNow?: () => Promise<boolean>;
  readonly onDragStart: () => void;
  readonly onDrop: () => void;
}) {
  const item = projection.item;
  const [sending, setSending] = useState(false);
  const blocked = item.entry.state === 'blocked' || item.targetOperation?.phase === 'blocked';
  return (
    <li
      draggable
      onDragStart={onDragStart}
      onDragOver={(event) => event.preventDefault()}
      onDrop={onDrop}
      className={blocked ? 'is-blocked' : undefined}
    >
      <div className="session-queue__row">
        <button
          type="button"
          onClick={onOpen}
          aria-label={`${queueEntryState(item)}: ${queueEntryTitle(item)}`}
        >
          <span className="session-queue__state" aria-hidden="true">
            {blocked ? (
              <AlertCircle size={14} />
            ) : item.targetOperation?.phase === 'staged' ? (
              <Clock3 size={14} />
            ) : (
              <Circle size={14} />
            )}
          </span>
          <strong>{queueEntryTitle(item)}</strong>
          <small>{queueEntryState(item)}</small>
        </button>
        {onSendNow && item.entry.item.kind === 'message' && item.entry.state === 'pending' && (
          <button
            type="button"
            className="session-queue__send-now"
            aria-label={`Steer Turn: ${queueEntryTitle(item)}`}
            title="Steer the running turn using its current settings, or send next if idle"
            disabled={sending}
            onClick={() => {
              setSending(true);
              void onSendNow().finally(() => setSending(false));
            }}
          >
            {sending ? (
              <LoaderCircle size={15} className="session-queue__spinner" />
            ) : (
              <Send size={15} />
            )}
          </button>
        )}
      </div>
    </li>
  );
}
