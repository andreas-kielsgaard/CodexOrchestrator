import type {
  AgentInvocationDto,
  AgentInvocationStatusDto,
  AgentRuntimeEventDto,
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionDto,
  AgentSessionSummaryDto,
  AgentSessionUpdateDto,
  AgentSessionUpdateListener,
  CancelAgentInvocationCommandDto,
  CreateAgentSessionCommandDto,
  ListAgentSessionsQueryDto,
  LoadAgentSessionQueryDto,
  SendAgentSessionMessageCommandDto,
  SendAgentSessionMessageResultDto,
} from '../../application/agentSessions';
import type { RecordedAgentSessionScenario, RecordedStep } from './scenarios';

export interface RecordedAgentSessionStore {
  sessions: Map<string, AgentSessionDetailsDto>;
  summaries: Map<string, AgentSessionSummaryDto>;
}

export function createRecordedAgentSessionStore(
  initial: readonly AgentSessionDetailsDto[] = [],
): RecordedAgentSessionStore {
  const sessions = new Map(initial.map((details) => [details.session.id, clone(details)]));
  return {
    sessions,
    summaries: new Map(
      [...sessions.values()].map((details) => [
        details.session.id,
        summary(details.session, details.invocations),
      ]),
    ),
  };
}

export interface RecordedAgentSessionClientOptions {
  store?: RecordedAgentSessionStore;
  scenario?: RecordedAgentSessionScenario;
}

export interface RecordedAgentSessionClient extends AgentSessionClient {
  readonly store: RecordedAgentSessionStore;
  readonly emittedUpdates: readonly AgentSessionUpdateDto[];
  readonly stepIndex: number;
  readonly stepCount: number;
  advance(): boolean;
  advanceAll(): number;
  peekNextStep(): RecordedStep | undefined;
}

const defaultTime = '2026-07-10T12:00:00.000Z';

