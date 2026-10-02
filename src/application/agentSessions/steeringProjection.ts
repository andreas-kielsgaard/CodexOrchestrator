import type { AgentRuntimeEventDto, SessionInteractionDto } from './contracts';
import { runtimeSteeringRecord } from './runtimeControlRecords';

export function applySteeringEvent(
  interactions: readonly SessionInteractionDto[] = [],
  event: AgentRuntimeEventDto,
): SessionInteractionDto[] | null {
  const raw = runtimeSteeringRecord(event);
  if (!raw) return null;
  const kind = raw.kind;
  if (kind === 'session_steering_result') {
    return interactions.map((i) =>
      i.invocationId === event.invocationId && i.id === raw.id
        ? {
            ...i,
            state: raw.state as SessionInteractionDto['state'],
            result: typeof raw.message === 'string' ? raw.message : null,
          }
        : i,
    );
  }
  const interaction: SessionInteractionDto = {
    id: String(raw.inputId),
    invocationId: event.invocationId,
    sequence: event.sequence,
    recordedAt: event.recordedAt,
    kind: 'steering',
    state: kind === 'session_steering_queued' ? 'queued' : 'pending',
    content: {
      text: String(raw.text ?? ''),
      ...(Array.isArray(raw.attachments)
        ? {
            attachments: raw.attachments as NonNullable<
              SessionInteractionDto['content']['attachments']
            >,
          }
        : {}),
    },
    result: null,
  };
  return [
    ...interactions.filter(
      (i) => !(i.id === interaction.id && i.invocationId === interaction.invocationId),
    ),
    interaction,
  ];
}
