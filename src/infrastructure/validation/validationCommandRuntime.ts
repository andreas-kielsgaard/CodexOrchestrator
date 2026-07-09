import { spawn } from 'node:child_process';
import process from 'node:process';

import type {
  ValidationCommandRuntime,
  ValidationCommandRuntimeInput,
  ValidationCommandRuntimeResult,
} from '../../application/commands/validationCommandRunner';

export interface ValidationCommandRuntimeOptions {
  runner?: ValidationCommandProcessRunner;
}

export interface ValidationCommandProcessRunner {
  run(input: ValidationCommandProcessRunInput): Promise<ValidationCommandProcessRunResult>;
}

export interface ValidationCommandProcessRunInput {
  command: string;
  args: string[];
  cwd: string;
  env?: Record<string, string | undefined>;
  onStdoutChunk?: (chunk: string) => void;
  onStderrChunk?: (chunk: string) => void;
}

export type ValidationCommandProcessRunResult = ValidationCommandRuntimeResult;

export interface RunValidationCommandDependencies {
  runner: ValidationCommandProcessRunner;
}

export function createValidationCommandRuntime(
  options: ValidationCommandRuntimeOptions = {},
): ValidationCommandRuntime {
  const runner = options.runner ?? createNodeValidationCommandProcessRunner();

  return {
    run: (input) => runValidationCommand(input, { runner }),
  };
}

export async function runValidationCommand(
  input: ValidationCommandRuntimeInput,
  dependencies: RunValidationCommandDependencies,
): Promise<ValidationCommandRuntimeResult> {
  return dependencies.runner.run({
    command: input.command,
    args: [...(input.args ?? [])],
    cwd: input.cwd,
    ...(input.env === undefined ? {} : { env: input.env }),
    ...(input.onStdoutChunk === undefined ? {} : { onStdoutChunk: input.onStdoutChunk }),
    ...(input.onStderrChunk === undefined ? {} : { onStderrChunk: input.onStderrChunk }),
  });
}

export function createNodeValidationCommandProcessRunner(): ValidationCommandProcessRunner {
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