export function createRecordedAgentSessionClient(
  options: RecordedAgentSessionClientOptions = {},
): RecordedAgentSessionClient {
  const store = options.store ?? createRecordedAgentSessionStore();
  const scenario = options.scenario;
  const listeners = new Set<AgentSessionUpdateListener>();
  const emittedUpdates: AgentSessionUpdateDto[] = [];
  let stepIndex = scenario ? findAppliedStepCount(store, scenario.steps) : 0;
  let nextSession = 1;
  let nextInvocation = 1;

  for (const fixture of scenario?.sessions ?? []) {
    const existing = store.sessions.get(fixture.sessionId);
    if (!existing) {
      const session = makeSession(fixture.sessionId, fixture.sessionId, null);
      store.sessions.set(fixture.sessionId, {
        session,
        invocations: [
          {
            invocation: makeInvocation(
              fixture.invocationId,
              fixture.sessionId,
              fixture.submittedText,
              'pending',
            ),
            events: [],
          },
        ],
      });
      store.summaries.set(
        fixture.sessionId,
        summary(session, store.sessions.get(fixture.sessionId)!.invocations),
      );
    } else if (
      !existing.invocations.some(({ invocation }) => invocation.id === fixture.invocationId)
    ) {
      existing.invocations.push({
        invocation: makeInvocation(
          fixture.invocationId,
          fixture.sessionId,
          fixture.submittedText,
          'pending',
        ),
        events: [],
      });
      store.summaries.set(fixture.sessionId, summary(existing.session, existing.invocations));
    }
  }

  const failure = (operation: keyof NonNullable<RecordedAgentSessionScenario['failures']>) => {
    const message = scenario?.failures?.[operation];
    if (message) throw new Error(message);
  };
  const emit = (update: AgentSessionUpdateDto) => {
    const safe = clone(update);
    emittedUpdates.push(safe);
    for (const listener of listeners) listener(clone(safe));
  };
  const findInvocation = (
    id: string,
  ): { details: AgentSessionDetailsDto; invocation: AgentInvocationDto } => {
    for (const details of store.sessions.values()) {
      const found = details.invocations.find(({ invocation }) => invocation.id === id);
      if (found) return { details, invocation: found.invocation };
    }
    throw new Error(`Recorded invocation not found: ${id}`);
  };
  const client: RecordedAgentSessionClient = {
    store,
    emittedUpdates,
    get stepIndex() {
      return stepIndex;
    },
    get stepCount() {
      return scenario?.steps.length ?? 0;
    },
    async createSession(command: CreateAgentSessionCommandDto) {
      const id = `recorded-session-${nextSession++}`;
      const session = makeSession(
        id,
        command.title ?? 'Recorded Agent Session',
        command.workingDirectory ?? null,
      );
      store.sessions.set(id, { session, invocations: [] });
      store.summaries.set(id, summary(session, []));
      return clone(session);
    },
    async listSessions(query: ListAgentSessionsQueryDto = {}) {
      const result = [...store.summaries.values()]
        .filter((item) => !query.availability || item.availability === query.availability)
        .sort((a, b) => a.createdAt.localeCompare(b.createdAt) || a.id.localeCompare(b.id));
      return clone(typeof query.limit === 'number' ? result.slice(0, query.limit) : result);
    },
    async loadSession(query: LoadAgentSessionQueryDto) {
      failure('load');
      const details = store.sessions.get(query.sessionId);
      if (!details) throw new Error(`Recorded session not found: ${query.sessionId}`);
      return clone(details);
    },
    async reloadSession(query: LoadAgentSessionQueryDto) {
      failure('reload');
      const details = store.sessions.get(query.sessionId);
      if (!details) throw new Error(`Recorded session not found: ${query.sessionId}`);
      return clone(details);
    },
    async subscribeUpdates(listener: AgentSessionUpdateListener) {
      failure('subscribe');
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    async sendMessage(
      command: SendAgentSessionMessageCommandDto,
    ): Promise<SendAgentSessionMessageResultDto> {
      failure('send');
      let session = command.sessionId ? store.sessions.get(command.sessionId)?.session : undefined;
      if (command.sessionId && !session)
        throw new Error(`Recorded session not found: ${command.sessionId}`);
      if (!session) {
        const created = await client.createSession({
          title: command.title,
          workingDirectory: command.workingDirectory,
          requestedOptions: command.requestedOptions,
        });
        session = created;
      }
      const invocationId = `recorded-invocation-${nextInvocation++}`;
      const invocation = makeInvocation(invocationId, session.id, command.submittedText, 'pending');
      store.sessions.get(session.id)!.invocations.push({ invocation, events: [] });
      store.summaries.set(
        session.id,
        summary(session, store.sessions.get(session.id)!.invocations),
      );
      return { sessionId: session.id, invocationId };
    },
    async cancelInvocation(command: CancelAgentInvocationCommandDto) {
      failure('cancel');
      const found = findInvocation(command.invocationId);
      if (!['pending', 'running'].includes(found.invocation.status)) {
        throw new Error(`Invocation is already terminal: ${command.invocationId}`);
      }
      found.invocation.status = 'canceled';
      found.invocation.completedAt = defaultTime;
      found.invocation.updatedAt = defaultTime;
      found.invocation.exitCode = null;
      store.summaries.set(
        found.details.session.id,
        summary(found.details.session, found.details.invocations),
      );
      emit({
        kind: 'invocation_terminal',
        sessionId: found.details.session.id,
        invocationId: command.invocationId,
        invocation: clone(found.invocation),
      });
      return clone(found.invocation);
    },
    async disconnectUpdates() {
      listeners.clear();
    },
    advance() {
      const step = scenario?.steps[stepIndex++];
      if (!step) return false;
      applyStep(step);
      return true;
    },
    advanceAll() {
      let count = 0;
      while (client.advance()) count += 1;
      return count;
    },
    peekNextStep() {
      return scenario?.steps[stepIndex];
    },
  };

  function applyStep(step: RecordedStep): void {
    const details = store.sessions.get(step.sessionId);
    if (!details) throw new Error(`Recorded scenario session not found: ${step.sessionId}`);
    const entry = details.invocations.find(({ invocation }) => invocation.id === step.invocationId);
    if (!entry) throw new Error(`Recorded scenario invocation not found: ${step.invocationId}`);
    if (step.kind === 'event') {
      entry.events.push(clone(step.event));
      emit({
        kind: 'event_persisted',
        sessionId: step.sessionId,
        invocationId: step.invocationId,
        event: clone(step.event),
      });
    } else if (step.kind === 'diagnostic') {
      entry.invocation.diagnostics.push(clone(step.diagnostic));
      emit({
        kind: 'diagnostic_recorded',
        sessionId: step.sessionId,
        invocationId: step.invocationId,
        invocation: clone(entry.invocation),
      });
    } else {
      Object.assign(entry.invocation, clone(step.invocation));
      store.summaries.set(step.sessionId, summary(details.session, details.invocations));
      emit({
        kind: 'invocation_terminal',
        sessionId: step.sessionId,
        invocationId: step.invocationId,
        invocation: clone(entry.invocation),
      });
    }
  }
  return client;
}

function makeSession(id: string, title: string, workingDirectory: string | null): AgentSessionDto {
  return {
    id,
    title,
    availability: 'available',
    runtimeBinding: { externalContextId: null, runtimeVersion: 'recorded' },
    workingDirectory,
    requestedOptions: { model: null, sandbox: null },
    createdAt: defaultTime,
    updatedAt: defaultTime,
  };
}
function makeInvocation(
  id: string,
  sessionId: string,
  submittedText: string,
  status: AgentInvocationStatusDto,
): AgentInvocationDto {
  return {
    id,
    sessionId,
    submittedText,
    inputProvenance: 'user',
    status,
    requestedOptions: { model: null, sandbox: null },
    effectiveOptions: null,
    startedAt: status === 'pending' ? null : defaultTime,
    completedAt: null,
    exitCode: null,
    signal: null,
    runtimeError: null,
    diagnostics: [],
    createdAt: defaultTime,
    updatedAt: defaultTime,
  };
}
function summary(
  session: AgentSessionDto,
  invocations: { invocation: AgentInvocationDto }[],
): AgentSessionSummaryDto {
  return {
    id: session.id,
    title: session.title,
    availability: session.availability,
    hasActiveInvocation: invocations.some(({ invocation }) =>
      ['pending', 'running'].includes(invocation.status),
    ),
    agentIdentity: session.agentIdentity,
    createdAt: session.createdAt,
    updatedAt: session.updatedAt,
  };
}

function findAppliedStepCount(store: RecordedAgentSessionStore, steps: readonly RecordedStep[]) {
  let count = 0;
  for (const step of steps) {
    const details = store.sessions.get(step.sessionId);
    const entry = details?.invocations.find(
      ({ invocation }) => invocation.id === step.invocationId,
    );
    if (!entry || !isStepApplied(step, entry)) break;
    count += 1;
  }
  return count;
}

function isStepApplied(
  step: RecordedStep,
  entry: { invocation: AgentInvocationDto; events: AgentRuntimeEventDto[] },
): boolean {
  if (step.kind === 'event') return entry.events.some((event) => event.id === step.event.id);
  if (step.kind === 'diagnostic') {
    return entry.invocation.diagnostics.some(
      (diagnostic) =>
        diagnostic.code === step.diagnostic.code &&
        diagnostic.recordedAt === step.diagnostic.recordedAt &&
        diagnostic.message === step.diagnostic.message,
    );
  }
  return (
    entry.invocation.status === step.invocation.status &&
    entry.invocation.completedAt === step.invocation.completedAt
  );
}

function clone<T>(value: T): T {
  return structuredClone(value);
}
