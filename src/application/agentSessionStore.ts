import type { EntityId, IsoDateTime } from '../domain/model';
import type { CLIInstanceSnapshot, CLIOutputChunk } from './cliInstanceHandler';

export type AgentSessionTurnStatus = 'running' | 'completed' | 'failed' | 'closed';

export interface AgentSessionTurnRecord {
  id: EntityId;
  prompt: string;
  status: AgentSessionTurnStatus;
  command: string | null;
  args: string[];
  output: CLIOutputChunk[];
  metadata?: Record<string, string>;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  exitCode?: number;
  signal?: string;
  error?: string;
}

export interface AgentSessionRecord {
  id: EntityId;
  status: CLIInstanceSnapshot['status'];
  turns: AgentSessionTurnRecord[];
  metadata?: Record<string, string>;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
  error?: string;
}

export interface AgentSessionDurableStore {
  loadSession(sessionId: EntityId): AgentSessionRecord | null;
  saveSession(session: AgentSessionRecord): AgentSessionRecord;
  deleteSession(sessionId: EntityId): void;
  renameSession(previousSessionId: EntityId, nextSessionId: EntityId): AgentSessionRecord;
  appendTurn(sessionId: EntityId, turn: AgentSessionTurnRecord): AgentSessionRecord;
  updateTurn(
    sessionId: EntityId,
    turnId: EntityId,
    update: Partial<AgentSessionTurnRecord>,
  ): AgentSessionRecord;
}

export class InMemoryAgentSessionDurableStore implements AgentSessionDurableStore {
  private readonly sessions = new Map<EntityId, AgentSessionRecord>();

  loadSession(sessionId: EntityId): AgentSessionRecord | null {
    const session = this.sessions.get(sessionId);
    return session ? cloneSession(session) : null;
  }

  saveSession(session: AgentSessionRecord): AgentSessionRecord {
    const cloned = cloneSession(session);
    this.sessions.set(cloned.id, cloned);
    return cloneSession(cloned);
  }

  deleteSession(sessionId: EntityId): void {
    this.sessions.delete(sessionId);
  }

  renameSession(previousSessionId: EntityId, nextSessionId: EntityId): AgentSessionRecord {
    const session = this.requireSession(previousSessionId);
    this.sessions.delete(previousSessionId);
    const renamed = {
      ...session,
      id: nextSessionId,
      updatedAt: nowIso(),
    };
    this.sessions.set(nextSessionId, renamed);
    return cloneSession(renamed);
  }

  appendTurn(sessionId: EntityId, turn: AgentSessionTurnRecord): AgentSessionRecord {
    const session = this.requireSession(sessionId);
    const updated = {
      ...session,
      turns: [...session.turns, cloneTurn(turn)],
      status: 'running' as const,
      updatedAt: nowIso(),
    };
    this.sessions.set(sessionId, updated);
    return cloneSession(updated);
  }

  updateTurn(
    sessionId: EntityId,
    turnId: EntityId,
    update: Partial<AgentSessionTurnRecord>,
  ): AgentSessionRecord {
    const session = this.requireSession(sessionId);
    const turns = session.turns.map((turn) =>
      turn.id === turnId ? cloneTurn({ ...turn, ...update }) : turn,
    );
    const updated = {
      ...session,
      status: sessionStatusFromTurns(turns),
      turns,
      updatedAt: nowIso(),
      ...(update.error ? { error: update.error } : {}),
    };
    this.sessions.set(sessionId, updated);
    return cloneSession(updated);
  }

  private requireSession(sessionId: EntityId): AgentSessionRecord {
    const session = this.sessions.get(sessionId);

    if (!session) {
      throw new Error(`Agent session ${sessionId} was not found.`);
    }

    return session;
  }
}

export function createEmptyAgentSessionRecord(sessionId: EntityId): AgentSessionRecord {
  const timestamp = nowIso();
  return {
    id: sessionId,
    status: 'idle',
    turns: [],
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}

export function createAgentSessionTurnRecord(input: {
  id: EntityId;
  prompt: string;
  startedAt?: IsoDateTime;
}): AgentSessionTurnRecord {
  return {
    id: input.id,
    prompt: input.prompt,
    status: 'running',
    command: null,
    args: [],
    output: [],
    ...(input.startedAt ? { startedAt: input.startedAt } : {}),
  };
}

function sessionStatusFromTurns(turns: AgentSessionTurnRecord[]): AgentSessionRecord['status'] {
  const latest = turns.at(-1);

  if (!latest) {
    return 'idle';
  }

  if (latest.status === 'running') {
    return 'running';
  }

  if (latest.status === 'closed') {
    return 'closed';
  }

  return latest.status;
}

function cloneSession(session: AgentSessionRecord): AgentSessionRecord {
  return {
    ...session,
    turns: session.turns.map(cloneTurn),
    ...(session.metadata ? { metadata: { ...session.metadata } } : {}),
  };
}

function cloneTurn(turn: AgentSessionTurnRecord): AgentSessionTurnRecord {
  return {
    ...turn,
    args: [...turn.args],
    output: turn.output.map((chunk) => ({ ...chunk })),
    ...(turn.metadata ? { metadata: { ...turn.metadata } } : {}),
  };
}

function nowIso(): IsoDateTime {
  return new Date().toISOString() as IsoDateTime;
}
