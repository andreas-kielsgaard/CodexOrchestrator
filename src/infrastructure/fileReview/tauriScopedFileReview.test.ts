import { FileReviewSourceError } from '../../application/applicationOwnedFileReview';
import { createTauriScopedFileReviewPorts } from './tauriScopedFileReview';

const available = {
  status: 'available' as const,
  document: {
    documentRefId: 'doc',
    title: 'Review',
    artifactId: 'artifact',
    payload: [1, 2],
    changedFiles: [
      { changedFileReferenceId: 'file', displayName: 'src/a.ts', changeKind: 'modified' },
    ],
  },
};

describe('scoped File Review Tauri adapter', () => {
  it('uses only the opaque reference and maps available facts', async () => {
    const invoke = vi.fn(async () => available);
    const ports = createTauriScopedFileReviewPorts('opaque', invoke);
    await expect(ports.documents.loadDocument()).resolves.toMatchObject({
      documentRefId: 'doc',
      artifactIds: ['artifact'],
    });
    expect(invoke).toHaveBeenCalledWith('load_scoped_file_review', {
      input: { opaqueReference: 'opaque' },
    });
    await expect(
      ports.artifacts.loadArtifact({ documentRefId: 'doc', artifactId: 'artifact' }),
    ).resolves.toMatchObject({ bytes: new Uint8Array([1, 2]) });
  });
  it.each(['unavailable', 'unauthorized', 'invalid'] as const)('maps %s safely', async (status) => {
    const ports = createTauriScopedFileReviewPorts('opaque', async () => ({ status }));
    if (status === 'unavailable') await expect(ports.documents.loadDocument()).resolves.toBeNull();
    else await expect(ports.documents.loadDocument()).rejects.toBeInstanceOf(FileReviewSourceError);
  });
  it('rejects caller-supplied identity mismatch without widening authority', async () => {
    const ports = createTauriScopedFileReviewPorts('opaque', async () => available);
    await expect(
      ports.artifacts.loadArtifact({ documentRefId: 'other', artifactId: 'artifact' }),
    ).resolves.toBeNull();
    await expect(
      ports.artifacts.loadArtifact({ documentRefId: 'doc', artifactId: 'other' }),
    ).resolves.toBeNull();
  });
});
