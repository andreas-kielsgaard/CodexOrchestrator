import {
  STORED_FILE_REVIEW_ARTIFACT_V1,
  createApplicationOwnedFileReviewClient,
  type ApplicationFileReviewDocument,
  type FileReviewDocumentPort,
  type FileReviewSourceError,
  type StoredFileReviewArtifact,
  type StoredFileReviewArtifactPort,
} from './applicationOwnedFileReview';

describe('application-owned file review', () => {
  it('resolves one currently authorized changed-files Document and stored artifact', async () => {
    const document = changedFilesDocument();
    const artifact = storedArtifact(
      artifactPayload({
        files: [
          storedFile({
            content: content('utf-8', utf8('# Review\n\nApplication-owned content.\n')),
          }),
        ],
      }),
    );
    const documents = documentPort(document);
    const artifacts = artifactPort(artifact);
    const client = createApplicationOwnedFileReviewClient(documents, artifacts);

    await expect(client.listSources()).resolves.toEqual([
      {
        sourceId: 'document-review',
        kind: 'application_owned',
        label: 'Accepted changed files',
        detail: 'Stored review material.',
      },
    ]);
    const snapshot = await client.loadSource('document-review');

    expect(documents.loadDocument).toHaveBeenCalledWith('document-review');
    expect(artifacts.loadArtifact).toHaveBeenCalledWith({
      documentRefId: 'document-review',
      artifactId: 'artifact-review',
    });
    expect(snapshot).toMatchObject({
      source: { sourceId: 'document-review', kind: 'application_owned' },
      files: [
        {
          fileId: 'changed-review',
          displayPath: 'docs/review.md',
          changeKind: 'modified',
          additions: 1,
          deletions: 1,
          content: {
            kind: 'markdown',
            text: '# Review\n\nApplication-owned content.\n',
          },
        },
      ],
    });
  });

  it('reauthorizes source identity before resolving the artifact', async () => {
    const artifacts = artifactPort(storedArtifact(artifactPayload()));
    const client = createApplicationOwnedFileReviewClient(
      {
        listDocuments: vi.fn(async () => [changedFilesDocument()]),
        loadDocument: vi.fn(async () => null),
      },
      artifacts,
    );

    await expectCode(client.loadSource('document-review'), 'source_unauthorized');
    expect(artifacts.loadArtifact).not.toHaveBeenCalled();
  });

  it('rejects artifact and payload identity mismatches', async () => {
    const document = changedFilesDocument();
    const wrongEnvelope = {
      ...storedArtifact(artifactPayload()),
      artifactId: 'artifact-other',
    };
    await expectCode(
      createApplicationOwnedFileReviewClient(
        documentPort(document),
        artifactPort(wrongEnvelope),
      ).loadSource(document.documentRefId),
      'identity_mismatch',
    );

    const wrongPayload = storedArtifact(artifactPayload({ documentRefId: 'document-other' }));
    await expectCode(
      createApplicationOwnedFileReviewClient(
        documentPort(document),
        artifactPort(wrongPayload),
      ).loadSource(document.documentRefId),
      'identity_mismatch',
    );
  });

  it('rejects unauthorized changed-file identities and unsafe display names', async () => {
    const extraFile = storedArtifact(
      artifactPayload({
        files: [storedFile(), storedFile({ changedFileReferenceId: 'changed-unlisted' })],
      }),
    );
    await expectCode(
      createApplicationOwnedFileReviewClient(
        documentPort(changedFilesDocument()),
        artifactPort(extraFile),
      ).loadSource('document-review'),
      'identity_mismatch',
    );

    const unsafeDocument = changedFilesDocument({
      changedFiles: [
        {
          changedFileReferenceId: 'changed-review',
          displayName: 'C:\\private\\review.md',
          changeKind: 'modified',
        },
      ],
    });
    await expectCode(
      createApplicationOwnedFileReviewClient(
        documentPort(unsafeDocument),
        artifactPort(storedArtifact(artifactPayload())),
      ).loadSource('document-review'),
      'source_unauthorized',
    );
  });

  it('applies the byte bound before decoding stored content', async () => {
    const artifact = storedArtifact(artifactPayload());
    const client = createApplicationOwnedFileReviewClient(
      documentPort(changedFilesDocument()),
      artifactPort(artifact),
      { maxArtifactBytes: artifact.bytes.byteLength - 1 },
    );

    await expectCode(client.loadSource('document-review'), 'artifact_too_large');
  });

  it('distinguishes unsupported encoding and invalid UTF-8 from binary content', async () => {
    const unsupported = await loadContent(content('utf-16le', Uint8Array.of(65, 0x20, 66, 0x20)));
    expect(unsupported).toEqual({
      kind: 'unsupported',
      reason: 'The stored file uses unsupported utf-16le encoding.',
    });

    const invalidUtf8 = await loadContent(content('utf-8', Uint8Array.of(0xc3, 0x28)));
    expect(invalidUtf8).toEqual({
      kind: 'unsupported',
      reason: 'The stored file bytes are not valid UTF-8.',
    });

    const binary = await loadContent(content('utf-8', Uint8Array.of(0, 1, 2, 3)));
    expect(binary).toEqual({
      kind: 'binary',
      reason: 'The stored file bytes were identified as binary.',
    });
  });

  it('fails closed when the stored diff artifact is binary or not UTF-8 JSON', async () => {
    const binary = {
      ...storedArtifact(artifactPayload()),
      bytes: Uint8Array.of(0, 1, 2),
    };
    await expectCode(
      createApplicationOwnedFileReviewClient(
        documentPort(changedFilesDocument()),
        artifactPort(binary),
      ).loadSource('document-review'),
      'artifact_invalid',
    );

    const invalidUtf8 = {
      ...storedArtifact(artifactPayload()),
      bytes: Uint8Array.of(0xc3, 0x28),
    };
    await expectCode(
      createApplicationOwnedFileReviewClient(
        documentPort(changedFilesDocument()),
        artifactPort(invalidUtf8),
      ).loadSource('document-review'),
      'artifact_invalid',
    );
  });

  it('reports an unavailable stored artifact without inventing review content', async () => {
    const client = createApplicationOwnedFileReviewClient(
      documentPort(changedFilesDocument()),
      artifactPort(null),
    );

    await expectCode(client.loadSource('document-review'), 'artifact_unavailable');
  });

  it('lists no source for Documents that are not unambiguous changed-files records', async () => {
    const document = changedFilesDocument({
      classification: 'review_material',
      artifactIds: ['artifact-a', 'artifact-b'],
    });
    const client = createApplicationOwnedFileReviewClient(
      documentPort(document),
      artifactPort(null),
    );

    await expect(client.listSources()).resolves.toEqual([]);
  });
});

