import { toolActivityLabel, type TranscriptActivity } from './transcriptProjector';

export interface ToolActivityDetailRow {
  readonly label: string;
  readonly value: string;
  readonly code: boolean;
}

export interface ToolActivityPresentation {
  readonly icon: string;
  readonly label: string;
  readonly failed: boolean;
  readonly batchLabel: string | null;
  readonly details: readonly ToolActivityDetailRow[];
}

export function presentToolActivity(
  item: TranscriptActivity,
  safeOnly: boolean,
): ToolActivityPresentation {
  const safe = item.safeDetail?.kind === 'tool' ? item.safeDetail : null;
  const failed = safe?.resultClassification === 'failed';
  if (safeOnly) {
    const label = safe
      ? [
          [safe.server, safe.tool].filter(Boolean).join(' / ') || toolActivityLabel(safe.activity),
          safe.phase,
          safe.resultClassification,
        ].join(' · ')
      : 'Tool activity detail unavailable';
    return { icon: '◇', label, failed, batchLabel: null, details: [] };
  }

  const evidence = toolEvidence(item);
  const text = item.text.trim();
  const status = firstString(evidence.completed, ['status']) ?? safe?.status;
  const duration = activityDuration(item);
  const details: ToolActivityDetailRow[] = [];
  const add = (label: string, value: unknown, code = false) => {
    const formatted = displayValue(value);
    if (formatted) details.push({ label, value: formatted, code });
  };

  switch (safe?.activity) {
    case 'command': {
      const commands = commandInputs(evidence, text);
      add(commands.length > 1 ? 'Commands' : 'Command', commands.join('\n'), true);
      add('Standard output', outputValue(evidence, 'stdout'), true);
      add('Standard error', outputValue(evidence, 'stderr'), true);
      add('Output', outputValue(evidence, 'output'), true);
      add('Exit code', firstValue(evidence.completed, ['exit_code', 'exitCode']));
      add('Status', status);
      add('Runtime', duration);
      const isBatch = commands.length > 1;
      return {
        icon: '>_',
        label: isBatch
          ? `Ran ${commands.length} commands`
          : commands[0]
            ? `Ran ${truncate(commands[0].replace(/\s+/g, ' '), 96)}`
            : 'Ran a command',
        failed,
        batchLabel: isBatch ? `Batch · ${commands.length} commands` : evidence.batchLabel,
        details: deduplicate(details),
      };
    }
    case 'file_change': {
      const files = fileChanges(item, evidence);
      if (files.length) add('Files', files.join('\n'), true);
      add('Status', status);
      add('Runtime', duration);
      return {
        icon: '✎',
        label: files.length
          ? `Edited ${files.length} ${files.length === 1 ? 'file' : 'files'}`
          : usefulToolText(text)
            ? `Edited ${truncate(text.replace(/\s+/g, ' '), 88)}`
            : 'Edited files',
        failed,
        batchLabel: evidence.batchLabel,
        details: deduplicate(details),
      };
    }
    case 'web_search':
      add('Query', evidence.input ?? (usefulToolText(text) ? text : null), true);
      add('Result', evidence.result, true);
      add('Status', status);
      add('Runtime', duration);
      return {
        icon: '⌕',
        label: usefulToolText(text)
          ? `Searched the web for ${truncate(text, 88)}`
          : 'Searched the web',
        failed,
        batchLabel: evidence.batchLabel,
        details: deduplicate(details),
      };
    case 'plan':
      add('Update', evidence.input ?? (usefulToolText(text) ? text : null), true);
      add('Status', status);
      return {
        icon: '☷',
        label: 'Updated the plan',
        failed,
        batchLabel: evidence.batchLabel,
        details: deduplicate(details),
      };
    case 'mcp_tool': {
      const name = [safe.server, safe.tool].filter(Boolean).join(' / ');
      add('Input', evidence.input, true);
      add('Result', evidence.result, true);
      add('Status', status);
      add('Runtime', duration);
      return {
        icon: '◇',
        label: name ? `Used ${name}` : 'Used a tool',
        failed,
        batchLabel: evidence.batchLabel,
        details: deduplicate(details),
      };
    }
    default:
      add('Input', evidence.input, true);
      add('Result', evidence.result, true);
      add('Status', status);
      add('Runtime', duration);
      return {
        icon: '◇',
        label: usefulToolText(text) ? truncate(text, 96) : 'Used a tool',
        failed,
        batchLabel: evidence.batchLabel,
        details: deduplicate(details),
      };
  }
}

