import type { EntityId, IsoDateTime } from '../domain/model';

export type CLIInstanceStatus = 'idle' | 'running' | 'completed' | 'failed' | 'closed';
export type CLIOutputStream = 'stdout' | 'stderr' | 'system';

export interface CLIOutputChunk {
  id: EntityId;
  stream: CLIOutputStream;
  content: string;
  receivedAt: IsoDateTime;
}

export interface CLIInstanceOpenInput {
  sessionId?: EntityId;
  command: string;
  args: readonly string[];
  cwd?: string;
  env?: Record<string, string | undefined>;
}

export interface CLIInstanceRunResult {
  sessionId: EntityId;
  status: 'completed' | 'failed';
  command: string;
  args: string[];
  stdout: string;
  stderr: string;
  metadata?: Record<string, string>;
  startedAt: IsoDateTime;
  completedAt: IsoDateTime;
  exitCode?: number;
  signal?: string;
  error?: string;
  outputWasStreamed?: boolean;
}

export interface CLIInstanceRunner {
  run(
    input: CLIInstanceOpenInput,
    onOutput: (chunk: Omit<CLIOutputChunk, 'id' | 'receivedAt'>) => void,
  ): Promise<CLIInstanceRunResult>;
  close?(sessionId: EntityId): Promise<void>;
}

export interface CLIInstanceSnapshot {
  sessionId: EntityId | null;
  status: CLIInstanceStatus;
  command: string | null;
  args: string[];
  cwd?: string;
  output: CLIOutputChunk[];
  metadata?: Record<string, string>;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  exitCode?: number;
  signal?: string;
  error?: string;
}

export type CLIInstanceListener = (snapshot: CLIInstanceSnapshot) => void;

export class CLIInstanceHandler {
  private listeners = new Set<CLIInstanceListener>();
  private outputSequence = 0;
  private snapshot: CLIInstanceSnapshot = {
    sessionId: null,
    status: 'idle',
    command: null,
    args: [],
    output: [],
  };

  constructor(private readonly runner: CLIInstanceRunner) {}

  getSnapshot(): CLIInstanceSnapshot {
    return cloneSnapshot(this.snapshot);
  }

  subscribe(listener: CLIInstanceListener): () => void {
    this.listeners.add(listener);
    listener(this.getSnapshot());

    return () => {
      this.listeners.delete(listener);
    };
  }

  loadResult(result: CLIInstanceRunResult): CLIInstanceSnapshot {
    this.outputSequence = 0;
    const output = buildProcessOutputChunks(result.stdout, result.stderr, result.completedAt);
    this.outputSequence = output.length;
    this.setSnapshot({
      sessionId: result.sessionId,
      status: result.status,
      command: result.command,
      args: result.args,
      output,
      metadata: result.metadata,
      startedAt: result.startedAt,
      completedAt: result.completedAt,
      exitCode: result.exitCode,
      signal: result.signal,
      error: result.error,
    });

    return this.getSnapshot();
  }

  async open(input: CLIInstanceOpenInput): Promise<CLIInstanceSnapshot> {
    if (this.snapshot.status === 'running') {
      throw new Error('CLI instance is already running.');
    }

    const sessionId = input.sessionId ?? (`cli-session-${crypto.randomUUID()}` as EntityId);
    const startedAt = nowIso();

    this.outputSequence = 0;
    this.setSnapshot({
      sessionId,
      status: 'running',
      command: input.command,
      args: [...input.args],
      ...(input.cwd ? { cwd: input.cwd } : {}),
      output: [],
      startedAt,
    });

    try {
      const result = this.transformRunResult(
        await this.runner.run({ ...input, sessionId }, (chunk) => this.appendOutput(chunk)),
      );

      if (!result.outputWasStreamed) {
        this.appendProcessOutput(result.stdout, result.stderr);
      }

      this.setSnapshot({
        ...this.snapshot,
        sessionId: result.sessionId,
        status: result.status,
        command: result.command,
        args: result.args,
        metadata: result.metadata,
        startedAt: result.startedAt,
        completedAt: result.completedAt,
        exitCode: result.exitCode,
        signal: result.signal,
        error: result.error,
      });
    } catch (caught) {
      this.setSnapshot({
        ...this.snapshot,
        status: 'failed',
        completedAt: nowIso(),
        error: errorMessage(caught),
      });
    }

    return this.getSnapshot();
  }

  async close(reason = 'CLI instance closed.'): Promise<CLIInstanceSnapshot> {
    const sessionId = this.snapshot.sessionId;

    if (sessionId && this.runner.close) {
      await this.runner.close(sessionId);
    }

    this.appendOutput({ stream: 'system', content: reason });
    this.setSnapshot({
      ...this.snapshot,
      status: 'closed',
      completedAt: this.snapshot.completedAt ?? nowIso(),
    });

    return this.getSnapshot();
  }

  protected appendOutput(chunk: Omit<CLIOutputChunk, 'id' | 'receivedAt'>): void {
    const nextChunk: CLIOutputChunk = {
      ...chunk,
      id: `cli-output-${++this.outputSequence}` as EntityId,
      receivedAt: nowIso(),
    };

    this.setSnapshot({
      ...this.snapshot,
      output: [...this.snapshot.output, nextChunk],
    });
  }

  protected transformRunResult(result: CLIInstanceRunResult): CLIInstanceRunResult {
    return result;
  }

  private appendProcessOutput(stdout: string, stderr: string): void {
    splitOutput(stdout).forEach((content) => this.appendOutput({ stream: 'stdout', content }));
    splitOutput(stderr).forEach((content) => this.appendOutput({ stream: 'stderr', content }));
  }

  private setSnapshot(snapshot: CLIInstanceSnapshot): void {
    this.snapshot = snapshot;
    this.listeners.forEach((listener) => listener(this.getSnapshot()));
  }
}

function cloneSnapshot(snapshot: CLIInstanceSnapshot): CLIInstanceSnapshot {
  return {
    ...snapshot,
    args: [...snapshot.args],
    output: snapshot.output.map((chunk) => ({ ...chunk })),
    ...(snapshot.metadata ? { metadata: { ...snapshot.metadata } } : {}),
  };
}

function buildProcessOutputChunks(
  stdout: string,
  stderr: string,
  receivedAt: IsoDateTime,
): CLIOutputChunk[] {
  let sequence = 0;
  return [
    ...splitOutput(stdout).map((content) => ({
      id: `cli-output-${++sequence}` as EntityId,
      stream: 'stdout' as const,
      content,
      receivedAt,
    })),
    ...splitOutput(stderr).map((content) => ({
      id: `cli-output-${++sequence}` as EntityId,
      stream: 'stderr' as const,
      content,
      receivedAt,
    })),
  ];
}

function splitOutput(output: string): string[] {
  return output
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter((line) => line.length > 0);
}

function nowIso(): IsoDateTime {
  return new Date().toISOString() as IsoDateTime;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
