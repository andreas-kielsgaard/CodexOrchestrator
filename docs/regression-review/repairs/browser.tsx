import { createRoot } from 'react-dom/client';
import { App } from '../../../src/app/App';
import { createRecordedDevelopmentApplicationComposition } from '../../../src/dev/orchestrationSection/recordedOrchestrationClient';
import { repairClients } from '../../../src/features/workflowAuthoring/testFixtures';
import { repairSessionClients } from '../../../src/features/agentSessions/profileTestFixtures';
import type {
  WorkflowInstanceClient,
  WorkflowRecipeInstance,
} from '../../../src/application/workflowInstances';
import type { RepoBranchWorktreeTargetSelectorProps } from '../../../src/application/worktreeTargets';
import type { SessionEventResultDto } from '../../../src/application/sessionEvents';
import '../../../src/styles.css';

const fixture = repairClients();
const sessions = repairSessionClients();
const instances: WorkflowRecipeInstance[] = JSON.parse(
  localStorage.getItem('repair-instances') ?? '[]',
);
const ref = (kind: string, id: string) => ({ namespace: 'orchestrator.agent_sessions', kind, id });
const sent = new Set<string>(
  JSON.parse(localStorage.getItem('repair-sent-instances') ?? '[]'),
);
const results = new Map<string, SessionEventResultDto>();
const calls = { births: 0, messages: 0 };
const instanceClient: WorkflowInstanceClient = {
  list: async () => instances,
  create: async (input) => {
    calls.births++;
    const instance = {
      id: crypto.randomUUID(),
      name: input.name,
      recipe: structuredClone(
        fixture.states.find((state) => state.draft.recipeId === input.recipeId)!.active!,
      ),
      target: input.target,
      createdAt: new Date().toISOString(),
    };
    instances.push(instance);
    localStorage.setItem('repair-instances', JSON.stringify(instances));
    return instance;
  },
  load: async (id) => ({
    instance: instances.find((instance) => instance.id === id)!,
    sessions: sent.has(id)
      ? [
          {
            session: ref('session', 'session-1'),
            logicalAddress: {
              scope: { namespace: 'workflow', kind: 'instance', id },
              subject: { namespace: 'workflow', kind: 'node', id: 'author' },
            },
            running: false,
          },
        ]
      : [],
    attempts: [],
  }),
  messageNode: async ({ instanceId }) => {
    calls.messages++;
    sent.add(instanceId);
    localStorage.setItem('repair-sent-instances', JSON.stringify([...sent]));
    const result: SessionEventResultDto = {
      group: {
        eventGroupId: ref('event_group', 'group'),
        definitionRef: ref('definition', 'entry'),
        trigger: { kind: 'user_request', request: ref('request', 'user') },
        source: { kind: 'user_request', request: ref('request', 'user') },
        promptSources: [],
        createdSessionPromptSources: [],
        targetSelection: {
          target: { kind: 'exact', session: ref('session', 'session-1') },
          cardinality: 'first',
          ordering: 'newest',
          running: 'any',
          createdBy: null,
          missing: 'fail',
        },
        resolvedSessions: [ref('session', 'session-1')],
        createdSession: ref('session', 'session-1'),
        outcome: 'delivered',
        deliveryCount: 1,
      },
      deliveries: [
        {
          deliveryId: ref('delivery', 'delivery'),
          eventGroupId: ref('event_group', 'group'),
          ordinal: 1,
          targetSession: ref('session', 'session-1'),
          logicalAddress: null,
          targetCreated: true,
          promptContributions: [],
          includedCreatedSessionContributions: [],
          addressedSequence: 1,
          addressingError: null,
          outcome: { kind: 'dispatched', invocation: ref('invocation', 'invocation-next') },
        },
      ],
    };
    results.set(instanceId, result);
    return result;
  },
};
function TargetSelector({ onChange, disabled }: RepoBranchWorktreeTargetSelectorProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() =>
        onChange({
          repository: {
            id: 'fixture',
            name: 'Demo repository',
            gitCommonDirectory: 'C:/demo/.git',
          },
          branch: { id: 'review', name: 'review' },
          worktree: { id: 'demo', path: 'C:/demo/worktree' },
        })
      }
    >
      Use demo worktree
    </button>
  );
}
Object.assign(window, { repairs: { fixture, instances, calls } });
createRoot(document.getElementById('root')!).render(
  <App
    {...createRecordedDevelopmentApplicationComposition({ initialSurface: 'workflows' })}
    workflowAuthoringClient={fixture.authoring}
    executionConfigurationClient={fixture.configuration}
    workflowInstanceClient={instanceClient}
    workflowTargetSelector={TargetSelector}
    agentSessionClient={sessions.sessions}
    agentSessionProfileClient={sessions.profiles}
    conversationHarnessManagementSource={undefined}
  />,
);
