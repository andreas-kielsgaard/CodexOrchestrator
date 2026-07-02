import { spawn } from 'node:child_process';
import process from 'node:process';

import {
  parseCodexJsonlEvents,
  summarizeCodexJsonlEvents,
  type CodexJsonlEvent,
  type CodexJsonlEventSummary,
} from './jsonlEvents';

export type CodexExecStatus = 'completed' | 'failed' | 'error';

export interface CodexRuntime {
  exec(input: CodexExecInput): Promise<CodexExecResult>;
}

export interface CodexExecInput {
  prompt: string;
  cwd?: string;
  additionalArgs?: readonly string[];
  env?: Record<string, string | undefined>;
  onStdoutChunk?: (chunk: string) => void;
  onStderrChunk?: (chunk: string) => void;
}

export interface CodexExecResult {
  command: string;
  args: string[];
  cwd?: string;
  exitCode: number | null;
  signal: string | null;
  status: CodexExecStatus;
  statusReason: string;
  stdoutJsonl: string;
  stderr: string;
  events: CodexJsonlEvent[];
  summary: CodexJsonlEventSummary;
}

export interface CodexRuntimeOptions {
  command?: string;
  runner?: CodexProcessRunner;
}

export interface CodexProcessRunner {
  run(input: CodexProcessRunInput): Promise<CodexProcessRunResult>;
}

export interface CodexProcessRunInput {
  command: string;
  args: string[];
  cwd?: string;
  env?: Record<string, string | undefined>;
  onStdoutChunk?: (chunk: string) => void;
  onStderrChunk?: (chunk: string) => void;
}

export interface CodexProcessRunResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  signal: string | null;
}

export function createCodexRuntime(options: CodexRuntimeOptions = {}): CodexRuntime {
  const command = options.command ?? 'codex';
  const runner = options.runner ?? createNodeCodexProcessRunner();

  return {
    exec: (input) => runCodexExec(input, { command, runner }),
  };
}

export interface RunCodexExecDependencies {
  command: string;
  runner: CodexProcessRunner;
}

export async function runCodexExec(
  input: CodexExecInput,
  dependencies: RunCodexExecDependencies,
): Promise<CodexExecResult> {
  const args = buildCodexExecArgs(input);
  const processResult = await dependencies.runner.run({
    command: dependencies.command,
    args,
    ...(input.cwd === undefined ? {} : { cwd: input.cwd }),
    ...(input.env === undefined ? {} : { env: input.env }),
    ...(input.onStdoutChunk === undefined ? {} : { onStdoutChunk: input.onStdoutChunk }),
    ...(input.onStderrChunk === undefined ? {} : { onStderrChunk: input.onStderrChunk }),
  });

  const events = parseCodexJsonlEvents(processResult.stdout);
  const summary = summarizeCodexJsonlEvents(events);
  const classification = classifyCodexExecResult(processResult, summary);

  return {
    command: dependencies.command,
    args,
    ...(input.cwd === undefined ? {} : { cwd: input.cwd }),
    exitCode: processResult.exitCode,
    signal: processResult.signal,
    status: classification.status,
    statusReason: classification.statusReason,
    stdoutJsonl: processResult.stdout,
    stderr: processResult.stderr,
    events,
    summary,
  };
}

export function buildCodexExecArgs(input: CodexExecInput): string[] {
  return ['exec', '--json', ...(input.additionalArgs ?? []), input.prompt];
}

export function createNodeCodexProcessRunner(): CodexProcessRunner {
  return {
    run: (input) =>
      new Promise((resolve, reject) => {
        const child = spawn(input.command, input.args, {
          cwd: input.cwd,
          env: input.env === undefined ? undefined : { ...process.env, ...input.env },
          shell: false,
          windowsHide: true,
        });
        let stdout = '';
        let stderr = '';
        let settled = false;

        child.stdout.setEncoding('utf8');
        child.stdout.on('data', (chunk: string) => {
          stdout += chunk;
          input.onStdoutChunk?.(chunk);
        });

        child.stderr.setEncoding('utf8');
        child.stderr.on('data', (chunk: string) => {
          stderr += chunk;
          input.onStderrChunk?.(chunk);
        });

        child.once('error', (error) => {
          if (settled) {
            return;
          }

          settled = true;
          reject(error);
        });

        child.once('close', (exitCode, signal) => {
          if (settled) {
            return;
          }

          settled = true;
          resolve({
            stdout,
            stderr,
            exitCode,
            signal,
          });
        });
      }),
  };
}

interface CodexExecClassification {
  status: CodexExecStatus;
  statusReason: string;
}

function classifyCodexExecResult(
  processResult: CodexProcessRunResult,
  summary: CodexJsonlEventSummary,
): CodexExecClassification {
  if (summary.terminalStatus?.kind === 'error') {
    return { status: 'error', statusReason: 'Codex emitted an error event' };
  }

  if (summary.terminalStatus?.kind === 'failed') {
    return { status: 'failed', statusReason: 'Codex emitted a turn.failed event' };
  }

  if (processResult.signal !== null) {
    return {
      status: 'failed',
      statusReason: `Codex process exited on signal ${processResult.signal}`,
    };
  }

  if (processResult.exitCode !== 0) {
    return {
      status: 'failed',
      statusReason: `Codex process exited with code ${processResult.exitCode}`,
    };
  }

  if (summary.terminalStatus?.kind === 'completed') {
    return { status: 'completed', statusReason: 'Codex emitted a turn.completed event' };
  }

  return { status: 'failed', statusReason: 'Codex output did not include a terminal event' };
}
