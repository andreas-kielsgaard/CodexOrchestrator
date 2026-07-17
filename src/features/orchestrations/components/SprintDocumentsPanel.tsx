import { FileUp } from 'lucide-react';
import { useState } from 'react';
import type {
  ArtifactAccessController,
  SprintWorkspacePresentationV1,
} from '../../../application/orchestrations';
import '../styles/sprintInformationSurfaces.css';

export function SprintDocumentsPanel({
  documents,
  artifactAccess,
}: {
  readonly documents: SprintWorkspacePresentationV1['documents'];
  readonly artifactAccess: ArtifactAccessController;
}) {
  const [feedback, setFeedback] = useState<Awaited<
    ReturnType<ArtifactAccessController['resolveForOpen']>
  > | null>(null);

  const run = async (
    operation: keyof Pick<
      ArtifactAccessController,
      'resolveForOpen' | 'openWithSystemDefault' | 'copyPath'
    >,
    document: SprintWorkspacePresentationV1['documents'][number],
  ) => {
    try {
      setFeedback(await artifactAccess[operation](document));
    } catch {
      setFeedback({
        operation:
          operation === 'resolveForOpen'
            ? 'resolve_for_open'
            : operation === 'copyPath'
              ? 'copy_path'
              : 'open_with_system_default',
        status: 'failed',
        message: `${document.title} operation failed before an outcome was observed.`,
      });
    }
  };

  return (
    <div className="sprint-documents" aria-label="Sprint documents">
      <header>
        <p className="eyebrow">Planner and execution records</p>
        <h2>Documents</h2>
        <p>Newest first. Resolve, open, and copy-path outcomes are reported separately.</p>
      </header>
      <div className="sprint-documents__list">
        {documents.map((document) => (
          <article key={document.documentRefId} className="sprint-document-card">
            <button
              type="button"
              className="sprint-document-card__content"
              onClick={() => void run('openWithSystemDefault', document)}
            >
              <span className="sprint-document-card__title">
                <FileUp size={17} aria-hidden="true" />
                <strong>{document.title}</strong>
                <small>{document.displayCategory.value ?? document.classification}</small>
              </span>
              <span>
                Recorded{' '}
                {document.recordedAt.value ? (
                  <time dateTime={document.recordedAt.value}>
                    {formatRecordedAt(document.recordedAt.value)}
                  </time>
                ) : (
                  unavailableReason(document.recordedAt.source)
                )}
              </span>
              <span>Provenance: {document.provenanceReference}</span>
              <span className="sprint-document-card__links">
                {document.sprintPlanRevisionIds.map((id) => (
                  <span key={id}>Plan {id}</span>
                ))}
                {document.sprintPlannerActivityIds.map((id) => (
                  <span key={id}>Planner Activity {id}</span>
                ))}
                {document.workUnitScopeIds.map((id) => (
                  <span key={id}>Work Unit scope {id}</span>
                ))}
              </span>
            </button>
            <div
              className="sprint-document-card__actions"
              aria-label={`${document.title} artifact actions`}
            >
              <button type="button" onClick={() => void run('resolveForOpen', document)}>
                Resolve
              </button>
              <button type="button" onClick={() => void run('openWithSystemDefault', document)}>
                Open
              </button>
              <button type="button" onClick={() => void run('copyPath', document)}>
                Copy path
              </button>
            </div>
          </article>
        ))}
      </div>
      <p className="sprint-documents__status" role="status" aria-live="polite">
        {feedback?.message}
        {feedback?.operation === 'copy_path' &&
        feedback.status === 'observed_success' &&
        feedback.rawPath ? (
          <code>{feedback.rawPath}</code>
        ) : null}
      </p>
    </div>
  );
}

function formatRecordedAt(value: string) {
  return new Intl.DateTimeFormat('en', {
    dateStyle: 'medium',
    timeStyle: 'short',
    timeZone: 'UTC',
  }).format(new Date(value));
}

function unavailableReason(
  source: SprintWorkspacePresentationV1['documents'][number]['recordedAt']['source'],
) {
  return source.status === 'available' ? 'not recorded' : source.reason;
}
