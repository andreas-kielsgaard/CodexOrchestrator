import type { JsonObject } from './jsonlEvents';

export type CodexAppServerMessage =
  CodexAppServerResponseMessage | CodexAppServerNotificationMessage | CodexAppServerUnknownMessage;

export interface CodexAppServerMessageBase {
  readonly lineNumber: number;
  readonly raw: JsonObject;
}

export interface CodexAppServerResponseMessage extends CodexAppServerMessageBase {
  readonly kind: 'response';
  readonly id: string | number;
  readonly result?: JsonObject;
  readonly error?: JsonObject;
}

export interface CodexAppServerNotificationMessage extends CodexAppServerMessageBase {
  readonly kind: 'notification';
  readonly method: string;
  readonly params?: JsonObject;
}

export interface CodexAppServerUnknownMessage extends CodexAppServerMessageBase {
  readonly kind: 'unknown';
}

export interface CodexAppServerMessageSummary {
  readonly threadId?: string;
  readonly turnId?: string;
  readonly terminalTurnStatus?: JsonObject;
  readonly responseCount: number;
  readonly errorResponseCount: number;
  readonly notificationCountsByMethod: Record<string, number>;
  readonly tokenUsageUpdates: JsonObject[];
}

export class CodexAppServerMessageParseError extends Error {
  constructor(
    readonly lineNumber: number,
    message: string,
  ) {
    super(`Line ${lineNumber}: ${message}`);
    this.name = 'CodexAppServerMessageParseError';
  }
}

export function parseCodexAppServerMessages(jsonl: string): CodexAppServerMessage[] {
  const messages: CodexAppServerMessage[] = [];
  const lines = jsonl.split(/\r\n|\n|\r/);

  lines.forEach((line, index) => {
    const lineNumber = index + 1;

    if (line.trim() === '') {
      return;
    }

    messages.push(parseCodexAppServerMessageLine(line, lineNumber));
  });

  return messages;
}

export function summarizeCodexAppServerMessages(
  messages: readonly CodexAppServerMessage[],
): CodexAppServerMessageSummary {
  const notificationCountsByMethod: Record<string, number> = {};
  const tokenUsageUpdates: JsonObject[] = [];
  let responseCount = 0;
  let errorResponseCount = 0;
  let threadId: string | undefined;
  let turnId: string | undefined;
  let terminalTurnStatus: JsonObject | undefined;

  for (const message of messages) {
    if (message.kind === 'response') {
      responseCount += 1;

      if (message.error !== undefined) {
        errorResponseCount += 1;
      }

      threadId ??= extractNestedString(message.result, ['thread', 'id']);
      continue;
    }

    if (message.kind === 'notification') {
      notificationCountsByMethod[message.method] =
        (notificationCountsByMethod[message.method] ?? 0) + 1;

      threadId ??=
        extractNestedString(message.params, ['thread', 'id']) ??
        extractNestedString(message.params, ['threadId']);
      turnId ??=
        extractNestedString(message.params, ['turn', 'id']) ??
        extractNestedString(message.params, ['turnId']);

      if (message.method === 'turn/completed') {
        terminalTurnStatus = message.params;
      }

      if (message.method === 'thread/tokenUsage/updated' && message.params !== undefined) {
        tokenUsageUpdates.push(message.params);
      }
    }
  }

  return {
    ...(threadId === undefined ? {} : { threadId }),
    ...(turnId === undefined ? {} : { turnId }),
    ...(terminalTurnStatus === undefined ? {} : { terminalTurnStatus }),
    responseCount,
    errorResponseCount,
    notificationCountsByMethod,
    tokenUsageUpdates,
  };
}

function parseCodexAppServerMessageLine(line: string, lineNumber: number): CodexAppServerMessage {
  let parsed: unknown;

  try {
    parsed = JSON.parse(line);
  } catch (error) {
    throw new CodexAppServerMessageParseError(
      lineNumber,
      `Invalid JSON: ${(error as Error).message}`,
    );
  }

  if (!isJsonObject(parsed)) {
    throw new CodexAppServerMessageParseError(lineNumber, 'Message line must be a JSON object');
  }

  return normalizeCodexAppServerMessage(parsed, lineNumber);
}

function normalizeCodexAppServerMessage(
  raw: JsonObject,
  lineNumber: number,
): CodexAppServerMessage {
  const id = raw.id;
  const method = raw.method;

  if (id !== undefined) {
    if (typeof id !== 'string' && typeof id !== 'number') {
      throw new CodexAppServerMessageParseError(
        lineNumber,
        'Response id must be a string or number',
      );
    }

    return {
      kind: 'response',
      lineNumber,
      raw,
      id,
      ...optionalMessagePayload(raw, lineNumber),
    };
  }

  if (method !== undefined) {
    if (typeof method !== 'string') {
      throw new CodexAppServerMessageParseError(lineNumber, 'Notification method must be a string');
    }

    const params = optionalJsonObject(raw.params, lineNumber, 'Notification params');

    return {
      kind: 'notification',
      lineNumber,
      raw,
      method,
      ...(params === undefined ? {} : { params }),
    };
  }

  return { kind: 'unknown', lineNumber, raw };
}

function optionalMessagePayload(
  raw: JsonObject,
  lineNumber: number,
): Pick<CodexAppServerResponseMessage, 'result' | 'error'> {
  const result = optionalJsonObject(raw.result, lineNumber, 'Response result');
  const error = optionalJsonObject(raw.error, lineNumber, 'Response error');

  return {
    ...(result === undefined ? {} : { result }),
    ...(error === undefined ? {} : { error }),
  };
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
    throw new CodexAppServerMessageParseError(lineNumber, `${label} must be a JSON object`);
  }

  return value;
}

function extractNestedString(value: unknown, path: readonly string[]): string | undefined {
  let current = value;

  for (const segment of path) {
    if (!isJsonObject(current)) {
      return undefined;
    }

    current = current[segment];
  }

  return typeof current === 'string' && current.length > 0 ? current : undefined;
}

function isJsonObject(value: unknown): value is JsonObject {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
