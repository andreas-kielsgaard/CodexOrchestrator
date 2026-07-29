import type {
  FileReviewChangeKind,
  FileReviewClient,
  FileReviewContent,
  FileReviewDiffHunk,
  FileReviewDiffLine,
  FileReviewFile,
  FileReviewSnapshot,
  FileReviewSourceSummary,
} from './fileReview';

export const STORED_FILE_REVIEW_ARTIFACT_V1 = 'stored-file-review-artifact/v1' as const;
export const DEFAULT_FILE_REVIEW_ARTIFACT_SIZE_LIMIT = 1_000_000;

export interface ApplicationFileReviewChangedFile {
  readonly changedFileReferenceId: string;
  readonly displayName: string;
  readonly changeKind: FileReviewChangeKind | 'other';
}

export interface ApplicationFileReviewDocument {
  readonly documentRefId: string;
  readonly classification: string;
  readonly title: string;
  readonly summary?: string;
  readonly artifactIds: readonly string[];
  readonly changedFiles: readonly ApplicationFileReviewChangedFile[];
}

/** Authoritative application-owned Document catalog. Loading rechecks current authorization. */
export interface FileReviewDocumentPort {
  listDocuments(): Promise<readonly ApplicationFileReviewDocument[]>;
  loadDocument(documentRefId: string): Promise<ApplicationFileReviewDocument | null>;
}

export interface StoredFileReviewArtifact {
  readonly documentRefId: string;
  readonly artifactId: string;
  readonly bytes: Uint8Array;
}

/** Read-only stored-artifact boundary. The request carries the authorized Document identity. */
export interface StoredFileReviewArtifactPort {
  loadArtifact(request: {
    readonly documentRefId: string;
    readonly artifactId: string;
  }): Promise<StoredFileReviewArtifact | null>;
}

export type FileReviewSourceErrorCode =
  | 'source_unauthorized'
  | 'artifact_unavailable'
  | 'identity_mismatch'
  | 'artifact_too_large'
  | 'artifact_invalid';

export class FileReviewSourceError extends Error {
  constructor(
    readonly code: FileReviewSourceErrorCode,
    message: string,
  ) {
    super(message);
    this.name = 'FileReviewSourceError';
  }
}

export function createApplicationOwnedFileReviewClient(
  documents: FileReviewDocumentPort,
  artifacts: StoredFileReviewArtifactPort,
  options: { readonly maxArtifactBytes?: number } = {},
): FileReviewClient {
  const maxArtifactBytes = options.maxArtifactBytes ?? DEFAULT_FILE_REVIEW_ARTIFACT_SIZE_LIMIT;
  if (!Number.isSafeInteger(maxArtifactBytes) || maxArtifactBytes < 1)
    throw new Error('File review artifact size limit must be a positive safe integer.');

  return {
    async listSources() {
      const listed = await documents.listDocuments();
      const sourceIds = new Set<string>();
      return listed.flatMap((document) => {
        if (!isEligibleDocument(document)) return [];
        validateDocument(document);
        if (sourceIds.has(document.documentRefId))
          fail('identity_mismatch', 'The Document catalog returned a duplicate source identity.');
        sourceIds.add(document.documentRefId);
        return [toSource(document)];
      });
    },
    async loadSource(sourceId) {
      const document = await documents.loadDocument(sourceId);
      if (!document || document.documentRefId !== sourceId || !isEligibleDocument(document))
        fail(
          'source_unauthorized',
          'The requested Document review is unavailable or not authorized.',
        );
      validateDocument(document);

      const artifactId = document.artifactIds[0];
      const artifact = await artifacts.loadArtifact({
        documentRefId: document.documentRefId,
        artifactId,
      });
      if (!artifact)
        fail('artifact_unavailable', 'The stored diff artifact is currently unavailable.');
      if (artifact.documentRefId !== document.documentRefId || artifact.artifactId !== artifactId)
        fail(
          'identity_mismatch',
          'The stored diff artifact identity does not match the authorized Document.',
        );
      if (artifact.bytes.byteLength > maxArtifactBytes)
        fail(
          'artifact_too_large',
          `The stored diff artifact exceeds the ${maxArtifactBytes}-byte review limit.`,
        );

      return decodeSnapshot(document, artifact);
    },
  };
}

export function combineFileReviewClients(clients: readonly FileReviewClient[]): FileReviewClient {
  return {
    async listSources() {
      const groups = await Promise.all(clients.map((client) => client.listSources()));
      const seen = new Set<string>();
      return groups.flat().map((source) => {
        if (seen.has(source.sourceId))
          fail('identity_mismatch', 'Multiple review clients returned the same source identity.');
        seen.add(source.sourceId);
        return source;
      });
    },
    async loadSource(sourceId) {
      const groups = await Promise.all(clients.map((client) => client.listSources()));
      const owners = groups
        .map((sources, index) =>
          sources.some((source) => source.sourceId === sourceId) ? index : -1,
        )
        .filter((index) => index >= 0);
      if (owners.length !== 1)
        fail(
          'source_unauthorized',
          'The requested review source has no unambiguous authorized client.',
        );
      return clients[owners[0]].loadSource(sourceId);
    },
  };
}

