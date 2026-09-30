import type { AgentSessionDetailsDto } from '../../application/agentSessions';
import type { WorkspaceTransitionReceiptDto } from '../../application/agentSessions/sessionHistory';
import type {
  SessionQueueViewDto,
  SessionTargetOperationDto,
} from '../../application/agentSessions/sessionQueue';
import type { ProjectedTranscript } from './transcriptProjector';
import { agentProviderLabel } from '../../application/agentProviders';

export type SessionTransition =
  | { kind: 'workspace'; id: string; receipt: WorkspaceTransitionReceiptDto }
  | { kind: 'provider'; id: string; previous: string; provider: string; invocationId: string }
  | { kind: 'blocked'; id: string; operation: SessionTargetOperationDto };
export type SessionHistoryItem =
  | {
      kind: 'invocation';
      id: string;
      providerChange?: Extract<SessionTransition, { kind: 'provider' }>;
    }
  | Exclude<SessionTransition, { kind: 'provider' }>;

export function transitionLabel(transition: SessionTransition): string {
  if (transition.kind === 'provider') return `${agentProviderLabel(transition.provider)}:`;
  if (transition.kind === 'blocked')
    return `Target switch blocked: ${transition.operation.error ?? 'Choose a resolution'}`;
  const { destination, intent, source } = transition.receipt;
  if (
    source &&
    source.execution.deviceId === destination.execution.deviceId &&
    source.worktreeId === destination.worktreeId &&
    source.path === destination.path &&
    source.repositoryId === destination.repositoryId &&
    source.branchRef === destination.branchRef
  )
    return `Prepared workspace on ${destination.execution.deviceName}`;
  return intent === 'continue_work'
    ? `Continuing work on ${destination.execution.deviceName}`
    : `Switching target to ${destination.execution.deviceName} ${destination.branchRef.replace(/^refs\/heads\//, '')}`.trim();
}

export function projectSessionHistory(
  details: AgentSessionDetailsDto | null,
  transcript: ProjectedTranscript | null,
  queue?: SessionQueueViewDto | null,
): readonly SessionHistoryItem[] {
  if (!details || !transcript) return [];
  const timeline = details.timeline;
  const positions = new Map(
    timeline?.positions.map((position) => [
      position.invocationId ?? position.operationId,
      position.ordinal,
    ]),
  );
  const providers = new Map(
    timeline?.providers.map((value) => [value.invocationId, value.provider]),
  );
  const items: { item: SessionHistoryItem; position: number }[] = transcript.invocations.map(
    (invocation, index) => ({
      item: { kind: 'invocation', id: invocation.id },
      position: positions.get(invocation.id) ?? Number.MAX_SAFE_INTEGER / 2 + index,
    }),
  );
  for (const receipt of timeline?.transitions ?? [])
    items.push({
      item: { kind: 'workspace', id: receipt.operationId, receipt },
      position: positions.get(receipt.operationId) ?? Number.MAX_SAFE_INTEGER,
    });
  items.sort((a, b) => a.position - b.position);
  let previous: string | null = null;
  for (const { item } of items) {
    if (item.kind !== 'invocation') continue;
    const provider = providers.get(item.id);
    if (provider && previous && previous !== provider)
      item.providerChange = {
        kind: 'provider',
        id: `provider:${item.id}`,
        previous,
        provider,
        invocationId: item.id,
      };
    previous = provider ?? null;
  }
  const head = queue?.entries.find(
    (item) => item.entry.state !== 'completed' && item.entry.state !== 'canceled',
  );
  if (head?.targetOperation && head.entry.state === 'blocked')
    items.push({
      item: {
        kind: 'blocked',
        id: `blocked:${head.targetOperation.id}`,
        operation: head.targetOperation,
      },
      position: Number.MAX_SAFE_INTEGER,
    });
  return items.map((value) => value.item);
}
