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
const reviewSprintId = 'sprint-parallel-review';
const reviewSprintSessionId = 'recorded-sprint-parallel-review';

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
const reviewSprintTranscript = transcript(
  reviewSprintSessionId,
  'Recorded parallel review Sprint',
  'Recorded mixed-state review composition only; no live Sprint was started.',
);
const controlPlannerDetails = lifecycleSession(
  'recorded-session-planner-r4-integration',
  'Recorded planner R4 integration',
  [['recorded-planner-r4-integration-scope', 'Recorded the Plan and Work Unit detail scope.']],
);
const ecs2eHandlerDetails = lifecycleSession(
  'recorded-session-WU-ECS2E',
  'Recorded WU-ECS2E Work Unit handler',
  [
    [
      'recorded-handler-WU-ECS2E-first-return',
      'Returned the first recorded detail-surface implementation.',
    ],
    [
      'recorded-handler-WU-ECS2E-first-review',
      'Reviewed the first return and requested a focused correction.',
    ],
    ['recorded-handler-WU-ECS2E-reprompt', 'Recorded the bounded correction request.'],
    [
      'recorded-handler-WU-ECS2E-second-return',
      'Returned the corrected detail-surface implementation.',
    ],
    ['recorded-handler-WU-ECS2E-acceptance', 'Accepted the corrected result.'],
  ],
);
const rdPlannerDetails = lifecycleSession(
  'recorded-planner-rd-r2',
  'Recorded review Sprint Planner',
  [
    [
      'recorded-planner-rd-r2-scope',
      'Recorded the relationship-model scope and its Work Unit membership.',
    ],
  ],
);
const rd1HandlerDetails = lifecycleSession(
  'recorded-handler-WU-RD1',
  'Relationship Work Unit handler',
  [
    ['recorded-handler-WU-RD1-launch', 'Recorded the first worker launch.'],
    ['recorded-handler-WU-RD1-first-review', 'Requested a focused correction.'],
    ['recorded-handler-WU-RD1-reprompt', 'Recorded the bounded correction request.'],
    ['recorded-handler-WU-RD1-second-review', 'Accepted the corrected result.'],
    ['recorded-handler-WU-RD1-merge', 'Recorded integration into the review checkpoint.'],
    ['recorded-handler-WU-RD1-completion', 'Recorded Work Unit completion.'],
  ],
);
const rd1WorkerDetails = lifecycleSession(
  'recorded-worker-WU-RD1',
  'Relationship implementation worker',
  [
    ['recorded-worker-WU-RD1-first-work', 'Returned the initial relationship model.'],
    ['recorded-worker-WU-RD1-renewed-work', 'Returned the corrected relationship model.'],
  ],
);
/** Recorded Agent Session inputs used by the embedded composition in the app-mounted demo. */
export const recordedAgentSessionDetails: readonly AgentSessionDetailsDto[] = [
  session(
    epicRunnerSessionId,
    'Orientation discovery handler',
    'Recorded development presentation only; no runtime continuation was initiated.',
  ),
  session(
    reviewSprintSessionId,
    'Recorded parallel review Sprint',
    'Recorded mixed-state review composition only; no live Sprint was started.',
  ),
  rdPlannerDetails,
  rd1HandlerDetails,
  rd1WorkerDetails,
  ...(['WU-RD2', 'WU-RD3'] as const).map((workUnitId) =>
    session(
      `recorded-worker-${workUnitId}`,
      `Recorded ${workUnitId} worker`,
      'Recorded mixed-state worker conversation; no live task was started.',
    ),
  ),
  session(
    sprintSessionId,
    'Sprint control surface discovery',
    'Recorded development facts are displayed through the canonical product composition.',
  ),
  controlPlannerDetails,
  ecs2eHandlerDetails,
  session(
    'recorded-independent-research',
    'Independent product research',
    'Recorded independent Agent Session with no Epic association.',
  ),
  ...['WU-ECS2B', 'WU-ECS2C', 'WU-ECS2D', 'WU-ECS3'].map((workUnitId) =>
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
    { id: 'handler', kind: 'worker', label: 'Work Unit handler', workUnitId: 'WU-ECS2E' },
  ],
  sharedStart: [step('ready', 'sprint', 'ready_scope', 'ready', 'Recorded ready scope')],
  workUnitLanes: [
    {
      id: 'recorded-lane',
      workUnitId: 'WU-ECS2E',
      title: 'Recorded review lane',
      initiatorActorId: 'planner',
      workerActorId: 'handler',
      steps: [
        step(
          'handler-return',
          'handler',
          'worker_return',
          'first_return',
          'Recorded handler return',
        ),
        step(
          'handler-review',
          'handler',
          'initiator_review',
          'first_review',
          'Recorded handler review',
        ),
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
      fromStepId: 'handler-return',
      toStepId: 'handler-review',
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
            sessionId: controlPlannerDetails.session.id,
            title: controlPlannerDetails.session.title,
            transcript: projectAgentSessionTranscript(controlPlannerDetails),
          },
        ],
        workUnitSessions: [
          {
            sessionId: ecs2eHandlerDetails.session.id,
            title: ecs2eHandlerDetails.session.title,
            workUnitId: 'WU-ECS2E',
            role: 'handler',
            transcript: projectAgentSessionTranscript(ecs2eHandlerDetails),
          },
          ...['WU-ECS2B', 'WU-ECS2C', 'WU-ECS2D', 'WU-ECS3'].map(
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
        ],
        plannerActivityWorkflows: [recordedPlanWorkflowAdjunct],
      },
    },
    [reviewSprintId]: {
      agentSession: {
        sessionId: reviewSprintSessionId,
        title: 'Recorded parallel review Sprint',
      },
      workspaceAdjunct: {
        agentSession: {
          sessionId: reviewSprintSessionId,
          title: 'Recorded parallel review Sprint',
          transcript: reviewSprintTranscript,
        },
        plannerActivitySessions: [
          {
            sessionId: rdPlannerDetails.session.id,
            title: rdPlannerDetails.session.title,
            transcript: projectAgentSessionTranscript(rdPlannerDetails),
          },
        ],
        workUnitSessions: [
          {
            sessionId: rd1HandlerDetails.session.id,
            title: rd1HandlerDetails.session.title,
            workUnitId: 'WU-RD1',
            role: 'handler',
            transcript: projectAgentSessionTranscript(rd1HandlerDetails),
          },
          {
            sessionId: rd1WorkerDetails.session.id,
            title: rd1WorkerDetails.session.title,
            workUnitId: 'WU-RD1',
            role: 'worker',
            transcript: projectAgentSessionTranscript(rd1WorkerDetails),
          },
          ...(['WU-RD2', 'WU-RD3'] as const).map(
            (workUnitId): WorkUnitAgentSessionPresentation => ({
              sessionId: `recorded-worker-${workUnitId}`,
              title: `Recorded ${workUnitId} worker`,
              workUnitId,
              role: 'worker',
              transcript: transcript(
                `recorded-worker-${workUnitId}`,
                `Recorded ${workUnitId} worker`,
                'Recorded mixed-state worker conversation; no live task was started.',
              ),
            }),
          ),
        ],
        plannerActivityWorkflows: [],
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

function lifecycleSession(
  id: string,
  title: string,
  turns: readonly (readonly [string, string])[],
): AgentSessionDetailsDto {
  const details = session(id, title, turns[0]?.[1] ?? 'Recorded lifecycle turn.');
  return {
    ...details,
    invocations: turns.map(([invocationId, response], index) => ({
      invocation: {
        ...details.invocations[0].invocation,
        id: invocationId,
        submittedText: `Recorded lifecycle step ${index + 1}`,
        createdAt: new Date(Date.parse(time) + index * 1_000).toISOString(),
        updatedAt: new Date(Date.parse(time) + index * 1_000).toISOString(),
      },
      events: [
        {
          ...details.invocations[0].events[0],
          id: `${invocationId}-response`,
          invocationId,
          normalized: {
            ...details.invocations[0].events[0].normalized!,
            text: response,
          },
          recordedAt: new Date(Date.parse(time) + index * 1_000).toISOString(),
        },
      ],
    })),
  };
}
