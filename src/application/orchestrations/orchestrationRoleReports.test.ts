import { describe, expect, it } from 'vitest';
import { ORCHESTRATION_ROLE_REPORTS_V1 } from './orchestrationRoleReports';
import { decodeOrchestrationRoleReportContractsV1 } from './orchestrationRoleReportsDecoder';

describe('orchestration role report conformance', () => {
  it('accepts explicit role-owned structured calls and dependency meanings', () => {
    const value = {
      version: ORCHESTRATION_ROLE_REPORTS_V1,
      reports: [
        {
          reportId: 'planner-report',
          toolName: 'record_work_slice_plan',
          agentRole: 'work_slice_planner',
          agentSessionRefId: 'planner-ref',
          workSlicePlanningPointId: 'point-1',
          sprintPlanRevisionId: 'revision-1',
          analysisItems: [
            {
              analysisItemId: 'analysis-1',
              text: 'Keep both lanes independent until the explicit join.',
              linkedWorkUnitScopeIds: ['scope-1', 'scope-2', 'scope-3'],
            },
          ],
          workUnitScopeIds: ['scope-1', 'scope-2', 'scope-3', 'scope-4'],
          dependencies: [
            {
              dependencyId: 'independent-prerequisites',
              inputWorkUnitScopeIds: ['scope-1', 'scope-2'],
              toWorkUnitScopeId: 'scope-3',
              kind: 'merge_join',
              label: 'Independent readiness completions',
              joinSemantics: 'independent_prerequisites',
            },
            {
              dependencyId: 'merged-result',
              fromWorkUnitScopeId: 'scope-3',
              toWorkUnitScopeId: 'scope-4',
              kind: 'merge_join',
              label: 'Join accepted outputs',
              joinSemantics: 'merged_result',
            },
          ],
          provenanceId: 'provenance-1',
        },
      ],
    } as const;

    expect(decodeOrchestrationRoleReportContractsV1(value)).toEqual(value);
  });

  it('requires typed independent prerequisite grouping with at least two unique inputs', () => {
    expect(() =>
      decodeOrchestrationRoleReportContractsV1({
        version: ORCHESTRATION_ROLE_REPORTS_V1,
        reports: [
          {
            reportId: 'planner-report',
            toolName: 'record_work_slice_plan',
            agentRole: 'work_slice_planner',
            agentSessionRefId: 'planner-ref',
            workSlicePlanningPointId: 'point-1',
            sprintPlanRevisionId: 'revision-1',
            analysisItems: [],
            workUnitScopeIds: ['scope-1', 'scope-2'],
            dependencies: [
              {
                dependencyId: 'invalid-prerequisites',
                inputWorkUnitScopeIds: ['scope-1'],
                toWorkUnitScopeId: 'scope-2',
                kind: 'merge_join',
                label: 'Incomplete grouping',
                joinSemantics: 'independent_prerequisites',
              },
            ],
            provenanceId: 'provenance-1',
          },
        ],
      }),
    ).toThrow('independent prerequisite joins require at least two inputs');
  });

  it('rejects a tool claimed by the wrong role', () => {
    expect(() =>
      decodeOrchestrationRoleReportContractsV1({
        version: ORCHESTRATION_ROLE_REPORTS_V1,
        reports: [
          {
            reportId: 'wrong-owner',
            toolName: 'report_handler_activity',
            agentRole: 'work_unit_implementer',
            agentSessionRefId: 'session-ref',
            workUnitExecutionId: 'execution-1',
            activity: 'reviewing',
            summary: 'Reviewing the return.',
            lifecycleEntryId: 'entry-1',
            provenanceId: 'provenance-1',
          },
        ],
      }),
    ).toThrow('tool name must match its five-role owner');
  });

  it('rejects transcript and route inference fields', () => {
    expect(() =>
      decodeOrchestrationRoleReportContractsV1({
        version: ORCHESTRATION_ROLE_REPORTS_V1,
        reports: [],
        transcriptInference: 'guess from prose',
      }),
    ).toThrow('transcriptInference cannot provide role-report authority');
  });
});
