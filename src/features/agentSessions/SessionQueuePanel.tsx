import { AlertCircle, Circle, Clock3, CornerDownRight } from 'lucide-react';
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
  const [dragged, setDragged] = useState<string | null>(null);
  if (!projected.length && !steeringEntries.length) return null;

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
          {steeringEntries.map((interaction) => (
            <SteeringLine interaction={interaction} key={interaction.id} />
          ))}
          {projected.map((projection) => (
            <QueueLine
              key={projection.item.entry.id}
              projection={projection}
              onOpen={() => setEditingId(projection.item.entry.id)}
              onDragStart={() => setDragged(projection.item.entry.id)}
              onDrop={() => dropBefore(projection.item.entry.id)}
            />
          ))}
        </ol>
        {projected.length + steeringEntries.length > 1 && (
          <span className="session-queue__more">
            +{projected.length + steeringEntries.length - 1} more
          </span>
        )}
      </section>
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

function SteeringLine({ interaction }: { readonly interaction: SessionInteractionDto }) {
  return (
    <li className="is-steering" aria-label={`Steering: ${interaction.content.text ?? ''}`}>
      <div className="session-queue__line">
        <span className="session-queue__state" aria-hidden="true">
          <CornerDownRight size={14} />
        </span>
        <strong>{interaction.content.text || 'Steering message'}</strong>
        <small>Steering next</small>
      </div>
    </li>
  );
}

function QueueLine({
  projection,
  onOpen,
  onDragStart,
  onDrop,
}: {
  readonly projection: ProjectedQueueEntry;
  readonly onOpen: () => void;
  readonly onDragStart: () => void;
  readonly onDrop: () => void;
}) {
  const item = projection.item;
  const blocked = item.entry.state === 'blocked' || item.targetOperation?.phase === 'blocked';
  return (
    <li
      draggable
      onDragStart={onDragStart}
      onDragOver={(event) => event.preventDefault()}
      onDrop={onDrop}
      className={blocked ? 'is-blocked' : undefined}
    >
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
    </li>
  );
}
