import type { EntityId } from '../domain/model';
import {
  parseCodexJsonlEvents,
  type CodexItemEvent,
  type CodexJsonlItem,
} from '../infrastructure/codex/jsonlEvents';
import type { CLIInstanceSnapshot, CLIOutputChunk } from './cliInstanceHandler';
import type { AgentSessionRecord, AgentSessionTurnRecord } from './agentSessionStore';

export type AgentSessionDisplayItem =
  | {
      id: EntityId;
      kind: 'user-message';
      text: string;
    }
  | {
      id: EntityId;
      kind: 'processing';
      text: string;
    }
  | {
      id: EntityId;
      kind: 'item';
      itemType: string;
      text: string;
      processing: boolean;
    }
  | {
      id: EntityId;
      kind: 'finished-turn';
      text: string;
      finalText: string;
      expanded: boolean;
      hiddenItems: AgentSessionDisplayItem[];
    }
  | {
      id: EntityId;
      kind: 'agent-message';
      text: string;
    }
  | {
      id: EntityId;
      kind: 'diagnostic';
      text: string;
    };

export interface AgentSessionViewModel {
  sessionId: EntityId | null;
  status: CLIInstanceSnapshot['status'];
  statusLabel: string;
  commandLine: string;
  promptText: string;
  items: AgentSessionDisplayItem[];
  metadata: Record<string, string>;
  contextSize: string;
  startedAt?: string;
  completedAt?: string;
  exitCode?: number;
  error?: string;
  errorOutput: string;
}

export interface AgentSessionOutputFormatterOptions {
  expandedTurnIds?: ReadonlySet<string>;
}

interface TurnAccumulator {
  id: string;
  completed: boolean;
  finalAgentText: string;
  visibleItems: AgentSessionDisplayItem[];
  hiddenItems: AgentSessionDisplayItem[];
}

export class AgentSessionOutputFormatter {
  format(
    snapshot: CLIInstanceSnapshot,
    options: AgentSessionOutputFormatterOptions = {},
  ): AgentSessionViewModel {
    const parsed = parseAgentSessionChunks(snapshot.output, options.expandedTurnIds ?? new Set());
    const promptText = extractPromptText(snapshot.args);
    const items: AgentSessionDisplayItem[] =
      promptText && snapshot.command
        ? [
            {
              id: 'agent-session-user-prompt' as EntityId,
              kind: 'user-message',
              text: promptText,
            },
            ...parsed.items,
          ]
        : parsed.items;

    return {
      sessionId: snapshot.sessionId,
      status: snapshot.status,
      statusLabel: parsed.processing ? 'Processing' : formatStatus(snapshot.status),
      commandLine: formatCommandLine(snapshot.command, snapshot.args),
      promptText,
      items,
      metadata: { ...parsed.metadata, ...(snapshot.metadata ?? {}) },
      contextSize: parsed.contextSize,
      startedAt: snapshot.startedAt,
      completedAt: snapshot.completedAt,
      exitCode: snapshot.exitCode,
      error: snapshot.error,
      errorOutput: formatErrorOutput(snapshot),
    };
  }

  formatRecord(
    record: AgentSessionRecord,
    options: AgentSessionOutputFormatterOptions = {},
  ): AgentSessionViewModel {
    const expandedTurnIds = options.expandedTurnIds ?? new Set();
    const renderedTurns = record.turns.map((turn, index) =>
      formatSessionTurn(turn, index, expandedTurnIds),
    );
    const latestTurn = record.turns.at(-1);
    const latestRenderedTurn = renderedTurns.at(-1);
    const metadata = renderedTurns.reduce<Record<string, string>>(
      (merged, turn) => ({ ...merged, ...turn.metadata }),
      { ...(record.metadata ?? {}) },
    );

    return {
      sessionId: record.id,
      status: record.status,
      statusLabel: latestRenderedTurn?.processing ? 'Processing' : formatStatus(record.status),
      commandLine: latestTurn ? formatCommandLine(latestTurn.command, latestTurn.args) : '',
      promptText: latestTurn?.prompt ?? '',
      items: renderedTurns.flatMap((turn) => turn.items),
      metadata,
      contextSize: latestRenderedTurn?.contextSize ?? 'Context pending',
      startedAt: record.turns[0]?.startedAt,
      completedAt: latestTurn?.completedAt,
      exitCode: latestTurn?.exitCode,
      error: record.error ?? latestTurn?.error,
      errorOutput: formatRecordErrorOutput(record),
    };
  }
}

function formatSessionTurn(
  turn: AgentSessionTurnRecord,
  index: number,
  expandedTurnIds: ReadonlySet<string>,
): {
  items: AgentSessionDisplayItem[];
  metadata: Record<string, string>;
  processing: boolean;
  contextSize: string;
} {
  const parsed = parseAgentSessionChunks(turn.output, expandedTurnIds, (turnIndex) =>
    turnIndex === 1 ? turn.id : (`${turn.id}-${turnIndex}` as EntityId),
  );

  return {
    ...parsed,
    items: [
      {
        id: `${turn.id}-prompt-${index + 1}` as EntityId,
        kind: 'user-message',
        text: turn.prompt,
      },
      ...parsed.items,
    ],
  };
}

