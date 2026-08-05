/** Recorded-only Work Unit inspection evidence for focused human review. */
import type {
  ProductReadModelsV1,
  ProductWorkUnitInspectionV1,
} from '../../application/orchestrations/productReadModels';
import type { FileReviewSource } from '../../application/fileReview';

const inspection: ProductWorkUnitInspectionV1 = {
  workUnitId: 'WU-ECS2E',
  materializationId: 'recorded-materialization-ECS-R4',
  activities: [
    {
      activityId:
        'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-1:handler-action:recorded-handler-WU-ECS2E-first-review',
      attemptId: 'WU-ECS2E-attempt-1',
      role: 'handler',
      agentSessionId: 'recorded-session-WU-ECS2E',
      invocationId: 'recorded-handler-WU-ECS2E-first-review',
      primaryStage: 'handler_action',
    },
    {
      activityId:
        'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-1:implementer-reporting:recorded-implementer-WU-ECS2E-first-return',
      attemptId: 'WU-ECS2E-attempt-1',
      role: 'implementer',
      agentSessionId: 'recorded-implementer-WU-ECS2E',
      invocationId: 'recorded-implementer-WU-ECS2E-first-return',
      primaryStage: 'implementer_reporting',
      applicationSummary: {
        owner: 'application',
        applicationEvents: [
          'submission_recorded',
          'file_evidence_recorded',
          'semantic_completion_recorded',
          'terminal_lifecycle_observed',
          'handler_review_ready',
        ],
        peerEvidenceActivityIds: [],
        mcpCallDetail: {
          owner: 'application',
          reason: 'No application-owned MCP-call detail is available for this recorded turn.',
        },
      },
    },
    {
      activityId:
        'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-2:handler-review:recorded-handler-WU-ECS2E-acceptance',
      attemptId: 'WU-ECS2E-attempt-2',
      role: 'handler',
      agentSessionId: 'recorded-session-WU-ECS2E',
      invocationId: 'recorded-handler-WU-ECS2E-acceptance',
      primaryStage: 'handler_review',
      applicationSummary: {
        owner: 'application',
        applicationEvents: ['review_delivery_persisted', 'review_judgment_recorded'],
        peerEvidenceActivityIds: [
          'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-2:implementer-reporting:recorded-implementer-WU-ECS2E-second-return',
        ],
        mcpCallDetail: {
          owner: 'application',
          reason: 'MCP-call detail is unavailable in this recorded review.',
        },
      },
    },
    {
      activityId:
        'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-2:implementer-reporting:recorded-implementer-WU-ECS2E-second-return',
      attemptId: 'WU-ECS2E-attempt-2',
      role: 'implementer',
      agentSessionId: 'recorded-implementer-WU-ECS2E',
      invocationId: 'recorded-implementer-WU-ECS2E-second-return',
      primaryStage: 'implementer_reporting',
      applicationSummary: {
        owner: 'application',
        applicationEvents: [
          'submission_recorded',
          'file_evidence_recorded',
          'semantic_completion_recorded',
          'terminal_lifecycle_observed',
          'application_acceptance_recorded',
          'handler_review_ready',
        ],
        peerEvidenceActivityIds: [],
        mcpCallDetail: {
          owner: 'application',
          reason: 'MCP-call detail is unavailable in this recorded review.',
        },
      },
    },
  ],
  fileEvidence: {
    status: 'available',
    owner: 'application',
    sourceActivityId:
      'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-2:implementer-reporting:recorded-implementer-WU-ECS2E-second-return',
    changedFiles: [
      {
        evidenceRef: 'recorded-evidence-work-unit-detail',
        fileId: 'recorded-file-work-unit-detail',
        displayName: 'src/features/orchestrations/components/WorkUnitDetailWorkspace.tsx',
        changeKind: 'modified',
        contentFingerprint: 'recorded-fingerprint-work-unit-detail',
        diffDestination: {
          status: 'available',
          owner: 'application',
          reviewId: 'recorded-work-unit-review',
          changedFileId: 'recorded-file-work-unit-detail',
        },
      },
      {
        evidenceRef: 'recorded-evidence-activity-tests',
        fileId: 'recorded-file-activity-tests',
        displayName:
          'src/features/orchestrations/components/WorkUnitDetailWorkspace.activityEvidence.test.tsx',
        changeKind: 'added',
        contentFingerprint: 'recorded-fingerprint-activity-tests',
        diffDestination: {
          status: 'unavailable',
          owner: 'application',
          reason:
            'No application-owned diff destination is available for this recorded changed file.',
        },
      },
    ],
  },
  testEvidence: {
    status: 'available',
    owner: 'application',
    sourceActivityId:
      'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-2:implementer-reporting:recorded-implementer-WU-ECS2E-second-return',
    runId: 'recorded-work-unit-review-tests',
    whatRan: 'Focused Work Unit Activity and Evidence interaction checks',
    command:
      'npm exec vitest run src/features/orchestrations/components/WorkUnitDetailWorkspace.activityEvidence.test.tsx',
    environment: 'Recorded development review fixture',
    result: 'passed',
    cases: [
      {
        caseId: 'recorded-activity-evidence-flow',
        label: 'Activity and Evidence flow',
        result: 'passed',
      },
    ],
  },
};

export const recordedWorkUnitReviewInspection = inspection;

/** Recorded-only diff material for the explicit recorded destination above. */
export function recordedWorkUnitReviewFileSource(
  reviewId: string,
  changedFileId: string,
): FileReviewSource | undefined {
  if (
    reviewId !== 'recorded-work-unit-review' ||
    changedFileId !== 'recorded-file-work-unit-detail'
  )
    return undefined;
  return {
    async load() {
      return {
        files: [
          {
            fileId: 'recorded-file-work-unit-detail',
            displayPath: 'src/features/orchestrations/components/WorkUnitDetailWorkspace.tsx',
            changeKind: 'modified',
            additions: 3,
            deletions: 1,
            content: { kind: 'text', text: 'Recorded read-only Work Unit Activity detail.' },
            hunks: [
              {
                hunkId: 'recorded-work-unit-detail:1',
                header: '@@ -1 +1 @@',
                lines: [
                  { kind: 'deletion', oldLineNumber: 1, text: 'Embedded full session workspace' },
                  { kind: 'addition', newLineNumber: 1, text: 'Selected Activity read-only turn' },
                ],
              },
            ],
          },
        ],
      };
    },
  };
}

/** Adds only recorded inspection presentation data; productive read composition is untouched. */
export function addRecordedWorkUnitReviewInspection(
  readModels: ProductReadModelsV1,
): ProductReadModelsV1 {
  return {
    ...readModels,
    epics: readModels.epics.map((epic) => ({
      ...epic,
      sprints: epic.sprints.map((sprint) =>
        sprint.sprintId !== 'sprint-control-surface'
          ? sprint
          : {
              ...sprint,
              revisionViews: sprint.revisionViews.map((view) =>
                view.sprintPlanRevisionId !== 'ECS-R4'
                  ? view
                  : {
                      ...view,
                      workUnits: view.workUnits.map((unit) =>
                        unit.workUnitId === inspection.workUnitId ? { ...unit, inspection } : unit,
                      ),
                    },
              ),
            },
      ),
    })),
  };
}