async function loadContent(storedContent: ReturnType<typeof content>) {
  const client = createApplicationOwnedFileReviewClient(
    documentPort(changedFilesDocument()),
    artifactPort(
      storedArtifact(
        artifactPayload({
          files: [storedFile({ content: storedContent })],
        }),
      ),
    ),
  );
  return (await client.loadSource('document-review')).files[0].content;
}

function changedFilesDocument(
  overrides: Partial<ApplicationFileReviewDocument> = {},
): ApplicationFileReviewDocument {
  return {
    documentRefId: 'document-review',
    classification: 'changed_files',
    title: 'Accepted changed files',
    summary: 'Stored review material.',
    artifactIds: ['artifact-review'],
    changedFiles: [
      {
        changedFileReferenceId: 'changed-review',
        displayName: 'docs/review.md',
        changeKind: 'modified',
      },
    ],
    ...overrides,
  };
}

function documentPort(document: ApplicationFileReviewDocument): FileReviewDocumentPort {
  return {
    listDocuments: vi.fn(async () => [document]),
    loadDocument: vi.fn(async (documentRefId) =>
      documentRefId === document.documentRefId ? document : null,
    ),
  };
}

function artifactPort(
  artifact: StoredFileReviewArtifact | null,
): StoredFileReviewArtifactPort & { loadArtifact: ReturnType<typeof vi.fn> } {
  return {
    loadArtifact: vi.fn(async () => artifact),
  };
}

function storedArtifact(payload: unknown): StoredFileReviewArtifact {
  return {
    documentRefId: 'document-review',
    artifactId: 'artifact-review',
    bytes: utf8(JSON.stringify(payload)),
  };
}

function artifactPayload(overrides: Record<string, unknown> = {}) {
  return {
    contractVersion: STORED_FILE_REVIEW_ARTIFACT_V1,
    documentRefId: 'document-review',
    artifactId: 'artifact-review',
    files: [storedFile()],
    ...overrides,
  };
}

function storedFile(overrides: Record<string, unknown> = {}) {
  return {
    changedFileReferenceId: 'changed-review',
    content: content('utf-8', utf8('Current content.\n')),
    hunks: [
      {
        header: '@@ -1 +1 @@',
        lines: [
          { kind: 'deletion', oldLineNumber: 1, text: 'Previous content.' },
          { kind: 'addition', newLineNumber: 1, text: 'Current content.' },
        ],
      },
    ],
    ...overrides,
  };
}

function content(encoding: string, bytes: Uint8Array) {
  return {
    encoding,
    bytesBase64: btoa(String.fromCharCode(...bytes)),
  };
}

function utf8(value: string) {
  return new TextEncoder().encode(value);
}

async function expectCode(promise: Promise<unknown>, code: FileReviewSourceError['code']) {
  await expect(promise).rejects.toMatchObject({ name: 'FileReviewSourceError', code });
}
