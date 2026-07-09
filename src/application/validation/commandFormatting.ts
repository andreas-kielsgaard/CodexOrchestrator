import type { ValidationCommandRuntimeResult } from './types';

export function classifyValidationStatus(
  result: ValidationCommandRuntimeResult,
): 'passed' | 'failed' {
  if (result.exitCode === 0 && result.signal === null) {
    return 'passed';
  }

  return 'failed';
}

export function renderValidationCommand(
  command: string,
  args: readonly string[] | undefined,
): string {
  const renderedArgs = (args ?? []).map(renderCommandArg);

  return [command, ...renderedArgs].join(' ');
}

function renderCommandArg(arg: string): string {
  if (/^[A-Za-z0-9_./:=@+-]+$/.test(arg)) {
    return arg;
  }

  return JSON.stringify(arg);
}

export function numericExitCode(exitCode: number | null): number | undefined {
  return typeof exitCode === 'number' && Number.isFinite(exitCode) ? exitCode : undefined;
}