interface ToolEvidence {
  readonly started: JsonRecord[];
  readonly completed: JsonRecord[];
  readonly input: unknown;
  readonly result: unknown;
  readonly batchLabel: string | null;
}

type JsonRecord = Record<string, unknown>;

function toolEvidence(item: TranscriptActivity): ToolEvidence {
  const lifecycle = asRecord(item.rawPayload)?.lifecycleEvents;
  const payloads = Array.isArray(lifecycle) ? lifecycle : [item.rawPayload];
  const records = payloads.map(asRecord).filter((value): value is JsonRecord => Boolean(value));
  const itemId = item.safeDetail?.kind === 'tool' ? item.safeDetail.itemId : null;
  const started = records
    .map((record) => toolRecord(record, itemId))
    .filter((value): value is JsonRecord => Boolean(value));
  const completed = [...started].reverse();
  const input = firstDefined(
    started.map((record) =>
      firstValue([record], ['arguments', 'input', 'command', 'commands', 'query']),
    ),
  );
  const result = firstDefined(
    completed.map((record) => firstValue([record], ['result', 'tool_use_result', 'toolUseResult'])),
  );
  return {
    started,
    completed,
    input,
    result,
    batchLabel: batchLabel(records, started, input),
  };
}

function toolRecord(payload: JsonRecord, itemId: string | null): JsonRecord | null {
  const item = asRecord(payload.item);
  if (item) return item;
  const blocks = asRecord(payload.message)?.content;
  if (Array.isArray(blocks)) {
    const tool = blocks
      .map(asRecord)
      .find(
        (block) =>
          block?.type === 'tool_use' &&
          (!itemId || block.id === itemId || block.tool_use_id === itemId),
      );
    if (tool) return { ...tool, tool_use_result: payload.tool_use_result };
  }
  return payload;
}

