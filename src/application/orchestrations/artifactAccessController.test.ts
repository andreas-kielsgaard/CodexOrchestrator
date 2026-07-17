import { createArtifactAccessController } from './artifactAccessController';

const document = { documentRefId: 'document-1', title: 'Review note', artifactIds: ['artifact-1'] };
const result = (status: 'requested' | 'unsupported' | 'denied' | 'failed', extra = {}) => ({
  artifactAccessOperationResultId: 'result-1',
  artifactAccessOperationRequestId: 'request-1',
  recordedAt: '2026-07-15T10:00:00.000Z',
  status,
  ...extra,
});
const observed = (extra: {
  readonly observedEffectReference: string;
  readonly rawPath?: string;
}) => ({
  ...result('requested', extra),
  status: 'observed_success' as const,
  observedEffectReference: extra.observedEffectReference,
});

describe('ArtifactAccessController', () => {
  it('keeps resolve, open, and copy operations separate and does not infer open from resolve', async () => {
    const resolveForOpen = vi.fn(() => observed({ observedEffectReference: 'artifact:resolved' }));
    const openWithSystemDefault = vi.fn(() =>
      result('unsupported', { message: 'No native opener.' }),
    );
    const copyPath = vi.fn(() => ({
      ...result('requested'),
      status: 'observed_success' as const,
      observedEffectReference: 'clipboard:copied',
      rawPath: 'C:/safe/explicit-copy.md',
    }));
    const controller = createArtifactAccessController(
      { resolveForOpen, openWithSystemDefault, copyPath },
      { now: () => '2026-07-15T10:00:00.000Z', nextRequestId: () => 'request-1' },
    );

    expect(await controller.resolveForOpen(document)).toMatchObject({
      status: 'observed_success',
      operation: 'resolve_for_open',
    });
    expect(await controller.openWithSystemDefault(document)).toMatchObject({
      status: 'unsupported',
      operation: 'open_with_system_default',
    });
    expect(await controller.copyPath(document)).toMatchObject({
      status: 'observed_success',
      rawPath: 'C:/safe/explicit-copy.md',
    });
    expect(resolveForOpen).toHaveBeenCalledWith(
      expect.objectContaining({ operationKind: 'resolve_for_open' }),
    );
    expect(openWithSystemDefault).toHaveBeenCalledWith(
      expect.objectContaining({ operationKind: 'open_with_system_default' }),
    );
    expect(copyPath).toHaveBeenCalledWith(expect.objectContaining({ operationKind: 'copy_path' }));
  });

  it.each(['requested', 'unsupported', 'denied', 'failed'] as const)(
    'reports %s without success or a raw path',
    async (status) => {
      const controller = createArtifactAccessController({
        resolveForOpen: () => result(status),
        openWithSystemDefault: () => result(status),
        copyPath: () => result(status),
      });
      const feedback = await controller.copyPath(document);
      expect(feedback.status).toBe(status);
      expect(feedback).not.toHaveProperty('rawPath');
    },
  );

  it('rejects file references in non-copy feedback while allowing opaque and normal URL references', async () => {
    const controller = createArtifactAccessController({
      resolveForOpen: () => observed({ observedEffectReference: 'https://example.test/artifact' }),
      openWithSystemDefault: () => observed({ observedEffectReference: 'urn:artifact:1' }),
      copyPath: () => result('failed', { message: 'file:///C:/secret.md' }),
    });
    expect((await controller.resolveForOpen(document)).status).toBe('observed_success');
    expect((await controller.openWithSystemDefault(document)).status).toBe('observed_success');
    expect(await controller.copyPath(document)).toMatchObject({
      status: 'failed',
      message: expect.stringContaining('prohibited path'),
    });
  });
});
