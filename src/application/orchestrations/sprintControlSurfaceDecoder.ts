import {
  SPRINT_EXECUTION_SNAPSHOT_V1,
  SPRINT_PLANNER_OUTPUT_V1,
  type SprintExecutionSnapshotV1,
  type SprintPlannerOutputV1,
} from './sprintControlSurfaceCompatibility';

/** Decodes only provisional discovery compatibility input and validates its references. */
export function decodeSprintPlannerOutputV1(value: unknown): SprintPlannerOutputV1 {
  const output = record(value, 'planner output');
  equal(output.version, SPRINT_PLANNER_OUTPUT_V1, 'version');
  string(output.epicId, 'epicId');
  const sprint = record(output.sprint, 'sprint');
  fields(sprint, ['id', 'title', 'summary', 'details']);
  const sprintPlan = record(output.sprintPlan, 'sprint plan');
  fields(sprintPlan, ['id', 'sprintId']);
  if (sprintPlan.sprintId !== sprint.id) fail('sprint plan belongs to a different sprint');

  const revisions = list(output.planRevisions, 'planRevisions');
  const activities = list(output.sprintPlannerActivities, 'sprintPlannerActivities');
  const groups = list(output.parallelGroups, 'parallelGroups');
  const changes = list(output.planChanges, 'planChanges');
  const units = list(output.workUnits, 'workUnits');
  const concerns = list(output.concerns, 'concerns');
  const gates = list(output.gates, 'gates');
  const documents = list(output.documents, 'documents');
  const revisionIds = identifiers(revisions, 'plan revision');
  const activityIds = identifiers(activities, 'Sprint Planner Activity');
  const groupIds = identifiers(groups, 'parallel group');
  const unitIds = identifiers(units, 'work unit');
  const concernIds = identifiers(concerns, 'concern');
  const gateIds = identifiers(gates, 'gate');
  identifiers(changes, 'plan change');
  identifiers(documents, 'document');

  const revisionNumbers = new Set<number>();
  revisions.forEach((candidate) => {
    const revision = record(candidate, 'plan revision');
    fields(revision, ['id', 'summary']);
    positiveInteger(revision.revision, 'revision');
    if (revisionNumbers.has(revision.revision as number)) fail('duplicate plan revision number');
    revisionNumbers.add(revision.revision as number);
    references(list(revision.workUnitIds, 'workUnitIds'), unitIds, 'plan revision work unit');
    optionalReference(revision.supersedesPlanRevisionId, revisionIds, 'superseded revision');
  });
  validateSupersession(revisions);

  const ownedByRevision = new Map<string, Set<string>>();
  activities.forEach((candidate) => {
    const activity = record(candidate, 'Sprint Planner Activity');
    fields(activity, ['id', 'title', 'purpose', 'planRevisionId']);
    reference(activity.planRevisionId, revisionIds, 'Sprint Planner Activity plan revision');
    const revision = revisions.find(
      (item) => record(item, 'revision').id === activity.planRevisionId,
    );
    const revisionUnits = new Set(list(record(revision, 'revision').workUnitIds, 'workUnitIds'));
    const owned = ownedByRevision.get(activity.planRevisionId as string) ?? new Set<string>();
    list(activity.workUnitIds, 'workUnitIds').forEach((id) => {
      reference(id, unitIds, 'Sprint Planner Activity work unit');
      if (!revisionUnits.has(id))
        fail('Sprint Planner Activity owns a work unit outside its revision');
      if (owned.has(id as string))
        fail('work unit cannot be owned by multiple Sprint Planner Activities in one revision');
      owned.add(id as string);
    });
    ownedByRevision.set(activity.planRevisionId as string, owned);
    references(
      list(activity.userReviewGateIds, 'userReviewGateIds'),
      gateIds,
      'Sprint Planner Activity gate',
    );
  });

  concerns.forEach((candidate) => {
    const concern = record(candidate, 'concern');
    fields(concern, ['id', 'title', 'summary', 'details']);
    references(
      list(concern.requiredWorkUnitIds, 'requiredWorkUnitIds'),
      unitIds,
      'concern work unit',
    );
  });

  gates.forEach((candidate) => {
    const gate = record(candidate, 'gate');
    fields(gate, ['id']);
    literal(gate.kind, ['user', 'planner', 'replan', 'convergence'], 'gate kind');
    identifiers(list(gate.specRevisions, 'gate specRevisions'), 'gate spec revision');
    list(gate.specRevisions, 'gate specRevisions').forEach((item) => {
      const spec = record(item, 'gate spec revision');
      positiveInteger(spec.revision, 'gate revision');
      fields(spec, ['id', 'summary']);
      reference(spec.planRevisionId, revisionIds, 'gate plan revision');
      references(list(spec.requiresWorkUnitIds, 'requiresWorkUnitIds'), unitIds, 'gate work unit');
      references(list(spec.requiresGateIds, 'requiresGateIds'), gateIds, 'gate dependency');
    });
  });

  groups.forEach((candidate) => {
    const group = record(candidate, 'parallel group');
    fields(group, ['id', 'rationale', 'planRevisionId']);
    reference(group.planRevisionId, revisionIds, 'parallel group revision');
    const members = list(group.workUnitIds, 'workUnitIds');
    references(members, unitIds, 'parallel group work unit');
    const revision = revisions.find((item) => record(item, 'revision').id === group.planRevisionId);
    const revisionUnits = new Set(list(record(revision, 'revision').workUnitIds, 'workUnitIds'));
    if (members.some((id) => !revisionUnits.has(id)))
      fail('parallel group member is absent from its plan revision');
  });

  const specIds = new Set<string>();
  units.forEach((candidate) => {
    const unit = record(candidate, 'work unit');
    fields(unit, ['id', 'shortTitle', 'summary', 'details']);
    references(list(unit.concernIds, 'concernIds'), concernIds, 'work unit concern');
    if (unit.parallelGroupId !== undefined) {
      reference(unit.parallelGroupId, groupIds, 'work unit parallel group');
      const group = groups.find((item) => record(item, 'group').id === unit.parallelGroupId);
      if (!list(record(group, 'group').workUnitIds, 'workUnitIds').includes(unit.id))
        fail('work unit is absent from its parallel group');
    }
    list(unit.dependencies, 'dependencies').forEach((candidate) => {
      const dependency = record(candidate, 'dependency');
      const kind = literal(dependency.kind, ['hard', 'preferred', 'gated'], 'dependency kind');
      if (kind === 'gated') {
        reference(dependency.gateId, gateIds, 'dependency gate');
        optionalReference(dependency.workUnitId, unitIds, 'dependency work unit');
      } else {
        reference(dependency.workUnitId, unitIds, 'dependency work unit');
        if (dependency.gateId !== undefined) fail('only a gated dependency may reference a gate');
      }
    });
    const numbers = new Set<number>();
    list(unit.specRevisions, 'specRevisions').forEach((candidate) => {
      const spec = record(candidate, 'spec revision');
      fields(spec, ['id', 'summary', 'details']);
      if (specIds.has(spec.id as string)) fail('duplicate spec revision');
      specIds.add(spec.id as string);
      positiveInteger(spec.revision, 'spec revision');
      if (numbers.has(spec.revision as number)) fail('duplicate spec revision number');
      numbers.add(spec.revision as number);
      reference(spec.planRevisionId, revisionIds, 'spec plan revision');
      const revision = revisions.find(
        (item) => record(item, 'revision').id === spec.planRevisionId,
      );
      if (!list(record(revision, 'revision').workUnitIds, 'workUnitIds').includes(unit.id))
        fail('spec revision belongs to a plan that does not contain its work unit');
    });
  });
  revisions.forEach((candidate) => {
    const revision = record(candidate, 'revision');
    list(revision.workUnitIds, 'workUnitIds').forEach((unitId) => {
      const unit = record(
        units.find((item) => record(item, 'unit').id === unitId),
        'unit',
      );
      if (
        !list(unit.specRevisions, 'specRevisions').some(
          (item) => record(item, 'spec').planRevisionId === revision.id,
        )
      )
        fail('plan revision work unit has no matching spec revision');
    });
  });

  changes.forEach((candidate) => {
    const change = record(candidate, 'plan change');
    fields(change, [
      'id',
      'summary',
      'priorPlanRevisionId',
      'resultingPlanRevisionId',
      'priorSprintPlannerActivityId',
      'resultingSprintPlannerActivityId',
    ]);
    equal(change.source, 'sprint_conversation', 'plan change source');
    reference(change.priorPlanRevisionId, revisionIds, 'prior revision');
    reference(change.resultingPlanRevisionId, revisionIds, 'resulting revision');
    reference(change.priorSprintPlannerActivityId, activityIds, 'prior Sprint Planner Activity');
    reference(
      change.resultingSprintPlannerActivityId,
      activityIds,
      'resulting Sprint Planner Activity',
    );
    const resulting = record(
      revisions.find((item) => record(item, 'revision').id === change.resultingPlanRevisionId),
      'revision',
    );
    if (resulting.supersedesPlanRevisionId !== change.priorPlanRevisionId)
      fail('plan change must link a direct supersession');
  });

  documents.forEach((candidate) => {
    const document = record(candidate, 'document');
    fields(document, ['id', 'title', 'sprintPlannerActivityId', 'planRevisionId']);
    literal(document.kind, ['plan', 'brief', 'decision', 'handoff'], 'document kind');
    reference(document.sprintPlannerActivityId, activityIds, 'document Sprint Planner Activity');
    reference(document.planRevisionId, revisionIds, 'document plan revision');
    timestamp(document.recordedAt, 'document recordedAt');
  });
  return value as SprintPlannerOutputV1;
}

