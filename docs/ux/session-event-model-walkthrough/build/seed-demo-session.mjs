import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const databasePath = process.argv[2];
if (!databasePath) throw new Error('database path is required');

const reference = (namespace, kind, id) => ({ namespace, kind, id });
const runtimeCapabilities = {
  models: ['gpt-5.6-sol', 'gpt-5.6-terra'],
  reasoningModes: ['high', 'low', 'max', 'medium', 'ultra', 'xhigh'],
  sandboxModes: ['workspace_write'],
  mcpTools: {},
  skills: [],
};
const sessionProfile = {
  contractVersion: 1,
  runtimeProfileRef:
    'native-codex:native-profile-fbea976c-102c-4b35-89b7-26bbaccbfed2',
  attachedRuntimeCapabilities: runtimeCapabilities,
  attachedRuntimeLocked: {
    model: null,
    reasoningMode: null,
    sandboxMode: 'workspace_write',
  },
  capabilityProfileId: 'demo-review-capabilities',
  capabilityProfileRevision: 1,
  nodeCapabilities: runtimeCapabilities,
  pinnedDefaults: {
    model: 'gpt-5.6-sol',
    reasoningMode: 'high',
    sandboxMode: 'workspace_write',
  },
};
const digest = createHash('sha256')
  .update(Buffer.from('execution-configuration/session-profile\0'))
  .update(Buffer.from([0, 0, 0, 1]))
  .update(Buffer.from(JSON.stringify(sessionProfile)))
  .digest('hex');
const creationResolution = {
  contractVersion: 1,
  sessionProfile,
  digest,
};

const sessionId = 'demo-session-architecture-review';
const sessionReference = reference('orchestrator.agent_sessions', 'session', sessionId);
const eventGroup = reference(
  'orchestrator.session_events',
  'event_group',
  'demo-profile-review',
);
const delivery = reference(
  'orchestrator.session_events',
  'delivery',
  'demo-profile-review-1',
);
const invocation = reference(
  'orchestrator.agent_sessions',
  'invocation',
  'demo-profile-review-invocation',
);
const request = reference(
  'orchestrator.user_requests',
  'request',
  'walkthrough-request-1',
);
const definition = reference(
  'orchestrator.workflows',
  'session_event_definition',
  'architecture-review-user-entry',
);
const logicalAddress = {
  scope: reference('orchestrator.workflows', 'workflow_instance', 'demo-architecture-review'),
  subject: reference(
    'orchestrator.workflows',
    'workflow_node',
    'node-d51f1b4d-4838-49cc-a87a-e19a74089e78',
  ),
};
const requestPrompt = {
  kind: 'user_request_text',
  request,
  text: 'Review the Capability Profile to Session Event boundary and recommend one next improvement.',
};
const initialPrompt = {
  kind: 'literal',
  text: "You are the Architecture Reviewer. Distinguish observed facts from recommendations.",
};
const targetSelection = {
  target: { kind: 'logical', address: logicalAddress },
  cardinality: 'first',
  ordering: 'newest',
  running: 'any',
  createdBy: null,
  missing: 'create',
};
const groupRecord = {
  eventGroupId: eventGroup,
  definitionRef: definition,
  trigger: { kind: 'user_request', request },
  source: { kind: 'user_request', request },
  promptSources: [requestPrompt],
  createdSessionPromptSources: [initialPrompt],
  targetSelection,
  resolvedSessions: [sessionReference],
  createdSession: sessionReference,
  outcome: 'delivered',
  deliveryCount: 1,
};
const deliveryRecord = {
  deliveryId: delivery,
  eventGroupId: eventGroup,
  ordinal: 1,
  targetSession: sessionReference,
  logicalAddress,
  targetCreated: true,
  promptContributions: [requestPrompt],
  includedCreatedSessionContributions: [initialPrompt],
  addressedSequence: 1,
  addressingError: null,
  outcome: { kind: 'dispatched', invocation },
};

const quote = (value) => `'${String(value).replaceAll("'", "''")}'`;
const json = (value) => quote(JSON.stringify(value));
const timestamp = '2026-09-02T12:00:00.000Z';
const sql = `
PRAGMA foreign_keys=ON;
BEGIN IMMEDIATE;
DELETE FROM session_event_deliveries WHERE id=${json(delivery)};
DELETE FROM session_event_groups WHERE id=${json(eventGroup)};
DELETE FROM agent_session_addresses WHERE session_id=${quote(sessionId)};
DELETE FROM agent_sessions WHERE id=${quote(sessionId)};
INSERT INTO agent_sessions (
  id,title,availability,external_context_id,runtime_version,working_directory,
  requested_options_json,session_profile_json,harness_version_ref_json,
  assigned_identity_json,created_at,updated_at
) VALUES (
  ${quote(sessionId)},${quote('Architecture Reviewer · demo inspection')},'available',
  NULL,NULL,NULL,${json({ model: null, sandbox: null })},${json(creationResolution)},
  NULL,NULL,${quote(timestamp)},${quote(timestamp)}
);
INSERT INTO agent_session_addresses (
  session_id,scope_namespace,scope_kind,scope_id,subject_namespace,subject_kind,
  subject_id,created_by_event_json,created_by_session_json,created_sequence,
  last_addressed_sequence
) VALUES (
  ${quote(sessionId)},'orchestrator.workflows','workflow_instance','demo-architecture-review',
  'orchestrator.workflows','workflow_node','node-d51f1b4d-4838-49cc-a87a-e19a74089e78',
  ${json(eventGroup)},NULL,1,1
);
INSERT INTO session_event_groups (id,record_json,recorded_at)
VALUES (${json(eventGroup)},${json(groupRecord)},${quote(timestamp)});
INSERT INTO session_event_deliveries (id,event_group_id,ordinal,record_json)
VALUES (${json(delivery)},${json(eventGroup)},1,${json(deliveryRecord)});
COMMIT;
`;

execFileSync('sqlite3', [databasePath], { input: sql, stdio: ['pipe', 'inherit', 'inherit'] });
console.log(JSON.stringify({ sessionId, digest }, null, 2));
