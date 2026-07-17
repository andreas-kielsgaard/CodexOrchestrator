import type {
  ArtifactAccessOperationKind,
  ArtifactId,
  ArtifactAccessPortV1,
  ArtifactAccessResultFor,
  CopyPathRequestV1,
  OpenWithSystemDefaultRequestV1,
  ResolveForOpenRequestV1,
} from './artifactAccess';

export type ArtifactAccessUiOperation =
  'resolve_for_open' | 'open_with_system_default' | 'copy_path';

export interface ArtifactAccessUiFeedback {
  readonly operation: ArtifactAccessUiOperation;
  readonly status: 'requested' | 'unsupported' | 'denied' | 'failed' | 'observed_success';
  readonly message: string;
  /** Present only after an observed successful copy-path operation. */
  readonly rawPath?: string;
}

/** UI-facing boundary: it turns a selected canonical Document into one explicit port operation. */
export interface ArtifactAccessController {
  resolveForOpen(document: ArtifactAccessDocument): Promise<ArtifactAccessUiFeedback>;
  openWithSystemDefault(document: ArtifactAccessDocument): Promise<ArtifactAccessUiFeedback>;
  copyPath(document: ArtifactAccessDocument): Promise<ArtifactAccessUiFeedback>;
}

export interface ArtifactAccessDocument {
  readonly documentRefId: string;
  readonly title: string;
  readonly artifactIds: readonly string[];
}

export interface ArtifactAccessControllerOptions {
  readonly now?: () => string;
  readonly nextRequestId?: () => string;
}

export function createArtifactAccessController(
  port: ArtifactAccessPortV1,
  {
    now = () => new Date().toISOString(),
    nextRequestId = defaultRequestId,
  }: ArtifactAccessControllerOptions = {},
): ArtifactAccessController {
  return {
    resolveForOpen: (document) => execute('resolve_for_open', document, port, now, nextRequestId),
    openWithSystemDefault: (document) =>
      execute('open_with_system_default', document, port, now, nextRequestId),
    copyPath: (document) => execute('copy_path', document, port, now, nextRequestId),
  };
}

/** Product composition stays honest until a focused native adapter is introduced. */
export const unsupportedArtifactAccessController: ArtifactAccessController =
  createArtifactAccessController({
    resolveForOpen: unsupported,
    openWithSystemDefault: unsupported,
    copyPath: unsupported,
  });

async function execute(
  operation: ArtifactAccessUiOperation,
  document: ArtifactAccessDocument,
  port: ArtifactAccessPortV1,
  now: () => string,
  nextRequestId: () => string,
): Promise<ArtifactAccessUiFeedback> {
  if (document.artifactIds.length !== 1)
    return feedback(
      operation,
      'failed',
      `${document.title} has no unambiguous artifact to ${verb(operation)}.`,
    );

  const artifactId = document.artifactIds[0] as ArtifactId;
  const requestBase = {
    artifactAccessOperationRequestId: nextRequestId(),
    operationKind: operation,
    artifactId,
    purpose: 'user_document_inspection' as const,
    idempotency: { key: `${operation}:${artifactId}`, scopeId: artifactId },
    requestedAt: now(),
    requestEvidenceReference: `document:${document.documentRefId}`,
  };
  try {
    const result =
      operation === 'resolve_for_open'
        ? await port.resolveForOpen({
            ...requestBase,
            operationKind: operation,
          } as ResolveForOpenRequestV1)
        : operation === 'open_with_system_default'
          ? await port.openWithSystemDefault({
              ...requestBase,
              operationKind: operation,
            } as OpenWithSystemDefaultRequestV1)
          : await port.copyPath({ ...requestBase, operationKind: operation } as CopyPathRequestV1);
    return toFeedback(operation, document.title, result);
  } catch {
    return feedback(
      operation,
      'failed',
      `${document.title} ${verb(operation)} failed before an outcome was observed.`,
    );
  }
}

function toFeedback(
  operation: ArtifactAccessUiOperation,
  title: string,
  result: ArtifactAccessResultFor<ArtifactAccessOperationKind>,
): ArtifactAccessUiFeedback {
  if (
    containsPath(result.message) ||
    ('observedEffectReference' in result && containsPath(result.observedEffectReference))
  )
    return feedback(
      operation,
      'failed',
      `${title} ${verb(operation)} failed because the adapter returned a prohibited path reference.`,
    );
  if (result.status === 'observed_success' && operation === 'copy_path') {
    if (!('rawPath' in result) || !result.rawPath)
      return feedback(
        operation,
        'failed',
        `${title} copy path succeeded without a copy-path value.`,
      );
    return {
      ...feedback(operation, result.status, result.message ?? `${title} path was copied.`),
      rawPath: result.rawPath,
    };
  }
  return feedback(
    operation,
    result.status,
    result.message ?? `${title} ${defaultMessage(operation, result.status)}`,
  );
}

function unsupported(request: { readonly artifactAccessOperationRequestId: string }) {
  return {
    artifactAccessOperationResultId: `${request.artifactAccessOperationRequestId}:unsupported`,
    artifactAccessOperationRequestId: request.artifactAccessOperationRequestId,
    recordedAt: new Date().toISOString(),
    status: 'unsupported' as const,
    message: 'Artifact access is not connected to a native implementation in product mode.',
  };
}

function feedback(
  operation: ArtifactAccessUiOperation,
  status: ArtifactAccessUiFeedback['status'],
  message: string,
): ArtifactAccessUiFeedback {
  return { operation, status, message };
}

function defaultMessage(
  operation: ArtifactAccessUiOperation,
  status: ArtifactAccessUiFeedback['status'],
) {
  return `${verb(operation)} ${status === 'observed_success' ? 'was observed.' : `is ${status}.`}`;
}

function verb(operation: ArtifactAccessUiOperation) {
  return {
    resolve_for_open: 'resolve for opening',
    open_with_system_default: 'open with the system default',
    copy_path: 'copy path',
  }[operation];
}

function containsPath(value: unknown) {
  return (
    typeof value === 'string' &&
    /(?:^|[\s"'(=,;])(?:[A-Za-z]:[\\/]|\\\\[^\\/\s]+[\\/]|\/|~[\\/])|file:\/\//i.test(value)
  );
}

function defaultRequestId() {
  return `artifact-access-${globalThis.crypto.randomUUID()}`;
}
