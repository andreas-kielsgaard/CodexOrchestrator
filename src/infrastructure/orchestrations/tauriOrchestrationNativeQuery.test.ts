import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  createNativeEpicPlanProposalSource,
  createNativeQueryOrchestrationClient,
  createTauriOrchestrationNativeQueryClient,
} from './tauriOrchestrationNativeQuery';
import { presentProductOrchestrations } from '../../app/orchestrationPresentation';
import { decodeEpicBootstrapTransitionQueryV2 } from '../../application/orchestrations';

const proposal = JSON.parse(
  readFileSync(
    resolve(
      'src-tauri/src/orchestration/fixtures/orchestration-native-query-v2/valid-proposal.json',
    ),
    'utf8',
  ),
);
const initiatedEpic = JSON.parse(
  readFileSync(
    resolve(
      'src-tauri/src/orchestration/fixtures/orchestration-native-query-v2/valid-initiated-epic.json',
    ),
    'utf8',
  ),
);

describe('Tauri orchestration native query connector', () => {
  it('uses the native command and re-queries a proposal source after missed updates', async () => {
    let calls = 0;
    const client = createTauriOrchestrationNativeQueryClient(async <T>(command: string) => {
      expect(command).toBe('load_orchestration_native_query');
      calls += 1;
      return proposal as T;
    });
    const source = createNativeEpicPlanProposalSource(client, 'epic-planning-draft-fixture');
    await source.refresh();
    expect(source.getSnapshot()).toMatchObject({
      kind: 'available',
      suggestedEpicName: 'Suggested Epic fixture',
    });
    await source.refresh();
    expect(calls).toBe(2);
  });

  it('does not turn a planning draft into an accepted orchestration root', async () => {
    const orchestration = createNativeQueryOrchestrationClient({
      async load() {
        return createTauriOrchestrationNativeQueryClient(async <T>() => proposal as T).load();
      },
    });
    await expect(orchestration.load()).resolves.toEqual({
      kind: 'empty',
      reason: 'No accepted Epic orchestration has been recorded.',
    });
  });

  it('restores an initiated native-v2 Epic through canonical composition into the overview', async () => {
    const native = createTauriOrchestrationNativeQueryClient(async <T>() => initiatedEpic as T);
    const orchestration = createNativeQueryOrchestrationClient(native);

    const result = await orchestration.load();

    expect(result.kind).toBe('ready');
    if (result.kind !== 'ready') throw new Error('expected durable initiated Epic');
    const overview = presentProductOrchestrations(result.readModels);
    expect(overview.epics).toEqual([
      expect.objectContaining({
        id: 'epic-fixture',
        name: 'Suggested Epic fixture',
        plan: expect.objectContaining({
          items: [
            expect.objectContaining({
              id: 'sprint-fixture',
              name: 'Sprint fixture',
              purpose: 'Move fixture forward.',
              status: 'not_started',
            }),
            expect.objectContaining({
              id: 'sprint-fixture-2',
              name: 'Second Sprint fixture',
              purpose: 'Move second fixture forward.',
              status: 'not_started',
            }),
          ],
        }),
      }),
    ]);
  });

  it('joins strict transition-v2 only at canonical composition and clears success on refresh failure', async () => {
    const native = createTauriOrchestrationNativeQueryClient(async <T>() => initiatedEpic as T);
    const epic = initiatedEpic.initiatedEpics[0];
    const attempt = {
      attemptId: 'attempt-0',
      ordinal: 0,
      agentSessionId: 'bootstrap-session',
      agentInvocationId: 'bootstrap-invocation',
      launchedAt: 't',
      lifecycleStatus: 'running',
      lifecycleObservedAt: 't',
      semanticCompletionFactId: null,
      semanticCompletedAt: null,
      retryDisposition: 'active',
      retryReason: null,
      retryAttemptId: null,
      acceptedAt: null,
    };
    const transition = decodeEpicBootstrapTransitionQueryV2({
      contract: 'epic-bootstrap-transition-query/v2',
      schemaVersion: 2,
      transitions: [
        {
          initiationId: epic.initiationId,
          epicId: epic.epicId,
          preparationId: 'preparation',
          preparedRoot: 'root',
          approvedPlanPath: 'plan',
          manifestPath: 'manifest',
          overviewPath: 'overview',
          runnerBriefPath: 'brief',
          bootstrapSessionId: 'bootstrap-session',
          bootstrapInvocationId: 'bootstrap-invocation',
          preparedAt: 't',
          bootstrapSessionCreatedAt: 't',
          bootstrapLaunchedAt: 't',
          bootstrapLifecycleStatus: 'running',
          bootstrapLifecycleObservedAt: 't',
          semanticCompletionFactId: null,
          semanticCompletedAt: null,
          materialAcceptedAt: null,
          runnerSessionId: 'runner-session',
          runnerInvocationId: 'runner-invocation',
          runnerSessionCreatedAt: null,
          runnerLaunchedAt: null,
          runnerLifecycleStatus: null,
          runnerLifecycleObservedAt: null,
          currentAttemptId: 'attempt-0',
          retryState: 'active',
          blockedReason: null,
          acceptedAttemptId: null,
          bootstrapAttempts: [attempt],
        },
      ],
    });
    let available = true;
    const orchestration = createNativeQueryOrchestrationClient(native, {
      load: async () => {
        if (!available) throw new Error('transition unavailable');
        return transition;
      },
    });
    const ready = await orchestration.load();
    expect(ready.kind).toBe('ready');
    if (ready.kind === 'ready')
      expect(ready.readModels.epics[0]?.bootstrapTransition).toMatchObject({
        kind: 'bootstrap_running',
      });
    available = false;
    await expect(orchestration.load()).resolves.toEqual({
      kind: 'unavailable',
      reason: 'The durable orchestration query is unavailable.',
    });
  });

  it('publishes unavailable on an initial or later authoritative refresh failure', async () => {
    let fail = true;
    const source = createNativeEpicPlanProposalSource(
      {
        async load() {
          if (fail) throw new Error('native detail');
          return createTauriOrchestrationNativeQueryClient(async <T>() => proposal as T).load();
        },
      },
      'epic-planning-draft-fixture',
    );
    const changes: string[] = [];
    source.subscribe(() => changes.push(source.getSnapshot().kind));
    await expect(source.refresh()).resolves.toBeUndefined();
    expect(source.getSnapshot()).toEqual({
      kind: 'unavailable',
      reason: 'The durable Epic Plan Proposal could not be refreshed.',
    });
    fail = false;
    await source.refresh();
    expect(source.getSnapshot()).toMatchObject({ kind: 'available' });
    fail = true;
    await source.refresh();
    expect(source.getSnapshot()).toEqual({
      kind: 'unavailable',
      reason: 'The durable Epic Plan Proposal could not be refreshed.',
    });
    expect(changes).toEqual(['unavailable', 'available', 'unavailable']);
  });
});