export function decodeSprintExecutionSnapshotV1(
  value: unknown,
  planner: SprintPlannerOutputV1,
): SprintExecutionSnapshotV1 {
  const snapshot = record(value, 'execution snapshot');
  equal(snapshot.version, SPRINT_EXECUTION_SNAPSHOT_V1, 'version');
  if (snapshot.sprintId !== planner.sprint.id)
    fail('execution snapshot belongs to a different sprint');
  const revisionIds = new Set(planner.planRevisions.map(({ id }) => id));
  const unitIds = new Set(planner.workUnits.map(({ id }) => id));
  const gateIds = new Set(planner.gates.map(({ id }) => id));
  const concernIds = new Set(planner.concerns.map(({ id }) => id));
  reference(snapshot.activePlanRevisionId, revisionIds, 'active plan revision');
  const sessions = list(snapshot.agentSessions, 'agentSessions');
  const sessionIds = identifiers(sessions, 'agent session');
  sessions.forEach((candidate) => {
    const session = record(candidate, 'agent session');
    fields(session, ['id', 'title']);
    const role = literal(
      session.role,
      ['sprint', 'work_unit_handler', 'work_unit_worker'],
      'agent session role',
    );
    if (role === 'sprint' && session.workUnitId !== undefined)
      fail('sprint agent session cannot reference a work unit');
    if (role !== 'sprint') reference(session.workUnitId, unitIds, 'agent session work unit');
  });
  const events = list(snapshot.events, 'events');
  const eventIds = identifiers(events, 'execution event');
  events.forEach((candidate) => {
    const event = record(candidate, 'event');
    fields(event, ['id', 'summary']);
    literal(
      event.kind,
      ['review', 'correction', 'acceptance', 'replan', 'blocker', 'deferred_decision'],
      'event kind',
    );
    optionalReference(event.workUnitId, unitIds, 'event work unit');
    optionalReference(event.gateId, gateIds, 'event gate');
    timestamp(event.recordedAt, 'event recordedAt');
  });
  identifiers(list(snapshot.workUnits, 'workUnits'), 'execution work unit', 'workUnitId');
  list(snapshot.workUnits, 'workUnits').forEach((candidate) => {
    const execution = record(candidate, 'execution work unit');
    const unitId = string(execution.workUnitId, 'workUnitId');
    reference(unitId, unitIds, 'execution work unit');
    const state = literal(
      execution.state,
      ['projected', 'launched', 'working', 'under_review', 'accepted', 'blocked', 'deferred'],
      'work unit state',
    );
    timestamp(execution.projectedAt, 'projectedAt');
    if (state === 'projected' && execution.actualLaunch !== undefined)
      fail('projected work unit cannot have an actual launch');
    if (!['projected', 'deferred'].includes(state) && execution.actualLaunch === undefined)
      fail('actual work unit needs an actual launch');
    if (execution.actualLaunch !== undefined) {
      const launch = record(execution.actualLaunch, 'actual launch');
      timestamp(launch.launchedAt, 'launchedAt');
      reference(launch.agentSessionId, sessionIds, 'actual launch agent session');
    }
    if (state === 'deferred') {
      reference(execution.deferredByEventId, eventIds, 'deferred event');
      const event = record(
        events.find((item) => record(item, 'event').id === execution.deferredByEventId),
        'event',
      );
      if (event.kind !== 'deferred_decision')
        fail('deferred work unit needs a deferred decision event');
    } else if (execution.deferredByEventId !== undefined)
      fail('only a deferred work unit may reference a deferred decision');
    const specs = new Set(
      planner.workUnits.find(({ id }) => id === unitId)?.specRevisions.map(({ id }) => id),
    );
    identifiers(list(execution.attempts, 'attempts'), 'attempt');
    list(execution.attempts, 'attempts').forEach((candidate) => {
      const attempt = record(candidate, 'attempt');
      fields(attempt, ['id']);
      reference(attempt.specRevisionId, specs, 'attempt spec revision');
      literal(
        attempt.outcome,
        ['working', 'returned', 'accepted', 'corrected', 'blocked'],
        'attempt outcome',
      );
      timestamp(attempt.recordedAt, 'attempt recordedAt');
      if (attempt.workerFeedback !== undefined) string(attempt.workerFeedback, 'workerFeedback');
    });
  });
  identifiers(list(snapshot.concernDecisions, 'concernDecisions'), 'concern decision', 'concernId');
  list(snapshot.concernDecisions, 'concernDecisions').forEach((candidate) => {
    const decision = record(candidate, 'concern decision');
    reference(decision.concernId, concernIds, 'decision concern');
    literal(decision.kind, ['deferred', 'accepted'], 'decision kind');
    string(decision.summary, 'decision summary');
  });
  identifiers(list(snapshot.generatedDocuments, 'generatedDocuments'), 'generated document');
  list(snapshot.generatedDocuments, 'generatedDocuments').forEach((candidate) => {
    const document = record(candidate, 'generated document');
    fields(document, ['id', 'title']);
    literal(document.kind, ['outcome', 'review', 'handoff'], 'generated document kind');
    optionalReference(document.workUnitId, unitIds, 'document work unit');
    timestamp(document.recordedAt, 'document recordedAt');
  });
  const continuation = record(snapshot.continuation, 'continuation');
  validateContinuation(continuation.sprint, 'sprint continuation');
  validateContinuation(continuation.epic, 'Epic continuation');
  return value as SprintExecutionSnapshotV1;
}

