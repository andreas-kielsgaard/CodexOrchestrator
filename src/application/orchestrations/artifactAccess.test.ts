import {
  ARTIFACT_ACCESS_CONTRACTS_V1,
  decodeArtifactAccessContractsV1,
  projectArtifactAccessOutcome,
  type ArtifactAccessContractsV1,
} from './index';

describe('Artifact access contracts', () => {
  it('keeps internal artifacts distinct from user-facing Documents and excludes technical plans', () => {
    const decoded = decodeArtifactAccessContractsV1(contracts());
    expect(decoded.artifacts.find(({ kind }) => kind === 'epic_plan')).toBeDefined();
    expect(decoded.documents.map(({ classification }) => classification)).not.toContain(
      'epic_plan',
    );
    expect(decoded.documents[0].artifactIds).toEqual(['artifact-handoff']);
  });

  it.each([
    ['artifactPath', 'C:\\private\\handoff.md'],
    ['storageLocator', 'opaque-adapter-value'],
    ['providerReference', 'provider-value'],
  ])('rejects %s leakage from read contracts', (key, value) => {
    const invalid = contracts() as Record<string, unknown>;
    (invalid.artifacts as Record<string, unknown>[])[0][key] = value;
    expect(() => decodeArtifactAccessContractsV1(invalid)).toThrow('not allowed');
  });

  it('rejects dangling artifact and changed-file links', () => {
    const artifact = contracts();
    artifact.documents[0].artifactIds = ['missing-artifact'] as never;
    expect(() => decodeArtifactAccessContractsV1(artifact)).toThrow('dangling document artifact');

    const changedFile = contracts();
    changedFile.documents[0].changedFileReferenceIds = ['missing-change'];
    expect(() => decodeArtifactAccessContractsV1(changedFile)).toThrow(
      'dangling document changed file',
    );
  });

  it('rejects user-document inspection of an internal-only artifact', () => {
    const internalOnly = contracts();
    internalOnly.requests[0].artifactId = 'artifact-plan' as never;
    (internalOnly.requests[0].idempotency as { scopeId: string }).scopeId = 'artifact-plan';

    expect(() => decodeArtifactAccessContractsV1(internalOnly)).toThrow(
      'user document inspection requires an explicit Document artifact link',
    );
    expect(() => decodeArtifactAccessContractsV1(contracts())).not.toThrow();
  });

  it('does not infer opening from a request or successful resolution', () => {
    const requested = decodeArtifactAccessContractsV1(contracts());
    expect(projectArtifactAccessOutcome(requested, 'request-open')).toEqual({
      requested: true,
      resolutionObserved: false,
      openedWithSystemDefault: false,
      copyPathObserved: false,
    });

    const resolved = contracts();
    resolved.results.push(observed('result-resolved', 'request-resolve'));
    expect(
      projectArtifactAccessOutcome(decodeArtifactAccessContractsV1(resolved), 'request-open'),
    ).toEqual({
      requested: true,
      resolutionObserved: false,
      openedWithSystemDefault: false,
      copyPathObserved: false,
    });
  });

  it.each(['unsupported', 'denied', 'failed'] as const)(
    'retains %s separately from observed success',
    (status) => {
      const input = contracts();
      input.results.push({
        artifactAccessOperationResultId: `result-${status}`,
        artifactAccessOperationRequestId: 'request-open',
        status,
        recordedAt: TIME,
      });
      expect(
        projectArtifactAccessOutcome(decodeArtifactAccessContractsV1(input), 'request-open'),
      ).toMatchObject({ requested: true, openedWithSystemDefault: false });
    },
  );

  it('permits a raw path only in an observed successful copy-path result', () => {
    const copied = contracts();
    copied.results.push({
      ...observed('result-copy', 'request-copy'),
      rawPath: 'C:\\safe\\handoff.md',
    });
    expect(
      projectArtifactAccessOutcome(decodeArtifactAccessContractsV1(copied), 'request-copy'),
    ).toMatchObject({ copyPathObserved: true, openedWithSystemDefault: false });

    const invalid = contracts();
    invalid.results.push({
      ...observed('result-open-path', 'request-open'),
      rawPath: 'C:\\unsafe.md',
    });
    expect(() => decodeArtifactAccessContractsV1(invalid)).toThrow('raw path is allowed only');
  });

  it.each([
    [
      'request evidence',
      (input: Mutable<ArtifactAccessContractsV1>) => {
        input.requests[0].requestEvidenceReference = 'evidence at C:\\private\\evidence.txt';
      },
    ],
    [
      'result message',
      (input: Mutable<ArtifactAccessContractsV1>) => {
        input.results.push({
          artifactAccessOperationResultId: 'result-message',
          artifactAccessOperationRequestId: 'request-open',
          status: 'failed',
          message: 'Failed to open C:\\private\\failure.txt',
          recordedAt: TIME,
        });
      },
    ],
    [
      'observed effect',
      (input: Mutable<ArtifactAccessContractsV1>) => {
        input.results.push({
          ...observed('result-effect', 'request-open'),
          observedEffectReference: 'C:\\private\\effect.txt',
        });
      },
    ],
  ])('rejects a raw path in %s', (_label, mutate) => {
    const input = contracts();
    mutate(input);
    expect(() => decodeArtifactAccessContractsV1(input)).toThrow(
      'raw path is not allowed outside an explicit successful copy-path result',
    );
  });

  it.each([
    'evidence at /private/evidence.txt',
    'evidence at \\\\server\\share\\evidence.txt',
    'evidence at ~/private/evidence.txt',
  ])('rejects embedded non-drive absolute paths', (reference) => {
    const input = contracts();
    input.requests[0].requestEvidenceReference = reference;
    expect(() => decodeArtifactAccessContractsV1(input)).toThrow(
      'raw path is not allowed outside an explicit successful copy-path result',
    );
  });

  it('keeps display-safe relative changed-file names valid', () => {
    const input = contracts();
    input.changedFileReferences[0].displayName = 'src/foo.ts';
    expect(() => decodeArtifactAccessContractsV1(input)).not.toThrow();
  });

  it.each(['https://example.com/review', 'artifact://resolver/handoff'])(
    'does not mistake the provider-neutral URI %s for a raw filesystem path',
    (reference) => {
      const input = contracts();
      input.requests[0].requestEvidenceReference = reference;
      expect(() => decodeArtifactAccessContractsV1(input)).not.toThrow();
    },
  );

  it('continues to reject a file URI as a raw filesystem location', () => {
    const input = contracts();
    input.requests[0].requestEvidenceReference = 'file:///C:/private/evidence.txt';
    expect(() => decodeArtifactAccessContractsV1(input)).toThrow(
      'raw path is not allowed outside an explicit successful copy-path result',
    );
  });
});

