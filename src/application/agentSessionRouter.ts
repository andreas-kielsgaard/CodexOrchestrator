import type { EntityId } from '../domain/model';
import type { RuntimeCommandClient } from './runtimeCommandClient';
import { AgentSession } from './agentSession';
import {
  AgentCLISessionInterface,
  createRuntimeAgentSessionRunner,
  type AgentSessionPromptInput,
} from './agentCLISessionInterface';
import { CLIInstanceHandler } from './cliInstanceHandler';
import {
  AgentSessionOutputFormatter,
  type AgentSessionViewModel,
} from './agentSessionOutputFormatter';
import {
  InMemoryAgentSessionDurableStore,
} from './agentSessionStore';
import type { CLISessionLease } from './cliSessionDistributor';
import { CLISessionMaster } from './cliSessionMaster';

export interface AgentSessionRouteInput extends AgentSessionPromptInput {
  sessionId?: EntityId;
}

export class AgentSessionRouter {
  private readonly sessions = new Map<EntityId, AgentSession>();
  private readonly expandedTurnsBySession = new Map<EntityId, Set<string>>();

  constructor(
    private readonly createSession: (sessionId: EntityId) => AgentSession,
    private readonly formatter = new AgentSessionOutputFormatter(),
    private readonly releaseSession: (session: AgentSession) => void = () => {},
  ) {}

  async launch(
    input: AgentSessionRouteInput,
    onUpdate?: (viewModel: AgentSessionViewModel) => void,
  ): Promise<AgentSessionViewModel> {
    const session = this.resolveSession(input.sessionId);
    const unsubscribe = onUpdate
      ? session.subscribe((record) => onUpdate(this.formatSessionRecord(record)))
      : undefined;

    try {
      const record = await session.deliverPrompt(input);

      this.sessions.set(record.id, session);

      return this.formatSessionRecord(record);
    } finally {
      unsubscribe?.();
    }
  }

  reload(sessionId: EntityId): AgentSessionViewModel {
    const session = this.sessions.get(sessionId);

    if (!session) {
      throw new Error(`Agent session ${sessionId} is not registered.`);
    }

    return this.formatSessionRecord(session.reloadSession(sessionId));
  }

  async close(sessionId: EntityId): Promise<AgentSessionViewModel> {
    const session = this.sessions.get(sessionId);

    if (!session) {
      throw new Error(`Agent session ${sessionId} is not registered.`);
    }

    const viewModel = this.formatSessionRecord(await session.close('Agent session closed.'));
    this.sessions.delete(sessionId);
    this.releaseSession(session);
    return viewModel;
  }

  async reloadStored(sessionId: EntityId): Promise<AgentSessionViewModel> {
    const session = this.sessions.get(sessionId) ?? this.registerSession(this.createSession(sessionId));
    return this.formatSessionRecord(await session.reloadStoredSession(sessionId));
  }

  emptyViewModel(): AgentSessionViewModel {
    return this.formatter.format({
      sessionId: null,
      status: 'idle',
      command: null,
      args: [],
      output: [],
    });
  }

  toggleTurn(sessionId: EntityId, turnId: string): AgentSessionViewModel {
    const expandedTurns = this.expandedTurnsBySession.get(sessionId) ?? new Set<string>();

    if (expandedTurns.has(turnId)) {
      expandedTurns.delete(turnId);
    } else {
      expandedTurns.add(turnId);
    }

    this.expandedTurnsBySession.set(sessionId, expandedTurns);
    return this.reload(sessionId);
  }

  private resolveSession(sessionId: EntityId | undefined): AgentSession {
    if (sessionId) {
      return this.sessions.get(sessionId) ?? this.registerSession(this.createSession(sessionId));
    }

    return this.registerSession(
      this.createSession(`agent-session-pending-${crypto.randomUUID()}` as EntityId),
    );
  }

  private registerSession(session: AgentSession): AgentSession {
    this.sessions.set(session.id, session);
    return session;
  }

  private formatSessionRecord(record: ReturnType<AgentSession['getRecord']>) {
    return this.formatter.formatRecord(record, {
      expandedTurnIds: this.expandedTurnsBySession.get(record.id) ?? new Set(),
    });
  }
}

export function createAgentSessionRouter(
  runtimeCommandClient: RuntimeCommandClient,
): AgentSessionRouter {
  const sessionMaster = new CLISessionMaster();
  const durableStore = new InMemoryAgentSessionDurableStore();
  const leasesBySession = new WeakMap<AgentSession, CLISessionLease>();

  const createSession = (sessionId: EntityId) => {
    const lease = sessionMaster.acquire({
      purpose: 'agent-session',
      createHandler: () =>
        new CLIInstanceHandler(createRuntimeAgentSessionRunner(runtimeCommandClient)),
    });
    const cliInterface = new AgentCLISessionInterface(lease.handler, runtimeCommandClient);
    const session = new AgentSession(sessionId, durableStore, cliInterface);
    leasesBySession.set(session, lease);
    return session;
  };

  return new AgentSessionRouter(createSession, new AgentSessionOutputFormatter(), (session) => {
    const lease = leasesBySession.get(session);
    if (lease) {
      sessionMaster.release(lease);
    }
  });
}
