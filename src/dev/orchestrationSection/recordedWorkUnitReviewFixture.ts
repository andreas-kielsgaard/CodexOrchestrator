/** Recorded-only Work Unit inspection evidence for focused human review. */
import type {
  ProductReadModelsV1,
  ProductWorkUnitInspectionV1,
} from '../../application/orchestrations/productReadModels';

const inspection: ProductWorkUnitInspectionV1 = {
  workUnitId: 'WU-ECS2E',
  materializationId: 'recorded-materialization-ECS-R4',
  activities: [
    {
      activityId: 'recorded-wu-ecs2e-handler-action',
      attemptId: 'WU-ECS2E-attempt-1',
      role: 'handler',
      agentSessionId: 'recorded-session-WU-ECS2E',
      invocationId: 'recorded-handler-WU-ECS2E-first-review',
      primaryStage: 'handler_action',
      applicationSummary: {
        owner: 'application',
        applicationEvents: ['submission_recorded', 'handler_review_ready'],
        peerEvidenceActivityIds: ['recorded-wu-ecs2e-missing-activity'],
        mcpCallDetail: {
          owner: 'application',
          reason: 'No application-owned MCP-call detail is available for this recorded turn.',
        },
      },
    },
    {
      activityId: 'recorded-wu-ecs2e-implementer-report',
      attemptId: 'WU-ECS2E-attempt-1',
      role: 'implementer',
      agentSessionId: 'recorded-implementer-WU-ECS2E',
      invocationId: 'recorded-implementer-WU-ECS2E-first-return',
      primaryStage: 'implementer_reporting',
      applicationSummary: {
        owner: 'application',
        applicationEvents: ['submission_recorded', 'file_evidence_recorded'],
        peerEvidenceActivityIds: [],
        mcpCallDetail: {
          owner: 'application',
          reason: 'No application-owned MCP-call detail is available for this recorded turn.',
        },
      },
    },
    {
      activityId: 'recorded-wu-ecs2e-handler-review',
      attemptId: 'WU-ECS2E-attempt-2',
      role: 'handler',
      agentSessionId: 'recorded-session-WU-ECS2E',
      invocationId: 'recorded-handler-WU-ECS2E-acceptance',
      primaryStage: 'handler_review',
      applicationSummary: {
        owner: 'application',
        applicationEvents: ['review_delivery_persisted', 'application_acceptance_recorded'],
        peerEvidenceActivityIds: ['recorded-wu-ecs2e-implementer-report'],
        mcpCallDetail: {
          owner: 'application',
          reason: 'MCP-call detail is unavailable in this recorded review.',
        },
      },
    },
    {
      activityId: 'recorded-wu-ecs2e-implementer-correction',
      attemptId: 'WU-ECS2E-attempt-2',
      role: 'implementer',
      agentSessionId: 'recorded-implementer-WU-ECS2E',
      invocationId: 'recorded-implementer-WU-ECS2E-second-return',
      primaryStage: 'implementer_reporting',
    },
  ],
  fileEvidence: {
    status: 'available',
    owner: 'application',
    sourceActivityId: 'recorded-wu-ecs2e-implementer-report',
    changedFiles: [
      {
        evidenceRef: 'recorded-evidence-work-unit-detail',
        displayName: 'src/features/orchestrations/components/WorkUnitDetailWorkspace.tsx',
        changeKind: 'modified',
        contentFingerprint: 'recorded-fingerprint-work-unit-detail',
      },
      {
        evidenceRef: 'recorded-evidence-activity-tests',
        displayName:
          'src/features/orchestrations/components/WorkUnitDetailWorkspace.activityEvidence.test.tsx',
        changeKind: 'added',
        contentFingerprint: 'recorded-fingerprint-activity-tests',
      },
    ],
  },
  testEvidence: {
    owner: 'application',
    reason: 'No application-owned test-detail evidence is available in this recorded scenario.',
  },
};

export const recordedWorkUnitReviewInspection = inspection;

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
                        unit.workUnitId === inspection.workUnitId
                          ? { ...unit, inspection }
                          : unit,
                      ),
                    },
              ),
            },
      ),
    })),
  };
}
