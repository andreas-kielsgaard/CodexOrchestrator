import type { EntityId } from '../domain/model';
import {
  AgentCLISessionInterface,
  type AgentSessionPromptInput,
} from './agentCLISessionInterface';
import type { CLIInstanceSnapshot } from './cliInstanceHandler';
import {
  createAgentSessionTurnRecord,
  createEmptyAgentSessionRecord,
  type AgentSessionDurableStore,
  type AgentSessionRecord,
  type AgentSessionTurnStatus,
} from './agentSessionStore';

export type AgentSessionListener = (record: AgentSessionRecord) => void;

export class AgentSession {
  private listeners = new Set<AgentSessionListener>();
  private activeSessionId: EntityId;

  constructor(
    sessionId: EntityId,
    private readonly durableStore: AgentSessionDurableStore,
    private readonly cliInterface: AgentCLISessionInterface,
  ) {
    this.activeSessionId = sessionId;

    if (!this.durableStore.loadSession(sessionId)) {
      this.durableStore.saveSession(createEmptyAgentSessionRecord(sessionId));
    }
  }

  get id(): EntityId {
    return this.activeSessionId;
  }

  getRecord(): AgentSessionRecord {
    return this.requireRecord();
  }

  subscribe(listener: AgentSessionListener): () => void {
    this.listeners.add(listener);
    listener(this.getRecord());

    return () => {
      this.listeners.delete(listener);
    };
  }

  async deliverPrompt(input: AgentSessionPromptInput): Promise<AgentSessionRecord> {
    const turnId = `agent-turn-${crypto.randomUUID()}` as EntityId;
    const promptInput = {
      ...input,
      ...(this.shouldResumeExistingSession(input) ? { sessionId: this.activeSessionId } : {}),
    };
    let sessionId = this.activeSessionId;
    this.durableStore.appendTurn(
      sessionId,
      createAgentSessionTurnRecord({
        id: turnId,
        prompt: input.prompt,
      }),
    );
    this.emit();

    let observedCurrentRun = false;
    const unsubscribe = this.cliInterface.subscribe((snapshot) => {
      if (!observedCurrentRun && snapshot.status !== 'running') {
        return;
      }

      observedCurrentRun = true;
      this.applyCliSnapshot(sessionId, turnId, snapshot);
    });

    try {
      const snapshot = await this.cliInterface.deliverPrompt(promptInput);
      this.applyCliSnapshot(sessionId, turnId, snapshot);

      if (snapshot.sessionId && snapshot.sessionId !== sessionId) {
        const renamed = this.durableStore.renameSession(sessionId, snapshot.sessionId);
        this.activeSessionId = renamed.id;
        sessionId = renamed.id;
        this.emit();
      }

      return this.getRecord();
    } finally {
      unsubscribe();
    }
  }

  reloadSession(sessionId: EntityId): AgentSessionRecord {
    const record = this.durableStore.loadSession(sessionId);

    if (!record) {
      throw new Error(`Agent session ${sessionId} is not registered.`);
    }

    this.activeSessionId = sessionId;
    return record;
  }

  async reloadStoredSession(sessionId: EntityId): Promise<AgentSessionRecord> {
    const snapshot = await this.cliInterface.reloadStoredSession(sessionId);
    const record = createEmptyAgentSessionRecord(sessionId);
    const turn = createAgentSessionTurnRecord({
      id: `agent-turn-${crypto.randomUUID()}` as EntityId,
      prompt: snapshot.args.at(-1) ?? '',
      startedAt: snapshot.startedAt,
    });
    this.durableStore.saveSession({
      ...record,
      status: snapshot.status,
      turns: [
        {
          ...turn,
          status: turnStatusFromCliSnapshot(snapshot),
          command: snapshot.command,
          args: snapshot.args,
          output: snapshot.output,
          metadata: snapshot.metadata,
          completedAt: snapshot.completedAt,
          exitCode: snapshot.exitCode,
          signal: snapshot.signal,
          error: snapshot.error,
        },
      ],
      updatedAt: snapshot.completedAt ?? record.updatedAt,
    });
    this.activeSessionId = sessionId;
    this.emit();
    return this.getRecord();
  }

  async close(reason = 'Agent session closed.'): Promise<AgentSessionRecord> {
    const snapshot = await this.cliInterface.close(reason);
    const latestTurn = this.requireRecord().turns.at(-1);

    if (latestTurn) {
      this.applyCliSnapshot(this.activeSessionId, latestTurn.id, snapshot);
    }

    const record = {
      ...this.getRecord(),
      status: 'closed' as const,
    };
    this.durableStore.saveSession(record);
    this.emit();
    return this.getRecord();
  }

  private applyCliSnapshot(
    sessionId: EntityId,
    turnId: EntityId,
    snapshot: CLIInstanceSnapshot,
  ): void {
    this.durableStore.updateTurn(sessionId, turnId, {
      status: turnStatusFromCliSnapshot(snapshot),
      command: snapshot.command,
      args: snapshot.args,
      output: snapshot.output,
      metadata: snapshot.metadata,
      startedAt: snapshot.startedAt,
      completedAt: snapshot.completedAt,
      exitCode: snapshot.exitCode,
      signal: snapshot.signal,
      error: snapshot.error,
    });
    this.emit();
  }

  private shouldResumeExistingSession(input: AgentSessionPromptInput): boolean {
    return input.sessionId !== undefined || this.getRecord().turns.length > 0;
  }

  private requireRecord(): AgentSessionRecord {
    const record = this.durableStore.loadSession(this.activeSessionId);

    if (!record) {
      throw new Error(`Agent session ${this.activeSessionId} was not found.`);
    }

    return record;
  }

  private emit(): void {
    const record = this.getRecord();
    this.listeners.forEach((listener) => listener(record));
  }
}

function turnStatusFromCliSnapshot(snapshot: CLIInstanceSnapshot): AgentSessionTurnStatus {
  switch (snapshot.status) {
    case 'completed':
    case 'failed':
    case 'closed':
      return snapshot.status;
    case 'idle':
    case 'running':
      return 'running';
  }
}
