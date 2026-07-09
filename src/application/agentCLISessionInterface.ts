import type { EntityId } from '../domain/model';
import type {
  RuntimeCommandClient,
  StartAgentSessionCommandInput,
  StartAgentSessionCommandResult,
} from './runtimeCommandClient';
import {
  CLIInstanceHandler,
  type CLIInstanceListener,
  type CLIInstanceOpenInput,
  type CLIInstanceRunResult,
  type CLIInstanceRunner,
  type CLIInstanceSnapshot,
  type CLIOutputChunk,
} from './cliInstanceHandler';

export interface AgentSessionPromptInput {
  sessionId?: EntityId;
  prompt: string;
  cwd?: string;
  additionalArgs?: readonly string[];
  env?: Record<string, string | undefined>;
}

export interface AgentSessionStorageGateway {
  loadAgentSession?(sessionId: EntityId): Promise<StartAgentSessionCommandResult | null>;
}

export class AgentCLISessionInterface {
  constructor(
    private readonly cliInstanceHandler: CLIInstanceHandler,
    private readonly storageGateway: AgentSessionStorageGateway = {},
  ) {}

  getSnapshot(): CLIInstanceSnapshot {
    return this.cliInstanceHandler.getSnapshot();
  }

  subscribe(listener: CLIInstanceListener): () => void {
    return this.cliInstanceHandler.subscribe(listener);
  }

  deliverPrompt(input: AgentSessionPromptInput): Promise<CLIInstanceSnapshot> {
    return this.cliInstanceHandler.open({
      sessionId: input.sessionId,
      command: 'codex',
      args: buildAgentSessionCliArgs(input),
      ...(input.cwd ? { cwd: input.cwd } : {}),
      ...(input.env ? { env: input.env } : {}),
    });
  }

  reloadSession(sessionId: EntityId): CLIInstanceSnapshot {
    const snapshot = this.cliInstanceHandler.getSnapshot();

    if (snapshot.sessionId !== sessionId) {
      throw new Error(`Agent session ${sessionId} is not loaded in this CLI interface.`);
    }

    return snapshot;
  }

  async reloadStoredSession(sessionId: EntityId): Promise<CLIInstanceSnapshot> {
    if (!this.storageGateway.loadAgentSession) {
      throw new Error('Agent session storage is not available.');
    }

    const result = await this.storageGateway.loadAgentSession(sessionId);

    if (!result) {
      throw new Error(`Agent session ${sessionId} was not found in storage.`);
    }

    return this.cliInstanceHandler.loadResult({
      sessionId: result.sessionId,
      status: result.status,
      command: result.command,
      args: result.args,
      stdout: result.stdout,
      stderr: result.stderr,
      startedAt: result.startedAt,
      completedAt: result.completedAt,
      exitCode: result.exitCode,
      signal: result.signal,
      error: result.error,
      outputWasStreamed: false,
    });
  }

  close(reason = 'Agent session closed.'): Promise<CLIInstanceSnapshot> {
    return this.cliInstanceHandler.close(reason);
  }
}

export function createRuntimeAgentSessionRunner(
  runtimeCommandClient: RuntimeCommandClient,
): CLIInstanceRunner {
  return {
    async run(
      input: CLIInstanceOpenInput,
      onOutput: (chunk: Omit<CLIOutputChunk, 'id' | 'receivedAt'>) => void,
    ): Promise<CLIInstanceRunResult> {
      const prompt = input.args[input.args.length - 1];

      if (!prompt) {
        throw new Error('Agent session prompt is required.');
      }

      onOutput({
        stream: 'system',
        content: `Launching ${formatCommand(input.command, input.args)}`,
      });

      const sessionId = resumedAgentSessionId(input);
      const commandInput: StartAgentSessionCommandInput = {
        ...(sessionId ? { sessionId } : {}),
        prompt,
        ...additionalAgentSessionRuntimeArgs(input),
        ...(input.cwd ? { cwd: input.cwd } : {}),
        ...(input.env ? { env: input.env } : {}),
      };
      const result = await runtimeCommandClient.startAgentSession(commandInput, {
        onOutput,
      });

      return {
        sessionId: result.sessionId,
        status: result.status,
        command: result.command,
        args: result.args,
        stdout: result.stdout,
        stderr: result.stderr,
        outputWasStreamed: result.outputWasStreamed,
        startedAt: result.startedAt,
        completedAt: result.completedAt,
        exitCode: result.exitCode,
        signal: result.signal,
        error: result.error,
      };
    },
  };
}

function resumedAgentSessionId(input: CLIInstanceOpenInput): EntityId | undefined {
  const resumeIndex = input.args.indexOf('resume');
  const sessionId = resumeIndex >= 0 ? input.args[resumeIndex + 1] : undefined;

  return sessionId ? (sessionId as EntityId) : undefined;
}

function buildAgentSessionCliArgs(input: AgentSessionPromptInput): string[] {
  return [
    'exec',
    '--json',
    ...(input.additionalArgs ?? []),
    ...(input.sessionId ? ['resume', input.sessionId] : []),
    input.prompt,
  ];
}

function additionalAgentSessionRuntimeArgs(
  input: CLIInstanceOpenInput,
): Pick<StartAgentSessionCommandInput, 'additionalArgs'> {
  const argsWithoutPrompt = input.args.slice(0, -1);
  const additionalArgs = argsWithoutPrompt.filter((arg, index, args) => {
    if (arg === 'exec') {
      return false;
    }

    if (arg === '--json') {
      return false;
    }

    if (input.sessionId && arg === 'resume') {
      return false;
    }

    if (input.sessionId && args[index - 1] === 'resume' && arg === input.sessionId) {
      return false;
    }

    return true;
  });

  return additionalArgs.length > 0 ? { additionalArgs } : {};
}

function formatCommand(command: string, args: readonly string[]): string {
  return [command, ...args.map(quoteArg)].join(' ');
}

function quoteArg(arg: string): string {
  return /\s/.test(arg) ? `"${arg.replaceAll('"', '\\"')}"` : arg;
}
