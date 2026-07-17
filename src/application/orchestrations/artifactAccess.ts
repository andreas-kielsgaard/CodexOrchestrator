/**
 * Provider- and storage-neutral facts and ports for technical artifacts and user Documents.
 * These contracts describe intent and recorded outcomes only; they neither resolve nor open files.
 */
export const ARTIFACT_ACCESS_CONTRACTS_V1 = 'orchestration-artifact-access/v1' as const;

declare const artifactIdBrand: unique symbol;
declare const documentRefIdBrand: unique symbol;
export type ArtifactId = string & { readonly [artifactIdBrand]: 'ArtifactId' };
export type DocumentRefId = string & { readonly [documentRefIdBrand]: 'DocumentRefId' };

export type ArtifactKind =
  | 'epic_plan'
  | 'handoff_note'
  | 'changed_file_manifest'
  | 'review_material'
  | 'agent_control_record'
  | 'other';

export interface ArtifactReferenceV1 {
  readonly artifactId: ArtifactId;
  readonly kind: ArtifactKind;
  readonly provenanceReference: string;
  readonly relatedFactReferences?: readonly string[];
}

export interface ChangedFileReferenceV1 {
  readonly changedFileReferenceId: string;
  readonly displayName: string;
  readonly changeKind: 'added' | 'modified' | 'deleted' | 'renamed' | 'other';
}

/** User-inspectable material only; technical artifacts are not Documents by default. */
export interface DocumentReferenceV1 {
  readonly documentRefId: DocumentRefId;
  readonly classification: 'handoff_note' | 'changed_files' | 'review_material' | 'other';
  readonly title: string;
  readonly summary?: string;
  readonly artifactIds: readonly ArtifactId[];
  readonly changedFileReferenceIds: readonly string[];
  readonly provenanceReference: string;
}

export type ArtifactAccessOperationKind =
  'resolve_for_open' | 'open_with_system_default' | 'copy_path';

export interface ArtifactAccessOperationRequestV1 {
  readonly artifactAccessOperationRequestId: string;
  readonly operationKind: ArtifactAccessOperationKind;
  readonly artifactId: ArtifactId;
  readonly purpose: 'user_document_inspection';
  readonly idempotency: { readonly key: string; readonly scopeId: ArtifactId };
  readonly requestedAt: string;
  readonly requestEvidenceReference: string;
}

export type ArtifactAccessRequestFor<Kind extends ArtifactAccessOperationKind> =
  ArtifactAccessOperationRequestV1 & { readonly operationKind: Kind };
export type ResolveForOpenRequestV1 = ArtifactAccessRequestFor<'resolve_for_open'>;
export type OpenWithSystemDefaultRequestV1 = ArtifactAccessRequestFor<'open_with_system_default'>;
export type CopyPathRequestV1 = ArtifactAccessRequestFor<'copy_path'>;

interface ArtifactAccessResultBaseV1 {
  readonly artifactAccessOperationResultId: string;
  readonly artifactAccessOperationRequestId: string;
  readonly recordedAt: string;
  readonly message?: string;
}

type ArtifactAccessUnobservedResultV1 = ArtifactAccessResultBaseV1 & {
  readonly status: 'requested' | 'unsupported' | 'denied' | 'failed';
};

type ArtifactAccessObservedResultV1 = ArtifactAccessResultBaseV1 & {
  readonly status: 'observed_success';
  readonly observedEffectReference: string;
};

export type ArtifactAccessResultFor<Kind extends ArtifactAccessOperationKind> =
  | ArtifactAccessUnobservedResultV1
  | (Kind extends 'copy_path'
      ? ArtifactAccessObservedResultV1 & {
          /** Raw paths are permitted only in this explicit successful copy-path operation result. */
          readonly rawPath: string;
        }
      : ArtifactAccessObservedResultV1);

export type ArtifactAccessOperationResultV1 =
  | ArtifactAccessResultFor<'resolve_for_open'>
  | ArtifactAccessResultFor<'open_with_system_default'>
  | ArtifactAccessResultFor<'copy_path'>;

export interface ArtifactAccessContractsV1 {
  readonly version: typeof ARTIFACT_ACCESS_CONTRACTS_V1;
  readonly artifacts: readonly ArtifactReferenceV1[];
  readonly changedFileReferences: readonly ChangedFileReferenceV1[];
  readonly documents: readonly DocumentReferenceV1[];
  readonly requests: readonly ArtifactAccessOperationRequestV1[];
  readonly results: readonly ArtifactAccessOperationResultV1[];
}

export interface ArtifactAccessOutcome {
  readonly requested: boolean;
  readonly resolutionObserved: boolean;
  readonly openedWithSystemDefault: boolean;
  readonly copyPathObserved: boolean;
}

/** A request or successful resolution never proves that the system application opened the item. */
export function projectArtifactAccessOutcome(
  contracts: ArtifactAccessContractsV1,
  requestId: string,
): ArtifactAccessOutcome {
  const request = contracts.requests.find(
    (candidate) => candidate.artifactAccessOperationRequestId === requestId,
  );
  const observed = contracts.results.some(
    (result) =>
      result.artifactAccessOperationRequestId === requestId && result.status === 'observed_success',
  );
  return {
    requested: request !== undefined,
    resolutionObserved: request?.operationKind === 'resolve_for_open' && observed,
    openedWithSystemDefault: request?.operationKind === 'open_with_system_default' && observed,
    copyPathObserved: request?.operationKind === 'copy_path' && observed,
  };
}

/** Future adapter boundary. No implementation is supplied in this Sprint. */
export interface ArtifactAccessPortV1 {
  resolveForOpen(
    request: ResolveForOpenRequestV1,
  ):
    | ArtifactAccessResultFor<'resolve_for_open'>
    | Promise<ArtifactAccessResultFor<'resolve_for_open'>>;
  openWithSystemDefault(
    request: OpenWithSystemDefaultRequestV1,
  ):
    | ArtifactAccessResultFor<'open_with_system_default'>
    | Promise<ArtifactAccessResultFor<'open_with_system_default'>>;
  copyPath(
    request: CopyPathRequestV1,
  ): ArtifactAccessResultFor<'copy_path'> | Promise<ArtifactAccessResultFor<'copy_path'>>;
}