function parseAgentSessionChunks(
  chunks: readonly CLIOutputChunk[],
  expandedTurnIds: ReadonlySet<string>,
  turnIdForIndex: (turnIndex: number) => EntityId = (turnIndex) =>
    `turn-${turnIndex}` as EntityId,
): {
  items: AgentSessionDisplayItem[];
  metadata: Record<string, string>;
  processing: boolean;
  contextSize: string;
} {
  const items: AgentSessionDisplayItem[] = [];
  const metadata: Record<string, string> = {};
  let turnIndex = 0;
  let activeTurn: TurnAccumulator | null = null;
  let processing = false;

  chunks.forEach((chunk) => {
    if (chunk.stream === 'system') {
      return;
    }

    if (chunk.stream === 'stderr') {
      Object.assign(metadata, parseAgentSessionMetadata(chunk.content));
      return;
    }

    const events = parseJsonlChunk(chunk.content);

    if (events.length === 0) {
      return;
    }

    events.forEach((event) => {
      if (event.type === 'thread.started') {
        if ('threadId' in event) {
          metadata.codexSessionId = event.threadId;
        }
        return;
      }

      if (event.type === 'turn.started') {
        turnIndex += 1;
        const turnId = turnIdForIndex(turnIndex);
        activeTurn = {
          id: turnId,
          completed: false,
          finalAgentText: '',
          visibleItems: [
            {
              id: `${turnId}-processing` as EntityId,
              kind: 'processing',
              text: 'Processing turn',
            },
          ],
          hiddenItems: [],
        };
        processing = true;
        items.push(...activeTurn.visibleItems);
        return;
      }

      if (event.type === 'turn.completed') {
        if (activeTurn) {
          completeTurn(activeTurn, items, expandedTurnIds);
          activeTurn = null;
        }
        processing = false;
        const usage = 'usage' in event ? event.usage : undefined;
        if (usage) {
          metadata.tokenUsage = JSON.stringify(usage);
        }
        metadata.terminalStatus = 'completed';
        return;
      }

      if (event.type === 'turn.failed') {
        processing = false;
        metadata.terminalStatus = 'failed';
        if (activeTurn) {
          completeTurn(activeTurn, items, expandedTurnIds);
          activeTurn = null;
        }
        return;
      }

      if (event.type === 'error') {
        processing = false;
        metadata.terminalStatus = 'error';
        return;
      }

      if (event.type.startsWith('item.') && 'item' in event) {
        const itemDisplay = formatItemEvent(event, chunk.id);
        if (!activeTurn) {
          if (event.type === 'item.completed' && event.item.type === 'agent_message') {
            items.push({
              id: itemDisplay.id,
              kind: 'agent-message',
              text: agentMessageText(event.item) ?? itemDisplay.text,
            });
            return;
          }

          items.push(itemDisplay);
          return;
        }

        if (event.type === 'item.completed' && event.item.type === 'agent_message') {
          const text = agentMessageText(event.item) ?? itemDisplay.text;
          activeTurn.finalAgentText = text;
          items.push({ id: itemDisplay.id, kind: 'agent-message', text });
          return;
        }

        upsertTurnHiddenItem(activeTurn, event, itemDisplay);
        items.push(itemDisplay);
      }
    });
  });

  return { items, metadata, processing, contextSize: formatContextSize(metadata.tokenUsage) };
}

function completeTurn(
  turn: TurnAccumulator,
  items: AgentSessionDisplayItem[],
  expandedTurnIds: ReadonlySet<string>,
): void {
  const expanded = expandedTurnIds.has(turn.id);
  const startIndex = items.findIndex(
    (item) => item.kind === 'processing' && item.id === turn.visibleItems[0]?.id,
  );
  const finishedTurn: AgentSessionDisplayItem = {
    id: turn.id as EntityId,
    kind: 'finished-turn',
    text: 'Finished turn',
    finalText: turn.finalAgentText,
    expanded,
    hiddenItems: turn.hiddenItems,
  };
  const replacement = [finishedTurn];

  if (startIndex >= 0) {
    items.splice(startIndex, items.length - startIndex, ...replacement);
  } else {
    items.push(...replacement);
  }
}

function formatItemEvent(event: CodexItemEvent, chunkId: EntityId): AgentSessionDisplayItem {
  const processing = event.type !== 'item.completed';
  const itemId = stringField(event.item.raw.id);
  return {
    id: (itemId ? `agent-item-${itemId}` : `${chunkId}-${event.lineNumber}`) as EntityId,
    kind: 'item',
    itemType: event.item.type,
    text: formatItemText(event.item),
    processing,
  };
}

