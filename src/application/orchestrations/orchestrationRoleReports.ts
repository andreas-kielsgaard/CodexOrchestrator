import type { AgentSessionSemanticRole } from './orchestrationEvents';

export const ORCHESTRATION_ROLE_REPORTS_V1 = 'orchestration-role-reports/v1' as const;

export type WorkDependencyKindV1 = 'functional_output' | 'shared_resource_exclusion' | 'merge_join';

interface WorkDependencyReportBaseV1 {
  readonly dependencyId: string;
  readonly toWorkUnitScopeId: string;
  readonly label: string;
}

export type WorkDependencyReportV1 =
  | (WorkDependencyReportBaseV1 & {
      readonly kind: 'functional_output';
      readonly fromWorkUnitScopeId: string;
    })
  | (WorkDependencyReportBaseV1 & {
      readonly kind: 'shared_resource_exclusion';
      readonly fromWorkUnitScopeId: string;
      readonly sharedResourceKey: string;
    })
  | (WorkDependencyReportBaseV1 & {
      readonly kind: 'merge_join';
      readonly joinSemantics: 'merged_result';
      readonly fromWorkUnitScopeId: string;
    })
  | (WorkDependencyReportBaseV1 & {
      readonly kind: 'merge_join';
      readonly joinSemantics: 'independent_prerequisites';
      readonly inputWorkUnitScopeIds: readonly string[];
    });

export type OrchestrationRoleReportV1 =
  | {
      readonly reportId: string;
      readonly toolName: 'record_sprint_plan';
      readonly agentRole: 'sprint_runner';
      readonly agentSessionRefId: string;
      readonly sprintId: string;
      readonly sprintPlanRevisionId: string;
      readonly managedObjectiveIds: readonly string[];
      readonly concernIds: readonly string[];
      readonly refinementSummary: string;
      readonly provenanceId: string;
    }
  | {
      readonly reportId: string;
      readonly toolName: 'record_sprint_oversight';
      readonly agentRole: 'epic_runner';
      readonly agentSessionRefId: string;
      readonly sprintId: string;
      readonly sprintPlanRevisionId: string;
      readonly decision: 'accepted' | 'needs_correction';
      readonly summary: string;
      readonly provenanceId: string;
    }
  | {
      readonly reportId: string;
      readonly toolName: 'record_work_slice_plan';
      readonly agentRole: 'work_slice_planner';
      readonly agentSessionRefId: string;
      readonly workSlicePlanningPointId: string;
      readonly sprintPlanRevisionId: string;
      readonly analysisItems: readonly {
        readonly analysisItemId: string;
        readonly text: string;
        readonly linkedWorkUnitScopeIds: readonly string[];
      }[];
      readonly workUnitScopeIds: readonly string[];
      readonly dependencies: readonly WorkDependencyReportV1[];
      readonly provenanceId: string;
    }
  | {
      readonly reportId: string;
      readonly toolName: 'report_handler_activity';
      readonly agentRole: 'work_unit_handler';
      readonly agentSessionRefId: string;
      readonly workUnitExecutionId: string;
      readonly activity:
        'creating_implementer' | 'reviewing' | 'correcting' | 'merging' | 'waiting' | 'approved';
      readonly summary: string;
      readonly lifecycleEntryId: string;
      readonly provenanceId: string;
    }
  | {
      readonly reportId: string;
      readonly toolName: 'report_worker_activity';
      readonly agentRole: 'work_unit_implementer';
      readonly agentSessionRefId: string;
      readonly workUnitExecutionId: string;
      readonly activity: 'waiting' | 'implementing' | 'correcting' | 'returned' | 'completed';
      readonly summary: string;
      readonly outcome?: string;
      readonly lifecycleEntryId: string;
      readonly provenanceId: string;
    }
  | {
      readonly reportId: string;
      readonly toolName: 'record_lifecycle_transition';
      readonly agentRole: AgentSessionSemanticRole;
      readonly agentSessionRefId: string;
      readonly subjectKind: 'sprint' | 'work_slice_planning_point' | 'work_unit_execution';
      readonly subjectId: string;
      readonly transition:
        | 'planned'
        | 'started'
        | 'waiting'
        | 'implementing'
        | 'returned'
        | 'reviewing'
        | 'correcting'
        | 'merging'
        | 'approved'
        | 'completed';
      readonly lifecycleEntryId?: string;
      readonly provenanceId: string;
    };

export interface OrchestrationRoleReportContractsV1 {
  readonly version: typeof ORCHESTRATION_ROLE_REPORTS_V1;
  readonly reports: readonly OrchestrationRoleReportV1[];
}