function validateContinuation(value: unknown, label: string) {
  const continuation = record(value, label);
  if (typeof continuation.automaticEnabled !== 'boolean')
    fail(`${label} automaticEnabled must be boolean`);
  if (typeof continuation.initiationObserved !== 'boolean')
    fail(`${label} initiationObserved must be boolean`);
  literal(
    continuation.status,
    ['not_ready', 'ready_for_manual', 'continuation_requested'],
    `${label} status`,
  );
  if (continuation.initiationObserved && continuation.status !== 'continuation_requested')
    fail(`${label} observed initiation needs requested status`);
}

function record(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    fail(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function list(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}
function string(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) fail(`${label} must be a non-empty string`);
  return value;
}
function fields(value: Record<string, unknown>, keys: readonly string[]) {
  keys.forEach((key) => string(value[key], key));
}
function positiveInteger(value: unknown, label: string) {
  if (!Number.isInteger(value) || Number(value) < 1) fail(`${label} must be a positive integer`);
}
function equal(value: unknown, expected: string, label: string) {
  if (value !== expected) fail(`${label} is invalid`);
}
function literal(value: unknown, allowed: readonly string[], label: string): string {
  if (typeof value !== 'string' || !allowed.includes(value)) fail(`${label} is invalid`);
  return value;
}
function identifiers(values: readonly unknown[], label: string, key = 'id'): Set<string> {
  const result = new Set<string>();
  values.forEach((value) => {
    const id = string(record(value, label)[key], `${label} ${key}`);
    if (result.has(id)) fail(`duplicate ${label} ${id}`);
    result.add(id);
  });
  return result;
}
function reference(value: unknown, ids: ReadonlySet<unknown>, label: string) {
  if (typeof value !== 'string' || !ids.has(value)) fail(`dangling ${label} reference`);
}
function references(values: readonly unknown[], ids: ReadonlySet<unknown>, label: string) {
  values.forEach((value) => reference(value, ids, label));
}
function optionalReference(value: unknown, ids: ReadonlySet<unknown>, label: string) {
  if (value !== undefined) reference(value, ids, label);
}
function timestamp(value: unknown, label: string) {
  if (typeof value !== 'string' || Number.isNaN(Date.parse(value)))
    fail(`${label} must be an ISO timestamp`);
}
function validateSupersession(revisions: readonly unknown[]) {
  const byId = new Map(
    revisions.map((value) => {
      const revision = record(value, 'revision');
      return [string(revision.id, 'revision id'), revision];
    }),
  );
  byId.forEach((revision, id) => {
    const seen = new Set([id]);
    let cursor = revision.supersedesPlanRevisionId;
    while (cursor !== undefined) {
      if (typeof cursor !== 'string' || !byId.has(cursor) || seen.has(cursor))
        fail('invalid supersession chain');
      seen.add(cursor);
      cursor = byId.get(cursor)?.supersedesPlanRevisionId;
    }
  });
  const roots = revisions.filter(
    (value) => record(value, 'revision').supersedesPlanRevisionId === undefined,
  );
  if (roots.length !== 1) fail('each sprint plan must have exactly one revision root');
  const successors = new Map<string, string>();
  byId.forEach((revision, id) => {
    if (revision.supersedesPlanRevisionId === undefined) return;
    const priorId = string(revision.supersedesPlanRevisionId, 'superseded revision');
    if (successors.has(priorId))
      fail('each sprint plan revision may have at most one direct successor');
    successors.set(priorId, id);
    if ((revision.revision as number) <= (byId.get(priorId)?.revision as number))
      fail('plan revision numbers must increase along supersession');
  });
  const reachable = new Set<string>();
  let cursor: string | undefined = string(record(roots[0], 'revision').id, 'revision id');
  while (cursor !== undefined) {
    if (reachable.has(cursor)) fail('invalid supersession chain');
    reachable.add(cursor);
    cursor = successors.get(cursor);
  }
  if (reachable.size !== revisions.length)
    fail('every plan revision must be reachable from its root');
}
function fail(message: string): never {
  throw new Error(`Invalid Sprint control surface data: ${message}`);
}
