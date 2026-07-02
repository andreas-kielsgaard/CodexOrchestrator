export type JsonObject = Record<string, unknown>;

export type CodexJsonlEvent =
  | CodexThreadStartedEvent
  | CodexTurnStartedEvent
  | CodexTurnCompletedEvent
  | CodexTurnFailedEvent
  | CodexItemEvent
  | CodexErrorEvent
  | CodexUnknownEvent;

export interface CodexEventBase {
  readonly type: string;
  readonly lineNumber: number;
  readonly raw: JsonObject;
}

export interface CodexThreadStartedEvent extends CodexEventBase {
  readonly type: 'thread.started';
  readonly threadId: string;
}

export interface CodexTurnStartedEvent extends CodexEventBase {
  readonly type: 'turn.started';
}

export interface CodexTurnCompletedEvent extends CodexEventBase {
  readonly type: 'turn.completed';
  readonly usage?: JsonObject;
}

export interface CodexTurnFailedEvent extends CodexEventBase {
  readonly type: 'turn.failed';
}

export interface CodexErrorEvent extends CodexEventBase {
  readonly type: 'error';
}

export interface CodexItemEvent extends CodexEventBase {
  readonly type: `item.${string}`;
  readonly item: CodexJsonlItem;
}

export interface CodexUnknownEvent extends CodexEventBase {
  readonly type: string;
  readonly known: false;
}

export type CodexJsonlItem =
  | CodexAgentMessageItem
  | CodexReasoningItem
  | CodexCommandExecutionItem
  | CodexFileChangeItem
  | CodexMcpToolCallItem
  | CodexWebSearchItem
  | CodexPlanUpdateItem
  | CodexUnknownItem;

export interface CodexItemBase {
  readonly type: string;
  readonly raw: JsonObject;
}

export interface CodexAgentMessageItem extends CodexItemBase {
  readonly type: 'agent_message';
  readonly text?: string;
}

export interface CodexReasoningItem extends CodexItemBase {
  readonly type: 'reasoning';
}

export interface CodexCommandExecutionItem extends CodexItemBase {
  readonly type: 'command_execution';
}

export interface CodexFileChangeItem extends CodexItemBase {
  readonly type: 'file_change';
}

export interface CodexMcpToolCallItem extends CodexItemBase {
  readonly type: 'mcp_tool_call';
}

export interface CodexWebSearchItem extends CodexItemBase {
  readonly type: 'web_search';
}

export interface CodexPlanUpdateItem extends CodexItemBase {
  readonly type: 'plan_update';
}

export interface CodexUnknownItem extends CodexItemBase {
  readonly type: string;
  readonly known: false;
}

export type CodexJsonlTerminalStatus =
  | { readonly kind: 'completed'; readonly lineNumber: number }
  | { readonly kind: 'failed'; readonly lineNumber: number }
  | { readonly kind: 'error'; readonly lineNumber: number };

export interface CodexJsonlEventSummary {
  readonly threadId?: string;
  readonly finalAgentMessageText?: string;
  readonly terminalStatus?: CodexJsonlTerminalStatus;
  readonly tokenUsage?: JsonObject;
  readonly itemCountsByType: Record<string, number>;
}

export class CodexJsonlParseError extends Error {
  constructor(
    readonly lineNumber: number,
    message: string,
  ) {
    super(`Line ${lineNumber}: ${message}`);
    this.name = 'CodexJsonlParseError';
  }
}

export function parseCodexJsonlEvents(jsonl: string): CodexJsonlEvent[] {
  const events: CodexJsonlEvent[] = [];
  const lines = jsonl.split(/\r\n|\n|\r/);

  lines.forEach((line, index) => {
    const lineNumber = index + 1;

    if (line.trim() === '') {
      return;
    }

    events.push(parseCodexJsonlEventLine(line, lineNumber));
  });

  return events;
}

export function summarizeCodexJsonlEvents(
  events: readonly CodexJsonlEvent[],
): CodexJsonlEventSummary {
  const itemCountsByType: Record<string, number> = {};
  let threadId: string | undefined;
  let finalAgentMessageText: string | undefined;
  let terminalStatus: CodexJsonlTerminalStatus | undefined;
  let tokenUsage: JsonObject | undefined;

  for (const event of events) {
    if (isCodexThreadStartedEvent(event)) {
      threadId = event.threadId;
      continue;
    }

    if (isCodexTurnCompletedEvent(event)) {
      terminalStatus = { kind: 'completed', lineNumber: event.lineNumber };
      tokenUsage = event.usage;
      continue;
    }

    if (isCodexTurnFailedEvent(event)) {
      terminalStatus = { kind: 'failed', lineNumber: event.lineNumber };
      continue;
    }

    if (isCodexErrorEvent(event)) {
      terminalStatus = { kind: 'error', lineNumber: event.lineNumber };
      continue;
    }

    if (isCodexItemEvent(event)) {
      itemCountsByType[event.item.type] = (itemCountsByType[event.item.type] ?? 0) + 1;

      if (
        event.type === 'item.completed' &&
        isCodexAgentMessageItem(event.item) &&
        event.item.text !== undefined
      ) {
        finalAgentMessageText = event.item.text;
      }
    }
  }

  return {
    ...(threadId === undefined ? {} : { threadId }),
    ...(finalAgentMessageText === undefined ? {} : { finalAgentMessageText }),
    ...(terminalStatus === undefined ? {} : { terminalStatus }),
    ...(tokenUsage === undefined ? {} : { tokenUsage }),
    itemCountsByType,
  };
}

