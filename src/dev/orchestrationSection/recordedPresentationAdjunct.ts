/** Narrow recorded-only display data keyed to canonical identities. It carries no product facts. */
import type { AgentSessionDetailsDto, AgentRuntimeEventDto } from '../../application/agentSessions';
import type { RecordedPlanWorkflowV1 } from '../../application/orchestrations/recordedPlanWorkflow';
import type { RecordedPresentationAdjunct } from '../../app/orchestrationPresentation';
import type { WorkUnitAgentSessionPresentation } from '../../features/orchestrations/orchestrationModel';
import {
  projectAgentSessionTranscript,
  selectLatestFinalAgentResponseRange,
} from '../../features/agentSessions';

const time = '2026-07-15T09:00:00.000Z';
const epicRunnerSessionId = 'recorded-epic-runner-manual-continuation-ready';
const sprintId = 'sprint-control-surface';
const sprintSessionId = 'recorded-sprint-control-surface-discovery';

const transcript = (id: string, title: string, response: string) =>
  projectAgentSessionTranscript(session(id, title, response));
const epicRunnerTranscript = transcript(
  epicRunnerSessionId,
  'Orientation discovery handler',
  'Recorded development presentation only; no runtime continuation was initiated.',
);
const sprintTranscript = transcript(
  sprintSessionId,
  'Sprint control surface discovery',
  'Recorded development facts are displayed through the canonical product composition.',
);

/** Recorded Agent Session inputs used by the embedded composition in the app-mounted demo. */
export const recordedAgentSessionDetails: readonly AgentSessionDetailsDto[] = [
  session(
    epicRunnerSessionId,
    'Orientation discovery handler',
    'Recorded development presentation only; no runtime continuation was initiated.',
  ),
  session(
    sprintSessionId,
    'Sprint control surface discovery',
    'Recorded development facts are displayed through the canonical product composition.',
  ),
  session(
    'recorded-session-planner-r4-integration',
    'Recorded planner R4 integration',
    'Recorded planner session; no planning runtime was started.',
  ),
  session(
    'recorded-session-reviewer-WU-ECS2E',
    'Recorded reviewer WU-ECS2E',
    'Recorded reviewer session; no review command was sent.',
  ),
  ...['WU-ECS2B', 'WU-ECS2C', 'WU-ECS2E', 'WU-ECS2D', 'WU-ECS3'].map((workUnitId) =>
    session(
      `recorded-session-${workUnitId}`,
      `Recorded ${workUnitId} worker`,
      'Recorded worker conversation; no live task was started.',
    ),
  ),
];

export const recordedPlanWorkflowAdjunct: RecordedPlanWorkflowV1 = {
  version: 'plan-workflow/v1',
  sprintPlannerActivityId: 'planner-r4-integration',
  scopeSummary: 'Recorded theoretical workflow display. No actual launch is represented.',
  fixtureKind: 'recorded_theoretical',
  actors: [
    { id: 'sprint', kind: 'sprint', label: 'Sprint' },
    { id: 'planner', kind: 'planner', label: 'Planner' },
    { id: 'worker', kind: 'worker', label: 'Recorded worker', workUnitId: 'WU-ECS2E' },
  ],
  sharedStart: [step('ready', 'sprint', 'ready_scope', 'ready', 'Recorded ready scope')],
  workUnitLanes: [
    {
      id: 'recorded-lane',
      workUnitId: 'WU-ECS2E',
      title: 'Recorded review lane',
      initiatorActorId: 'planner',
      workerActorId: 'worker',
      steps: [
        step('worker-return', 'worker', 'worker_return', 'first_return', 'Recorded worker return'),
        step('planner-review', 'planner', 'initiator_review', 'first_review', 'Recorded review'),
      ],
    },
  ],
  sharedCompletion: [
    step('outcome', 'sprint', 'sprint_outcome', 'sprint_return', 'Recorded Sprint outcome'),
  ],
  interactions: [
    {
      id: 'recorded-return',
      kind: 'return',
      fromStepId: 'worker-return',
      toStepId: 'planner-review',
    },
  ],
};

/** The shape deliberately excludes workspace, lifecycle, continuation, and all semantic collections. */
export const recordedPresentationAdjunct: RecordedPresentationAdjunct = {
  epic: {
    epicRunnerSession: {
      sessionId: epicRunnerSessionId,
      title: 'Orientation discovery handler',
      transcript: epicRunnerTranscript,
      latestAgentTurnRange: selectLatestFinalAgentResponseRange(epicRunnerTranscript)!,
    },
  },
  sprints: {
    [sprintId]: {
      agentSession: { sessionId: sprintSessionId, title: 'Sprint control surface discovery' },
      workspaceAdjunct: {
        agentSession: {
          sessionId: sprintSessionId,
          title: 'Sprint control surface discovery',
          transcript: sprintTranscript,
        },
        plannerActivitySessions: [
          {
            sessionId: 'recorded-session-planner-r4-integration',
            title: 'Recorded planner R4 integration',
            transcript: transcript(
              'recorded-session-planner-r4-integration',
              'Recorded planner R4 integration',
              'Recorded planner session; no planning runtime was started.',
            ),
          },
        ],
        workUnitSessions: ['WU-ECS2B', 'WU-ECS2C', 'WU-ECS2E', 'WU-ECS2D', 'WU-ECS3'].map(
          (workUnitId): WorkUnitAgentSessionPresentation => ({
            sessionId: `recorded-session-${workUnitId}`,
            title: `Recorded ${workUnitId} worker`,
            workUnitId,
            role: 'worker' as const,
            transcript: transcript(
              `recorded-session-${workUnitId}`,
              `Recorded ${workUnitId} worker`,
              'Recorded worker conversation; no live task was started.',
            ),
          }),
        ),
        plannerActivityWorkflows: [recordedPlanWorkflowAdjunct],
      },
    },
  },
};

function step(
  id: string,
  actorId: string,
  kind: RecordedPlanWorkflowV1['sharedStart'][number]['kind'],
  phase: RecordedPlanWorkflowV1['sharedStart'][number]['phase'],
  title: string,
) {
  return {
    id,
    actorId,
    kind,
    phase,
    title,
    summary: `${title}. Recorded/theoretical display data only.`,
  };
}

function session(id: string, title: string, response: string): AgentSessionDetailsDto {
  const invocationId = `${id}-recorded-turn`;
  const event: AgentRuntimeEventDto = {
    id: `${invocationId}-response`,
    invocationId,
    sequence: 1,
    source: 'stdout',
    rawPayload: { recorded: true },
    normalized: {
      kind: 'agent_message',
      text: response,
      externalContextId: null,
      usage: null,
      details: { role: 'final' },
    },
    recordedAt: time,
  };
  return {
    session: {
      id,
      title,
      availability: 'available',
      runtimeBinding: {
        externalContextId: `recorded-thread-${id}`,
        runtimeVersion: 'recorded-adjunct',
      },
      workingDirectory: 'C:/recorded/development-adjunct',
      requestedOptions: { model: null, sandbox: null },
      createdAt: time,
      updatedAt: time,
    },
    invocations: [
      {
        invocation: {
          id: invocationId,
          sessionId: id,
          submittedText: 'Recorded development display',
          inputProvenance: 'user',
          status: 'completed',
          requestedOptions: { model: null, sandbox: null },
          effectiveOptions: null,
          startedAt: time,
          completedAt: time,
          exitCode: 0,
          signal: null,
          runtimeError: null,
          diagnostics: [],
          createdAt: time,
          updatedAt: time,
        },
        events: [event],
      },
    ],
  };
}
