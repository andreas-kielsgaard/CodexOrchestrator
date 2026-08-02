import {
  ORCHESTRATION_ROLE_REPORTS_V1,
  type OrchestrationRoleReportContractsV1,
} from './orchestrationRoleReports';

const TOOL_ROLES = {
  record_sprint_plan: 'sprint_runner',
  record_sprint_oversight: 'epic_runner',
  record_work_slice_plan: 'work_slice_planner',
  report_handler_activity: 'work_unit_handler',
  report_worker_activity: 'work_unit_implementer',
} as const;

/** Contract-only decoder. Cross-record product authority is validated by the read composer. */
export function decodeOrchestrationRoleReportContractsV1(
  value: unknown,
): OrchestrationRoleReportContractsV1 {
  rejectTranscriptAuthority(value);
  const root = object(value, 'role report contracts');
  if (root.version !== ORCHESTRATION_ROLE_REPORTS_V1) fail('version is invalid');
  const reports = array(root.reports, 'reports');
  const ids = new Set<string>();
  for (const value of reports) {
    const report = object(value, 'role report');
    const reportId = string(report.reportId, 'reportId');
    if (ids.has(reportId)) fail('report identities must be unique');
    ids.add(reportId);
    const toolName = string(report.toolName, 'toolName');
    const role = string(report.agentRole, 'agentRole');
    if (toolName === 'record_lifecycle_transition') {
      oneOf(role, FIVE_ROLES, 'lifecycle transition role');
      oneOf(
        report.subjectKind,
        ['sprint', 'work_slice_planning_point', 'work_unit_execution'],
        'lifecycle subject kind',
      );
      oneOf(
        report.transition,
        [
          'planned',
          'started',
          'waiting',
          'implementing',
          'returned',
          'reviewing',
          'correcting',
          'merging',
          'approved',
          'completed',
        ],
        'lifecycle transition',
      );
    } else {
      const expectedRole = TOOL_ROLES[toolName as keyof typeof TOOL_ROLES];
      if (!expectedRole || role !== expectedRole) fail('tool name must match its five-role owner');
    }
    string(report.agentSessionRefId, 'agentSessionRefId');
    string(report.provenanceId, 'provenanceId');
    validateToolPayload(report, toolName);
  }
  return value as OrchestrationRoleReportContractsV1;
}

