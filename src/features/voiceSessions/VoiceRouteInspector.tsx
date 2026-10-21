import type { VoiceView } from '../../application/voiceSessions';

type Conversation = NonNullable<VoiceView['state']['conversation']>;
export type VoiceActivityRecord = NonNullable<Conversation['activity']>[number];

function voiceActivityText(record: VoiceActivityRecord): string {
  const blocks = Array.isArray(record.data.blocks)
    ? record.data.blocks
        .map((block) =>
          block && typeof block === 'object' && 'text' in block ? String(block.text) : '',
        )
        .join(' ')
    : undefined;
  return (
    [
      record.data.text,
      record.data.reason,
      record.data.summary,
      record.data.finalReply,
      blocks,
    ].find((value): value is string => typeof value === 'string' && !!value) ?? ''
  );
}

export function VoiceRouteInspector({
  view,
  selectedId,
  onSelect,
  onInspectNode,
  onOpenSession,
}: {
  view: VoiceView;
  selectedId: string | null;
  onSelect(record: VoiceActivityRecord): void;
  onInspectNode(id: string): void;
  onOpenSession(id: string, invocationId?: string): void;
}) {
  const records = [...(view.state.conversation?.activity ?? [])].reverse();
  const record = records.find((r) => r.id === selectedId);
  const name = (id: string) => view.workflow?.nodes.find((n) => n.id === id)?.name ?? id;
  return (
    <section aria-label="Recorded route">
      <h2>Recorded routes</h2>
      <p className="voice-muted">
        Select an event to see its route and delivery evidence together. Events may belong to
        parallel work.
      </p>
      {!records.length ? (
        <p>No events recorded for this conversation.</p>
      ) : (
        <label>
          Event
          <select
            value={selectedId ?? ''}
            onChange={(e) => {
              const next = records.find((r) => r.id === e.target.value);
              if (next) onSelect(next);
            }}
          >
            <option value="" disabled>
              Choose an event
            </option>
            {records.map((r) => (
              <option key={r.id} value={r.id}>
                {new Date(r.createdAt).toLocaleTimeString()} · {r.kind.replaceAll('_', ' ')} ·{' '}
                {voiceActivityText(r).slice(0, 80)}
              </option>
            ))}
          </select>
        </label>
      )}
      {record && (
        <>
          <h3>{record.kind.replaceAll('_', ' ')}</h3>
          <p className="voice-route-text">
            {voiceActivityText(record) || 'This event has no text summary.'}
          </p>
          <time dateTime={record.createdAt}>{new Date(record.createdAt).toLocaleString()}</time>
          {record.route ? (
            <>
              <h3>From</h3>
              <button onClick={() => onInspectNode(record.route!.sourceNodeId)}>
                {name(record.route.sourceNodeId)}
              </button>
              <h3>Delivery</h3>
              {!record.route.destinations.length && <p>No routed recipient recorded.</p>}
              {record.route.destinations.map((destination) => {
                const receipt = record.deliveries?.find(
                  (d) => d.sessionId === destination.sessionId,
                )?.receipt;
                return (
                  <article key={`${destination.nodeId}:${destination.sessionId}`}>
                    <button onClick={() => onInspectNode(destination.nodeId)}>
                      {name(destination.nodeId)}
                    </button>
                    <p>
                      {receipt
                        ? receipt.inputId
                          ? `Input ${receipt.state}`
                          : 'Launch queued'
                        : 'Awaiting delivery receipt'}
                    </p>
                    <button
                      onClick={() =>
                        onOpenSession(destination.sessionId, receipt?.invocationId || undefined)
                      }
                    >
                      {receipt?.invocationId
                        ? 'Open delivered invocation'
                        : 'Open destination session'}
                    </button>
                  </article>
                );
              })}
              <p className="voice-muted">
                Delivery acceptance does not establish completion, speech publication or playback.
              </p>
            </>
          ) : (
            <p>Route evidence unavailable for this event.</p>
          )}
          <details>
            <summary>Source and delivery references</summary>
            <pre>{JSON.stringify(record, null, 2)}</pre>
          </details>
        </>
      )}
    </section>
  );
}