function batchLabel(
  records: JsonRecord[],
  toolRecords: JsonRecord[],
  input: unknown,
): string | null {
  for (const payload of records) {
    const blocks = asRecord(payload.message)?.content;
    if (Array.isArray(blocks)) {
      const count = blocks.filter((block) => asRecord(block)?.type === 'tool_use').length;
      if (count > 1) return `Batch · ${count} calls`;
    }
  }
  for (const record of toolRecords) {
    for (const key of ['commands', 'calls', 'requests']) {
      const values = record[key];
      if (Array.isArray(values) && values.length > 1) return `Batch · ${values.length} calls`;
    }
  }
  const code = asRecord(input)?.code;
  if (typeof code === 'string' && /Promise\.(?:all|allSettled)\s*\(/.test(code)) {
    const count = [...code.matchAll(/\btools\.[A-Za-z0-9_]+\s*\(/g)].length;
    return count > 1 ? `Batch · ${count} calls` : 'Batch';
  }
  return null;
}

function commandInputs(evidence: ToolEvidence, fallback: string): string[] {
  for (const record of evidence.started) {
    const commands = record.commands;
    if (Array.isArray(commands)) {
      const values = commands.map(displayValue).filter(Boolean);
      if (values.length) return values;
    }
    if (typeof record.command === 'string') return [unwrapShellCommand(record.command)];
  }
  if (typeof evidence.input === 'string') return [unwrapShellCommand(evidence.input)];
  return usefulToolText(fallback) ? [unwrapShellCommand(fallback)] : [];
}

function outputValue(evidence: ToolEvidence, kind: 'stdout' | 'stderr' | 'output'): unknown {
  const keys = kind === 'output' ? ['aggregated_output', 'aggregatedOutput', 'output'] : [kind];
  return firstValue(evidence.completed, keys) ?? firstValueFromObject(evidence.result, keys);
}

function fileChanges(item: TranscriptActivity, evidence: ToolEvidence): string[] {
  const normalized = asRecord(item.normalizedDetails)?.fileChanges;
  const raw = firstDefined(evidence.completed.map((record) => record.changes));
  const changes = Array.isArray(normalized) ? normalized : Array.isArray(raw) ? raw : [];
  return changes
    .map(asRecord)
    .filter((value): value is JsonRecord => Boolean(value))
    .map((change) => {
      const path = firstString([change], ['path', 'file_path', 'filePath']);
      const operation = firstString([change], ['operation', 'kind']);
      if (!path) return '';
      return operation ? `${path} — ${capitalize(operation)}` : path;
    })
    .filter(Boolean);
}

function activityDuration(item: TranscriptActivity): string | null {
  if (!item.completedAt) return null;
  const elapsed = Date.parse(item.completedAt) - Date.parse(item.recordedAt);
  if (!Number.isFinite(elapsed) || elapsed < 0) return null;
  if (elapsed < 1_000) return `${elapsed} ms`;
  if (elapsed < 60_000) return `${(elapsed / 1_000).toFixed(elapsed < 10_000 ? 1 : 0)} s`;
  const minutes = Math.floor(elapsed / 60_000);
  const seconds = Math.floor((elapsed % 60_000) / 1_000);
  return `${minutes}m ${seconds}s`;
}

function unwrapShellCommand(value: string): string {
  const trimmed = value.trim();
  const match = trimmed.match(
    /^(?:"[^"]*(?:pwsh|powershell)(?:\.exe)?"|\S*(?:pwsh|powershell)(?:\.exe)?)\s+-Command\s+([\s\S]+)$/i,
  );
  const command = (match?.[1] ?? trimmed).trim();
  if (
    command.length >= 2 &&
    ((command.startsWith("'") && command.endsWith("'")) ||
      (command.startsWith('"') && command.endsWith('"')))
  ) {
    return command.slice(1, -1).trim();
  }
  return command;
}

function deduplicate(rows: ToolActivityDetailRow[]): ToolActivityDetailRow[] {
  const seen = new Set<string>();
  return rows.filter((row) => {
    const key = `${row.label}\0${row.value}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function displayValue(value: unknown): string {
  if (value === null || value === undefined || value === '') return '';
  if (typeof value === 'string') return value.trim();
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return '';
  }
}

function firstValue(records: JsonRecord[], keys: string[]): unknown {
  for (const record of records) {
    for (const key of keys)
      if (record[key] !== undefined && record[key] !== null) return record[key];
  }
  return null;
}

function firstValueFromObject(value: unknown, keys: string[]): unknown {
  const record = asRecord(value);
  return record ? firstValue([record], keys) : null;
}

function firstString(records: JsonRecord[], keys: string[]): string | null {
  const value = firstValue(records, keys);
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function firstDefined(values: unknown[]): unknown {
  return values.find((value) => value !== null && value !== undefined);
}

function asRecord(value: unknown): JsonRecord | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as JsonRecord) : null;
}

function usefulToolText(text: string): boolean {
  return Boolean(
    text &&
    ![
      'command execution',
      'file change',
      'web search',
      'plan update',
      'mcp tool call',
      'tool activity',
    ].includes(text.toLowerCase()),
  );
}

function truncate(value: string, length: number): string {
  return value.length > length ? `${value.slice(0, length - 1).trimEnd()}…` : value;
}

function capitalize(value: string): string {
  return value ? `${value[0].toUpperCase()}${value.slice(1)}` : value;
}