function validateToolPayload(report: Record<string, unknown>, toolName: string) {
  if (toolName === 'record_sprint_plan') {
    string(report.sprintId, 'sprintId');
    string(report.sprintPlanRevisionId, 'sprintPlanRevisionId');
    stringArray(report.managedObjectiveIds, 'managedObjectiveIds');
    stringArray(report.concernIds, 'concernIds');
    string(report.refinementSummary, 'refinementSummary');
  } else if (toolName === 'record_sprint_oversight') {
    string(report.sprintId, 'sprintId');
    string(report.sprintPlanRevisionId, 'sprintPlanRevisionId');
    oneOf(report.decision, ['accepted', 'needs_correction'], 'oversight decision');
    string(report.summary, 'summary');
  } else if (toolName === 'record_work_slice_plan') {
    string(report.workSlicePlanningPointId, 'workSlicePlanningPointId');
    string(report.sprintPlanRevisionId, 'sprintPlanRevisionId');
    stringArray(report.workUnitScopeIds, 'workUnitScopeIds');
    array(report.analysisItems, 'analysisItems').forEach((value) => {
      const item = object(value, 'analysis item');
      string(item.analysisItemId, 'analysisItemId');
      string(item.text, 'analysis text');
      stringArray(item.linkedWorkUnitScopeIds, 'linkedWorkUnitScopeIds');
    });
    const dependencyIds = new Set<string>();
    array(report.dependencies, 'dependencies').forEach((value) => {
      const dependency = object(value, 'dependency');
      const dependencyId = string(dependency.dependencyId, 'dependencyId');
      if (dependencyIds.has(dependencyId)) fail('dependency identities must be unique');
      dependencyIds.add(dependencyId);
      const targetScopeId = string(dependency.toWorkUnitScopeId, 'toWorkUnitScopeId');
      const kind = oneOf(
        dependency.kind,
        ['functional_output', 'shared_resource_exclusion', 'merge_join'],
        'dependency kind',
      );
      string(dependency.label, 'dependency label');
      if (kind === 'shared_resource_exclusion')
        string(dependency.sharedResourceKey, 'sharedResourceKey');
      if (kind === 'merge_join') {
        const joinSemantics = oneOf(
          dependency.joinSemantics,
          ['merged_result', 'independent_prerequisites'],
          'join semantics',
        );
        if (joinSemantics === 'independent_prerequisites') {
          if (dependency.fromWorkUnitScopeId !== undefined)
            fail('independent prerequisite joins use typed inputWorkUnitScopeIds');
          const inputs = strings(dependency.inputWorkUnitScopeIds, 'inputWorkUnitScopeIds');
          if (inputs.length < 2) fail('independent prerequisite joins require at least two inputs');
          if (new Set(inputs).size !== inputs.length)
            fail('independent prerequisite join inputs must be unique');
          if (inputs.includes(targetScopeId))
            fail('independent prerequisite join target cannot also be an input');
        } else {
          string(dependency.fromWorkUnitScopeId, 'fromWorkUnitScopeId');
          if (dependency.inputWorkUnitScopeIds !== undefined)
            fail('merged-result joins use one typed fromWorkUnitScopeId');
        }
      } else {
        string(dependency.fromWorkUnitScopeId, 'fromWorkUnitScopeId');
        if (dependency.inputWorkUnitScopeIds !== undefined)
          fail('direct dependencies use one typed fromWorkUnitScopeId');
      }
    });
  } else if (toolName === 'report_handler_activity') {
    string(report.workUnitExecutionId, 'workUnitExecutionId');
    oneOf(
      report.activity,
      ['creating_implementer', 'reviewing', 'correcting', 'merging', 'waiting', 'approved'],
      'handler activity',
    );
    string(report.summary, 'summary');
    string(report.lifecycleEntryId, 'lifecycleEntryId');
  } else if (toolName === 'report_worker_activity') {
    string(report.workUnitExecutionId, 'workUnitExecutionId');
    oneOf(
      report.activity,
      ['waiting', 'implementing', 'correcting', 'returned', 'completed'],
      'worker activity',
    );
    string(report.summary, 'summary');
    string(report.lifecycleEntryId, 'lifecycleEntryId');
    if (report.outcome !== undefined) string(report.outcome, 'outcome');
  } else if (toolName === 'record_lifecycle_transition') {
    string(report.subjectId, 'subjectId');
    if (report.lifecycleEntryId !== undefined) string(report.lifecycleEntryId, 'lifecycleEntryId');
  }
}

const FIVE_ROLES = [
  'epic_runner',
  'sprint_runner',
  'work_slice_planner',
  'work_unit_handler',
  'work_unit_implementer',
] as const;

function rejectTranscriptAuthority(value: unknown): void {
  if (Array.isArray(value)) return value.forEach(rejectTranscriptAuthority);
  if (!value || typeof value !== 'object') return;
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    if (
      ['transcript', 'message', 'prompt', 'harness', 'route'].some((part) =>
        key.toLowerCase().includes(part),
      )
    )
      fail(`${key} cannot provide role-report authority`);
    rejectTranscriptAuthority(child);
  }
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    fail(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function array(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}
function stringArray(value: unknown, label: string) {
  strings(value, label);
}
function strings(value: unknown, label: string): string[] {
  return array(value, label).map((item) => string(item, label));
}
function string(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) fail(`${label} must be a non-empty string`);
  return value;
}
function oneOf<T extends string>(value: unknown, allowed: readonly T[], label: string): T {
  if (typeof value !== 'string' || !allowed.includes(value as T)) fail(`${label} is invalid`);
  return value as T;
}
function fail(message: string): never {
  throw new Error(`Invalid orchestration role reports: ${message}`);
}
