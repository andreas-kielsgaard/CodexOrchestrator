import {
  ARTIFACT_ACCESS_CONTRACTS_V1,
  type ArtifactAccessContractsV1,
  type ArtifactAccessOperationRequestV1,
} from './artifactAccess';

const forbiddenReadFieldFragments = [
  'path',
  'storage',
  'database',
  'provider',
  'locator',
  'filesystem',
  'row',
];
const rawPath =
  /(?:^|[\s"'(=,;])(?:[A-Za-z]:[\\/]|\\\\[^\\/\s]+[\\/]|\/|~[\\/])|file:(?:\/\/)?[\\/]/i;

/** Decodes safe references plus explicit operation facts without resolving any artifact. */
export function decodeArtifactAccessContractsV1(value: unknown): ArtifactAccessContractsV1 {
  const root = record(value, 'artifact access contracts');
  equal(required(root, 'version'), ARTIFACT_ACCESS_CONTRACTS_V1, 'version');
  const artifacts = list(required(root, 'artifacts'), 'artifacts');
  const changed = list(required(root, 'changedFileReferences'), 'changed file references');
  const documents = list(required(root, 'documents'), 'documents');
  const requests = list(required(root, 'requests'), 'requests');
  const results = list(required(root, 'results'), 'results');
  rejectReadLeakage({ artifacts, changed, documents });
  rejectRawPathStrings({ artifacts, changed, documents, requests });

  const artifactIds = identifiers(artifacts, 'artifactId', 'artifact');
  const changedIds = identifiers(changed, 'changedFileReferenceId', 'changed file reference');
  identifiers(documents, 'documentRefId', 'document');
  const requestIds = identifiers(requests, 'artifactAccessOperationRequestId', 'request');
  identifiers(results, 'artifactAccessOperationResultId', 'result');

  for (const item of artifacts) {
    const artifact = record(item, 'artifact');
    literal(
      required(artifact, 'kind'),
      [
        'epic_plan',
        'handoff_note',
        'changed_file_manifest',
        'review_material',
        'agent_control_record',
        'other',
      ],
      'artifact kind',
    );
    string(required(artifact, 'provenanceReference'), 'artifact provenance');
    optionalStrings(artifact.relatedFactReferences, 'artifact related fact references');
  }
  for (const item of changed) {
    const reference = record(item, 'changed file reference');
    displayString(required(reference, 'displayName'), 'changed file display name');
    literal(
      required(reference, 'changeKind'),
      ['added', 'modified', 'deleted', 'renamed', 'other'],
      'changed file kind',
    );
  }
  const documentArtifactIds = new Set<string>();
  for (const item of documents) {
    const document = record(item, 'document');
    literal(
      required(document, 'classification'),
      ['handoff_note', 'changed_files', 'review_material', 'other'],
      'document classification',
    );
    displayString(required(document, 'title'), 'document title');
    if (document.summary !== undefined) displayString(document.summary, 'document summary');
    const linkedArtifacts = list(required(document, 'artifactIds'), 'document artifact ids');
    const linkedChanged = list(
      required(document, 'changedFileReferenceIds'),
      'document changed file ids',
    );
    if (!linkedArtifacts.length && !linkedChanged.length)
      fail('document requires an artifact or changed-file reference');
    linkedArtifacts.forEach((id) => {
      reference(id, artifactIds, 'document artifact');
      documentArtifactIds.add(string(id, 'document artifact'));
    });
    linkedChanged.forEach((id) => reference(id, changedIds, 'document changed file'));
    string(required(document, 'provenanceReference'), 'document provenance');
  }
  const requestsById = new Map<string, ArtifactAccessOperationRequestV1>();
  const idempotency = new Map<string, string>();
  for (const item of requests) {
    const request = record(item, 'request');
    const id = string(required(request, 'artifactAccessOperationRequestId'), 'request id');
    const kind = literal(
      required(request, 'operationKind'),
      ['resolve_for_open', 'open_with_system_default', 'copy_path'],
      'operation kind',
    );
    const artifactId = string(required(request, 'artifactId'), 'request artifact');
    reference(artifactId, artifactIds, 'request artifact');
    equal(required(request, 'purpose'), 'user_document_inspection', 'operation purpose');
    if (!documentArtifactIds.has(artifactId))
      fail('user document inspection requires an explicit Document artifact link');
    const key = record(required(request, 'idempotency'), 'idempotency');
    const idempotencyKey = string(required(key, 'key'), 'idempotency key');
    if (string(required(key, 'scopeId'), 'idempotency scope') !== artifactId)
      fail('idempotency scope must equal request artifact');
    const signature = `${kind}:${artifactId}`;
    const previous = idempotency.get(`${artifactId}:${idempotencyKey}`);
    if (previous !== undefined && previous !== signature)
      fail('idempotency key cannot represent another artifact operation');
    idempotency.set(`${artifactId}:${idempotencyKey}`, signature);
    timestamp(required(request, 'requestedAt'), 'request time');
    string(required(request, 'requestEvidenceReference'), 'request evidence');
    requestsById.set(id, request as unknown as ArtifactAccessOperationRequestV1);
  }
  for (const item of results) validateResult(record(item, 'result'), requestIds, requestsById);
  return root as unknown as ArtifactAccessContractsV1;
}

function validateResult(
  result: Record<string, unknown>,
  requestIds: ReadonlySet<string>,
  requests: ReadonlyMap<string, ArtifactAccessOperationRequestV1>,
) {
  rejectRawPathStrings(result, true);
  const requestId = string(required(result, 'artifactAccessOperationRequestId'), 'result request');
  reference(requestId, requestIds, 'result request');
  const status = literal(
    required(result, 'status'),
    ['requested', 'unsupported', 'denied', 'failed', 'observed_success'],
    'result status',
  );
  timestamp(required(result, 'recordedAt'), 'result time');
  if (result.message !== undefined) string(result.message, 'result message');
  if (status === 'observed_success')
    string(required(result, 'observedEffectReference'), 'observed effect');
  else if (result.observedEffectReference !== undefined)
    fail('only observed success may carry an observed effect');
  if (result.rawPath !== undefined) {
    if (status !== 'observed_success' || requests.get(requestId)?.operationKind !== 'copy_path')
      fail('raw path is allowed only in an observed successful copy-path result');
    string(result.rawPath, 'copied raw path');
  }
}
/** Raw paths never cross this provider-neutral contract except the validated copy-path response. */
function rejectRawPathStrings(value: unknown, allowRawPathField = false): void {
  if (Array.isArray(value)) return value.forEach((item) => rejectRawPathStrings(item));
  if (!value || typeof value !== 'object') return;
  for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
    if (allowRawPathField && key === 'rawPath') continue;
    if (typeof nested === 'string' && rawPath.test(nested))
      fail('raw path is not allowed outside an explicit successful copy-path result');
    rejectRawPathStrings(nested);
  }
}
function rejectReadLeakage(value: unknown): void {
  if (Array.isArray(value)) return value.forEach(rejectReadLeakage);
  if (!value || typeof value !== 'object') return;
  for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
    if (forbiddenReadFieldFragments.some((fragment) => key.toLowerCase().includes(fragment)))
      fail(`${key} is not allowed in a provider-neutral read contract`);
    if (typeof nested === 'string' && rawPath.test(nested))
      fail('raw path is not allowed in a read contract');
    rejectReadLeakage(nested);
  }
}
function identifiers(values: unknown[], field: string, label: string) {
  const ids = new Set<string>();
  for (const item of values) {
    const id = string(required(record(item, label), field), `${label} id`);
    if (ids.has(id)) fail(`duplicate ${label} id`);
    ids.add(id);
  }
  return ids;
}
function optionalStrings(value: unknown, label: string) {
  if (value !== undefined) list(value, label).forEach((item) => string(item, label));
}
function reference(value: unknown, ids: ReadonlySet<string>, label: string) {
  const id = string(value, label);
  if (!ids.has(id)) fail(`dangling ${label}`);
}
function displayString(value: unknown, label: string) {
  const text = string(value, label);
  if (rawPath.test(text)) fail(`${label} must not be a raw path`);
}
function record(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    fail(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function required(value: Record<string, unknown>, key: string): unknown {
  if (!(key in value)) fail(`${key} is required`);
  return value[key];
}
function list(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}
function string(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) fail(`${label} must be a non-empty string`);
  return value;
}
function literal(value: unknown, allowed: readonly string[], label: string): string {
  if (typeof value !== 'string' || !allowed.includes(value)) fail(`${label} is invalid`);
  return value;
}
function equal(value: unknown, expected: string, label: string) {
  if (value !== expected) fail(`${label} is invalid`);
}
function timestamp(value: unknown, label: string) {
  if (typeof value !== 'string' || Number.isNaN(Date.parse(value)))
    fail(`${label} must be a timestamp`);
}
function fail(message: string): never {
  throw new Error(`Invalid artifact access contracts: ${message}`);
}