function decodeSnapshot(
  document: ApplicationFileReviewDocument,
  artifact: StoredFileReviewArtifact,
): FileReviewSnapshot {
  if (looksBinary(artifact.bytes))
    fail('artifact_invalid', 'The stored diff artifact is binary, not a review contract.');
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(artifact.bytes);
  } catch {
    fail('artifact_invalid', 'The stored diff artifact is not valid UTF-8.');
  }
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    fail('artifact_invalid', 'The stored diff artifact is not valid JSON.');
  }
  const root = record(value, 'stored diff artifact');
  equal(root.contractVersion, STORED_FILE_REVIEW_ARTIFACT_V1, 'contract version');
  equal(root.documentRefId, document.documentRefId, 'Document identity');
  equal(root.artifactId, artifact.artifactId, 'artifact identity');

  const fileValues = list(root.files, 'files');
  const authorizedFiles = new Map(
    document.changedFiles.map((file) => [file.changedFileReferenceId, file]),
  );
  if (fileValues.length !== authorizedFiles.size)
    fail(
      'identity_mismatch',
      'The stored diff artifact does not contain the authorized changed-file set.',
    );
  const seen = new Set<string>();
  const files = fileValues.map((file, index) => {
    const stored = record(file, `file ${index + 1}`);
    const changedFileReferenceId = string(
      stored.changedFileReferenceId,
      'changed-file reference identity',
    );
    const authorized = authorizedFiles.get(changedFileReferenceId);
    if (!authorized || seen.has(changedFileReferenceId))
      fail(
        'identity_mismatch',
        'The stored diff artifact contains an unauthorized changed-file identity.',
      );
    seen.add(changedFileReferenceId);
    if (authorized.changeKind === 'other')
      fail('artifact_invalid', 'The changed-file kind has no truthful viewer mapping.');
    validateDisplayName(authorized.displayName);
    return decodeFile(stored, {
      ...authorized,
      changeKind: authorized.changeKind,
    });
  });

  return {
    source: toSource(document),
    files,
  };
}

function decodeFile(
  stored: Record<string, unknown>,
  authorized: ApplicationFileReviewChangedFile & { readonly changeKind: FileReviewChangeKind },
): FileReviewFile {
  const hunks = list(stored.hunks, 'file hunks').map((value, index) =>
    decodeHunk(value, `${authorized.changedFileReferenceId}:hunk:${index + 1}`),
  );
  const counts = hunks.reduce(
    (total, hunk) => {
      for (const line of hunk.lines) {
        if (line.kind === 'addition') total.additions += 1;
        if (line.kind === 'deletion') total.deletions += 1;
      }
      return total;
    },
    { additions: 0, deletions: 0 },
  );
  return {
    fileId: authorized.changedFileReferenceId,
    displayPath: authorized.displayName,
    changeKind: authorized.changeKind,
    additions: counts.additions,
    deletions: counts.deletions,
    content: decodeContent(record(stored.content, 'file content'), authorized.displayName),
    hunks,
  };
}

function decodeContent(content: Record<string, unknown>, displayName: string): FileReviewContent {
  const encoding = string(content.encoding, 'content encoding').toLowerCase();
  const bytes = decodeBase64(text(content.bytesBase64, 'content bytes'));
  if (looksBinary(bytes))
    return {
      kind: 'binary',
      reason: 'The stored file bytes were identified as binary.',
    };
  if (encoding !== 'utf-8')
    return {
      kind: 'unsupported',
      reason: `The stored file uses unsupported ${encoding} encoding.`,
    };
  try {
    const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    return markdownDisplayName(displayName) ? { kind: 'markdown', text } : { kind: 'text', text };
  } catch {
    return {
      kind: 'unsupported',
      reason: 'The stored file bytes are not valid UTF-8.',
    };
  }
}

function decodeHunk(value: unknown, hunkId: string): FileReviewDiffHunk {
  const hunk = record(value, 'diff hunk');
  return {
    hunkId,
    header: string(hunk.header, 'diff hunk header'),
    lines: list(hunk.lines, 'diff hunk lines').map(decodeLine),
    ...(hunk.collapsedBefore === undefined
      ? {}
      : {
          collapsedBefore: list(hunk.collapsedBefore, 'collapsed lines above').map(decodeLine),
        }),
    ...(hunk.collapsedAfter === undefined
      ? {}
      : {
          collapsedAfter: list(hunk.collapsedAfter, 'collapsed lines below').map(decodeLine),
        }),
  };
}