const TIME = '2026-07-14T12:00:00.000Z';

function observed(resultId: string, requestId: string) {
  return {
    artifactAccessOperationResultId: resultId,
    artifactAccessOperationRequestId: requestId,
    status: 'observed_success' as const,
    observedEffectReference: `observation-${resultId}`,
    recordedAt: TIME,
  };
}

function contracts(): Mutable<ArtifactAccessContractsV1> {
  return {
    version: ARTIFACT_ACCESS_CONTRACTS_V1,
    artifacts: [
      {
        artifactId: 'artifact-plan',
        kind: 'epic_plan',
        provenanceReference: 'provenance-plan',
        relatedFactReferences: ['epic-1'],
      },
      { artifactId: 'artifact-handoff', kind: 'handoff_note', provenanceReference: 'provenance-1' },
    ],
    changedFileReferences: [
      { changedFileReferenceId: 'change-1', displayName: 'README.md', changeKind: 'modified' },
    ],
    documents: [
      {
        documentRefId: 'document-handoff',
        classification: 'handoff_note',
        title: 'Sprint handoff',
        artifactIds: ['artifact-handoff'],
        changedFileReferenceIds: ['change-1'],
        provenanceReference: 'provenance-1',
      },
    ],
    requests: [
      request('request-resolve', 'resolve_for_open'),
      request('request-open', 'open_with_system_default'),
      request('request-copy', 'copy_path'),
    ],
    results: [],
  } as unknown as Mutable<ArtifactAccessContractsV1>;
}

function request(
  id: string,
  operationKind: 'resolve_for_open' | 'open_with_system_default' | 'copy_path',
) {
  return {
    artifactAccessOperationRequestId: id,
    operationKind,
    artifactId: 'artifact-handoff',
    purpose: 'user_document_inspection' as const,
    idempotency: { key: id, scopeId: 'artifact-handoff' },
    requestedAt: TIME,
    requestEvidenceReference: `evidence-${id}`,
  };
}

type Mutable<T> = {
  -readonly [K in keyof T]: T[K] extends readonly (infer U)[] ? Mutable<U>[] : T[K];
};