function upsertTurnHiddenItem(
  turn: TurnAccumulator,
  event: CodexItemEvent,
  itemDisplay: AgentSessionDisplayItem,
): void {
  const itemId = stringField(event.item.raw.id);

  if (!itemId || event.type !== 'item.completed') {
    turn.hiddenItems.push(itemDisplay);
    return;
  }

  const existingIndex = turn.hiddenItems.findIndex(
    (item) => item.kind === 'item' && item.id === (`agent-item-${itemId}` as EntityId),
  );
  const completedItem = { ...itemDisplay, id: `agent-item-${itemId}` as EntityId };

  if (existingIndex >= 0) {
    turn.hiddenItems.splice(existingIndex, 1, completedItem);
  } else {
    turn.hiddenItems.push(completedItem);
  }
}

function formatItemText(item: CodexJsonlItem): string {
  if (item.type === 'agent_message') {
    return agentMessageText(item) ?? 'Agent message';
  }

  if (item.type === 'web_search') {
    const query = stringField(item.raw.query) ?? stringField(item.raw.action, 'query');
    return query ? `Web search: ${query}` : 'Web search';
  }

  if (item.type === 'command_execution') {
    return stringField(item.raw.command) ?? 'Command execution';
  }

  if (item.type === 'reasoning') {
    return 'Reasoning';
  }

  if (item.type === 'mcp_tool_call') {
    return stringField(item.raw.name) ?? 'Tool call';
  }

  return item.type;
}

function agentMessageText(item: CodexJsonlItem): string | undefined {
  return item.type === 'agent_message' && 'text' in item && typeof item.text === 'string'
    ? item.text
    : undefined;
}

function parseJsonlChunk(content: string): ReturnType<typeof parseCodexJsonlEvents> {
  try {
    return parseCodexJsonlEvents(content);
  } catch {
    return [];
  }
}

function parseAgentSessionMetadata(stderr: string): Record<string, string> {
  const metadata: Record<string, string> = {};
  const fieldMap: Record<string, string> = {
    model: 'model',
    approval: 'approval',
    sandbox: 'sandbox',
    'reasoning effort': 'reasoningEffort',
    'reasoning summaries': 'reasoningSummaries',
    workdir: 'workdir',
    provider: 'provider',
  };

  stderr.split(/\r?\n/).forEach((line) => {
    const match = line.trim().match(/^([^:]+):\s*(.+)$/);
    if (!match) {
      return;
    }

    const key = fieldMap[match[1].trim().toLowerCase()];
    if (key) {
      metadata[key] = match[2].trim();
    }
  });

  return metadata;
}

function stringField(value: unknown, nestedKey?: string): string | undefined {
  if (nestedKey && value && typeof value === 'object' && !Array.isArray(value)) {
    const nested = (value as Record<string, unknown>)[nestedKey];
    return typeof nested === 'string' ? nested : undefined;
  }

  return typeof value === 'string' ? value : undefined;
}

function formatStatus(status: CLIInstanceSnapshot['status']): string {
  switch (status) {
    case 'idle':
      return 'Idle';
    case 'running':
      return 'Running';
    case 'completed':
      return 'Completed';
    case 'failed':
      return 'Failed';
    case 'closed':
      return 'Closed';
  }
}

function formatCommandLine(command: string | null, args: readonly string[]): string {
  if (!command) {
    return '';
  }

  return [command, ...args.map(quoteArg)].join(' ');
}

function quoteArg(arg: string): string {
  return /\s/.test(arg) ? `"${arg.replaceAll('"', '\\"')}"` : arg;
}

function extractPromptText(args: readonly string[]): string {
  return args.at(-1) ?? '';
}

function formatErrorOutput(snapshot: CLIInstanceSnapshot): string {
  if (!snapshot.error && snapshot.status !== 'failed') {
    return '';
  }

  return [
    snapshot.error,
    ...snapshot.output
      .filter((chunk) => chunk.stream === 'stderr')
      .map((chunk) => chunk.content)
      .filter((content) => content.trim().length > 0),
  ]
    .filter((content): content is string => Boolean(content?.trim()))
    .join('\n');
}

function formatRecordErrorOutput(record: AgentSessionRecord): string {
  return [
    record.error,
    ...record.turns.flatMap((turn) => [
      turn.error,
      ...turn.output
        .filter((chunk) => chunk.stream === 'stderr')
        .map((chunk) => chunk.content)
        .filter((content) => content.trim().length > 0),
    ]),
  ]
    .filter((content): content is string => Boolean(content?.trim()))
    .join('\n');
}

function formatContextSize(tokenUsage: string | undefined): string {
  if (!tokenUsage) {
    return 'Context pending';
  }

  try {
    const parsed = JSON.parse(tokenUsage) as Record<string, unknown>;
    const inputTokens = numberField(parsed.input_tokens) ?? numberField(parsed.inputTokens);
    const cachedTokens =
      numberField(parsed.cached_input_tokens) ?? numberField(parsed.cachedInputTokens);

    if (inputTokens === undefined) {
      return 'Context available';
    }

    return cachedTokens === undefined
      ? `${inputTokens.toLocaleString()} tokens`
      : `${inputTokens.toLocaleString()} tokens, ${cachedTokens.toLocaleString()} cached`;
  } catch {
    return 'Context available';
  }
}

function numberField(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}