function parseCodexJsonlEventLine(line: string, lineNumber: number): CodexJsonlEvent {
  let parsed: unknown;

  try {
    parsed = JSON.parse(line);
  } catch (error) {
    throw new CodexJsonlParseError(lineNumber, `Invalid JSON: ${(error as Error).message}`);
  }

  if (!isJsonObject(parsed)) {
    throw new CodexJsonlParseError(lineNumber, 'Event line must be a JSON object');
  }

  const type = parsed.type;

  if (type === undefined) {
    throw new CodexJsonlParseError(lineNumber, 'Event type is required');
  }

  if (typeof type !== 'string') {
    throw new CodexJsonlParseError(lineNumber, 'Event type must be a string');
  }

  return normalizeCodexJsonlEvent(parsed, type, lineNumber);
}

function normalizeCodexJsonlEvent(
  raw: JsonObject,
  type: string,
  lineNumber: number,
): CodexJsonlEvent {
  if (type === 'thread.started') {
    const threadId = raw.thread_id;

    if (typeof threadId !== 'string' || threadId.length === 0) {
      throw new CodexJsonlParseError(lineNumber, 'thread.started thread_id must be a string');
    }

    return { type, lineNumber, raw, threadId };
  }

  if (type === 'turn.started') {
    return { type, lineNumber, raw };
  }

  if (type === 'turn.completed') {
    const usage = optionalJsonObject(raw.usage, lineNumber, 'turn.completed usage');

    return {
      type,
      lineNumber,
      raw,
      ...(usage === undefined ? {} : { usage }),
    };
  }

  if (type === 'turn.failed') {
    return { type, lineNumber, raw };
  }

  if (type === 'error') {
    return { type, lineNumber, raw };
  }

  if (type.startsWith('item.')) {
    const item = raw.item;

    if (!isJsonObject(item)) {
      throw new CodexJsonlParseError(lineNumber, `${type} item must be a JSON object`);
    }

    return {
      type: type as `item.${string}`,
      lineNumber,
      raw,
      item: normalizeCodexJsonlItem(item, lineNumber),
    };
  }

  return { type, lineNumber, raw, known: false };
}

function normalizeCodexJsonlItem(raw: JsonObject, lineNumber: number): CodexJsonlItem {
  const type = raw.type;

  if (type === undefined) {
    throw new CodexJsonlParseError(lineNumber, 'Item type is required');
  }

  if (typeof type !== 'string') {
    throw new CodexJsonlParseError(lineNumber, 'Item type must be a string');
  }

  if (type === 'agent_message') {
    const text = raw.text;

    if (text !== undefined && typeof text !== 'string') {
      throw new CodexJsonlParseError(lineNumber, 'agent_message text must be a string');
    }

    return {
      type,
      raw,
      ...(text === undefined ? {} : { text }),
    };
  }

  if (type === 'reasoning') {
    return { type, raw };
  }

  if (type === 'command_execution') {
    return { type, raw };
  }

  if (type === 'file_change') {
    return { type, raw };
  }

  if (type === 'mcp_tool_call') {
    return { type, raw };
  }

  if (type === 'web_search') {
    return { type, raw };
  }

  if (type === 'plan_update') {
    return { type, raw };
  }

  return { type, raw, known: false };
}

function optionalJsonObject(
  value: unknown,
  lineNumber: number,
  label: string,
): JsonObject | undefined {
  if (value === undefined) {
    return undefined;
  }

  if (!isJsonObject(value)) {
    throw new CodexJsonlParseError(lineNumber, `${label} must be a JSON object`);
  }

  return value;
}

function isCodexItemEvent(event: CodexJsonlEvent): event is CodexItemEvent {
  return event.type.startsWith('item.');
}

function isCodexThreadStartedEvent(event: CodexJsonlEvent): event is CodexThreadStartedEvent {
  return event.type === 'thread.started' && !isUnknownCodexEvent(event);
}

function isCodexTurnCompletedEvent(event: CodexJsonlEvent): event is CodexTurnCompletedEvent {
  return event.type === 'turn.completed' && !isUnknownCodexEvent(event);
}

function isCodexTurnFailedEvent(event: CodexJsonlEvent): event is CodexTurnFailedEvent {
  return event.type === 'turn.failed' && !isUnknownCodexEvent(event);
}

function isCodexErrorEvent(event: CodexJsonlEvent): event is CodexErrorEvent {
  return event.type === 'error' && !isUnknownCodexEvent(event);
}

function isCodexAgentMessageItem(item: CodexJsonlItem): item is CodexAgentMessageItem {
  return item.type === 'agent_message' && !isUnknownCodexItem(item);
}

function isUnknownCodexEvent(event: CodexJsonlEvent): event is CodexUnknownEvent {
  return 'known' in event && event.known === false;
}

function isUnknownCodexItem(item: CodexJsonlItem): item is CodexUnknownItem {
  return 'known' in item && item.known === false;
}

function isJsonObject(value: unknown): value is JsonObject {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
