import { invoke } from '@tauri-apps/api/core';
import {
  FileReviewSourceError,
  type ApplicationFileReviewDocument,
  type FileReviewDocumentPort,
  type StoredFileReviewArtifactPort,
} from '../../application/applicationOwnedFileReview';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
type Response =
  | { readonly status: 'available'; readonly document: Document }
  | { readonly status: 'unavailable' | 'unauthorized' | 'invalid' };
interface Document {
  readonly documentRefId: string;
  readonly title: string;
  readonly summary?: string;
  readonly artifactId: string;
  readonly payload: number[];
  readonly changedFiles: readonly { readonly changedFileReferenceId: string; readonly displayName: string; readonly changeKind: string }[];
}

/** One application-owned opaque selection; it never accepts Document or artifact authority. */
export function createTauriScopedFileReviewPorts(
  opaqueReference: string,
  invokeCommand: TauriInvoke = invoke,
): { readonly documents: FileReviewDocumentPort; readonly artifacts: StoredFileReviewArtifactPort } {
  let cached: Document | undefined;
  const load = async () => {
    const result = await invokeCommand<Response>('load_scoped_file_review', { input: { opaqueReference } });
    if (result.status === 'unauthorized') throw new FileReviewSourceError('source_unauthorized', 'The scoped changed-files Document is not authorized.');
    if (result.status === 'invalid') throw new FileReviewSourceError('artifact_invalid', 'The scoped File Review facts are invalid.');
    cached = result.status === 'available' ? result.document : undefined;
    return cached;
  };
  return {
    documents: {
      async loadDocument(): Promise<ApplicationFileReviewDocument | null> {
        const document = await load();
        return document ? {
          documentRefId: document.documentRefId,
          classification: 'changed_files',
          title: document.title,
          ...(document.summary ? { summary: document.summary } : {}),
          artifactIds: [document.artifactId],
          changedFiles: document.changedFiles.map((file) => ({
            changedFileReferenceId: file.changedFileReferenceId,
            displayName: file.displayName,
            changeKind: file.changeKind as ApplicationFileReviewDocument['changedFiles'][number]['changeKind'],
          })),
        } : null;
      },
    },
    artifacts: {
      async loadArtifact(request) {
        const document = await load();
        if (!document || request.documentRefId !== document.documentRefId || request.artifactId !== document.artifactId)
          return null;
        return { documentRefId: document.documentRefId, artifactId: document.artifactId, bytes: new Uint8Array(document.payload) };
      },
    },
  };
}