function decodeLine(value: unknown): FileReviewDiffLine {
  const line = record(value, 'diff line');
  const kind = literal(line.kind, ['context', 'addition', 'deletion'] as const, 'diff line kind');
  const oldLineNumber = optionalLineNumber(line.oldLineNumber, 'old line number');
  const newLineNumber = optionalLineNumber(line.newLineNumber, 'new line number');
  if (kind === 'addition' && oldLineNumber !== undefined)
    fail('artifact_invalid', 'An addition cannot carry an old line number.');
  if (kind === 'deletion' && newLineNumber !== undefined)
    fail('artifact_invalid', 'A deletion cannot carry a new line number.');
  if (kind === 'addition' && newLineNumber === undefined)
    fail('artifact_invalid', 'An addition requires a new line number.');
  if (kind === 'deletion' && oldLineNumber === undefined)
    fail('artifact_invalid', 'A deletion requires an old line number.');
  if (kind === 'context' && (oldLineNumber === undefined || newLineNumber === undefined))
    fail('artifact_invalid', 'A context line requires old and new line numbers.');
  return {
    kind,
    text: text(line.text, 'diff line text'),
    ...(oldLineNumber === undefined ? {} : { oldLineNumber }),
    ...(newLineNumber === undefined ? {} : { newLineNumber }),
  };
}

function toSource(document: ApplicationFileReviewDocument): FileReviewSourceSummary {
  return {
    sourceId: document.documentRefId,
    kind: 'application_owned',
    label: document.title,
    detail: document.summary ?? 'Application-owned Document',
    comparisonLabel: 'Compare with Sprint start',
  };
}

function isEligibleDocument(document: ApplicationFileReviewDocument) {
  return document.artifactIds.length === 1 && document.changedFiles.length > 0;
}

function validateDocument(document: ApplicationFileReviewDocument) {
  string(document.documentRefId, 'Document identity');
  string(document.title, 'Document title');
  string(document.artifactIds[0], 'artifact identity');
  const changedFileIds = new Set<string>();
  for (const file of document.changedFiles) {
    string(file.changedFileReferenceId, 'changed-file reference identity');
    if (changedFileIds.has(file.changedFileReferenceId))
      fail('identity_mismatch', 'The Document repeats a changed-file identity.');
    changedFileIds.add(file.changedFileReferenceId);
    validateDisplayName(file.displayName);
  }
}

function validateDisplayName(value: string) {
  string(value, 'changed-file display name');
  if (
    Array.from(value).some((character) => character.charCodeAt(0) < 32) ||
    /^(?:[A-Za-z]:[\\/]|\\\\|\/|~[\\/])/.test(value) ||
    value.split(/[\\/]/).includes('..')
  )
    fail('source_unauthorized', 'The changed-files Document contains a non-relative display name.');
}

function looksBinary(bytes: Uint8Array) {
  if (bytes.includes(0)) return true;
  const sample = bytes.subarray(0, Math.min(bytes.length, 8_192));
  const controls = sample.reduce(
    (count, byte) => count + (byte < 32 && byte !== 9 && byte !== 10 && byte !== 13 ? 1 : 0),
    0,
  );
  return sample.length > 0 && controls / sample.length > 0.1;
}

function decodeBase64(value: string) {
  if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value))
    fail('artifact_invalid', 'Stored file content is not valid base64.');
  try {
    return Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
  } catch {
    fail('artifact_invalid', 'Stored file content is not valid base64.');
  }
}

function markdownDisplayName(value: string) {
  return /\.(?:md|mdx|markdown)$/i.test(value);
}

function optionalLineNumber(value: unknown, label: string) {
  if (value === undefined) return undefined;
  if (!Number.isSafeInteger(value) || (value as number) < 1)
    fail('artifact_invalid', `${label} must be a positive safe integer.`);
  return value as number;
}

function record(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    fail('artifact_invalid', `${label} must be an object.`);
  return value as Record<string, unknown>;
}

function list(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) fail('artifact_invalid', `${label} must be an array.`);
  return value;
}

function string(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim())
    fail('artifact_invalid', `${label} must be a non-empty string.`);
  return value;
}

function text(value: unknown, label: string): string {
  if (typeof value !== 'string') fail('artifact_invalid', `${label} must be a string.`);
  return value;
}

function equal(value: unknown, expected: string, label: string) {
  if (value !== expected) fail('identity_mismatch', `The stored ${label} does not match.`);
}

function literal<const Values extends readonly string[]>(
  value: unknown,
  values: Values,
  label: string,
): Values[number] {
  if (typeof value !== 'string' || !values.includes(value))
    fail('artifact_invalid', `${label} is unsupported.`);
  return value as Values[number];
}

function fail(code: FileReviewSourceErrorCode, message: string): never {
  throw new FileReviewSourceError(code, message);
}
