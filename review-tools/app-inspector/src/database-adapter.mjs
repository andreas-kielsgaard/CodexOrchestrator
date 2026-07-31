import { createHash } from 'node:crypto';
import { stat } from 'node:fs/promises';
import { DatabaseSync } from 'node:sqlite';

const countTables = [
  'epic_planning_drafts',
  'proposal_revisions',
  'epic_initiations',
  'initiated_sprints',
  'epic_bootstrap_transitions',
  'epic_bootstrap_attempts',
  'agent_sessions',
  'agent_session_invocations',
  'agent_session_invocation_launch_acceptances',
  'plan_builder_context_deliveries',
];

export async function inspectDatabaseState(databasePath) {
  let database;
  try {
    const details = await stat(databasePath);
    database = new DatabaseSync(databasePath, { readOnly: true });
    database.exec('PRAGMA query_only = ON');

    const tables = new Set(
      database
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .all()
        .map((row) => row.name),
    );
    requireTables(tables, [
      'epic_planning_drafts',
      'proposal_revisions',
      'epic_initiations',
      'initiated_sprints',
      'agent_sessions',
      'agent_session_invocations',
    ]);

    const planningDrafts = rows(
      database,
      `
      SELECT draft.id,
             draft.title,
             CASE WHEN initiated.draft_id IS NOT NULL THEN 'initiated' ELSE draft.status END AS status,
             draft.created_at AS createdAt,
             draft.updated_at AS updatedAt,
             latest.id AS proposalRevisionId,
             latest.proposal_json AS proposalJson
      FROM epic_planning_drafts draft
      LEFT JOIN initiated_planning_drafts initiated ON initiated.draft_id = draft.id
      LEFT JOIN proposal_revisions latest ON latest.id = (
        SELECT revision.id FROM proposal_revisions revision
        WHERE revision.draft_id = draft.id
        ORDER BY revision.recorded_at DESC, revision.id DESC LIMIT 1
      )
      ORDER BY draft.created_at, draft.id
    `,
    ).map((row) => {
      const proposal = parseJson(row.proposalJson);
      return {
        id: row.id,
        title: row.title,
        status: row.status,
        createdAt: row.createdAt,
        updatedAt: row.updatedAt,
        proposalRevisionId: row.proposalRevisionId,
        suggestedEpicName: proposal?.suggestedEpicName ?? null,
        proposedSprintCount: Array.isArray(proposal?.sprints) ? proposal.sprints.length : 0,
      };
    });

    const initiatedEpics = rows(
      database,
      `
      SELECT initiation.id,
             initiation.epic_id AS epicId,
             initiation.draft_id AS draftId,
             initiation.proposal_revision_id AS proposalRevisionId,
             initiation.recorded_at AS recordedAt,
             snapshot.proposal_json AS proposalJson
      FROM epic_initiations initiation
      JOIN epic_initiation_material_snapshots snapshot ON snapshot.id = initiation.material_snapshot_id
      ORDER BY initiation.recorded_at, initiation.id
    `,
    ).map((row) => {
      const proposal = parseJson(row.proposalJson);
      return {
        id: row.id,
        epicId: row.epicId,
        draftId: row.draftId,
        proposalRevisionId: row.proposalRevisionId,
        recordedAt: row.recordedAt,
        name: proposal?.suggestedEpicName ?? null,
      };
    });

    const initiatedSprints = rows(
      database,
      `
      SELECT id, epic_id AS epicId, ordinal, title
      FROM initiated_sprints ORDER BY epic_id, ordinal
    `,
    );

    const transitions = tables.has('epic_bootstrap_transitions')
      ? rows(
          database,
          `
          SELECT initiation_id AS initiationId,
                 epic_id AS epicId,
                 prepared_at AS preparedAt,
                 bootstrap_session_created_at AS bootstrapSessionCreatedAt,
                 bootstrap_launched_at AS bootstrapLaunchedAt,
                 bootstrap_lifecycle_status AS bootstrapLifecycleStatus,
                 semantic_completed_at AS semanticCompletedAt,
                 material_accepted_at AS materialAcceptedAt,
                 runner_session_created_at AS runnerSessionCreatedAt,
                 runner_launched_at AS runnerLaunchedAt,
                 runner_lifecycle_status AS runnerLifecycleStatus,
                 runner_lifecycle_observed_at AS runnerLifecycleObservedAt,
                 updated_at AS updatedAt
          FROM epic_bootstrap_transitions ORDER BY created_at, initiation_id
        `,
        )
      : [];

    const recentInvocations = rows(
      database,
      `
      SELECT invocation.id,
             invocation.session_id AS sessionId,
             session.title,
             invocation.input_provenance AS inputProvenance,
             invocation.status,
             invocation.started_at AS startedAt,
             invocation.completed_at AS completedAt,
             CASE WHEN acceptance.invocation_id IS NULL THEN 0 ELSE 1 END AS launchAccepted
      FROM agent_session_invocations invocation
      JOIN agent_sessions session ON session.id = invocation.session_id
      LEFT JOIN agent_session_invocation_launch_acceptances acceptance
        ON acceptance.invocation_id = invocation.id
      ORDER BY invocation.created_at DESC, invocation.id DESC LIMIT 12
    `,
    ).map((row) => ({ ...row, launchAccepted: row.launchAccepted === 1 }));

    const counts = Object.fromEntries(
      countTables
        .filter((table) => tables.has(table))
        .map((table) => [
          table,
          Number(database.prepare(`SELECT COUNT(*) AS count FROM ${table}`).get().count),
        ]),
    );

    const value = {
      databasePath,
      bytes: details.size,
      planningDrafts,
      initiatedEpics,
      initiatedSprints,
      transitions,
      recentInvocations,
      counts,
    };
    value.fingerprint = fingerprint({
      planningDrafts,
      initiatedEpics,
      initiatedSprints,
      transitions,
      recentInvocations,
      counts,
    });
    return {
      disposition: 'observed',
      evidenceClass: 'recorded',
      source: databasePath,
      access: 'SQLite readOnly=true; PRAGMA query_only=ON',
      exclusions: ['submitted message text', 'runtime event payloads', 'credentials'],
      value,
    };
  } catch (error) {
    return {
      disposition: 'unavailable',
      reason: `Read-only durable-state inspection failed: ${error instanceof Error ? error.message : String(error)}`,
      source: databasePath,
    };
  } finally {
    database?.close();
  }
}

function rows(database, sql) {
  return database
    .prepare(sql)
    .all()
    .map((row) => ({ ...row }));
}

function requireTables(actual, required) {
  const missing = required.filter((table) => !actual.has(table));
  if (missing.length > 0)
    throw new Error(`database is missing expected tables: ${missing.join(', ')}`);
}

function parseJson(value) {
  if (typeof value !== 'string') return null;
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function fingerprint(value) {
  return createHash('sha256').update(JSON.stringify(value)).digest('hex');
}
