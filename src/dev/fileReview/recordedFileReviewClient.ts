import type {
  FileReviewClient,
  FileReviewDiffLine,
  FileReviewFile,
  FileReviewSnapshot,
  FileReviewSourceSummary,
} from '../../application/fileReview';
import {
  STORED_FILE_REVIEW_ARTIFACT_V1,
  combineFileReviewClients,
  createApplicationOwnedFileReviewClient,
  type ApplicationFileReviewDocument,
  type FileReviewDocumentPort,
  type StoredFileReviewArtifactPort,
} from '../../application/applicationOwnedFileReview';
import type { AppProps } from '../../app/App';
import { createRecordedDevelopmentApplicationComposition } from '../orchestrationSection/recordedOrchestrationClient';
import { recordedProductReadCompositionInput } from '../orchestrationSection/recordedProductReadCompositionInput';

interface RecordedSource {
  /** Adapter-private locator facts are intentionally excluded from the returned snapshot. */
  readonly locator: {
    readonly repository: string;
    readonly worktree?: string;
    readonly revision?: string;
  };
  readonly snapshot: FileReviewSnapshot;
}

const workingTreeSource = source(
  'source-working-tree',
  'working_tree',
  'Working tree changes',
  'Uncommitted changes · review worktree',
);
const stagedSource = source(
  'source-staged',
  'staged',
  'Staged changes',
  'Index snapshot · ready for commit review',
);
const commitRangeSource = source(
  'source-commit-range',
  'commit_range',
  'Commit range',
  'main…exploration · three commits',
);
const generatedSource = source(
  'source-generated',
  'generated_material',
  'Generated material',
  'Bootstrap preview · not persisted',
);
const recordedSources: readonly RecordedSource[] = [
  {
    locator: {
      repository: 'repository:codex-orchestrator',
      worktree: 'worktree:exploration-review',
    },
    snapshot: {
      source: workingTreeSource,
      files: [
        {
          fileId: 'file-review-notes',
          displayPath: 'docs/orchestration/file-diff-viewer-exploration.md',
          changeKind: 'modified',
          additions: 18,
          deletions: 6,
          content: {
            kind: 'markdown',
            text: `# A review surface inside the product

The viewer presents **display-ready facts** from an injected client. It does not resolve a repository, authorize a path, or edit content.

## Review questions

- Is the changed-file list dense enough?
- Should unified or split view be the default?
- Is generated material clearly distinct from persisted material?

| Boundary | Owner |
| --- | --- |
| Repository and worktree selection | Application adapter |
| Markdown and diff presentation | Viewer |
| Writes and edits | Outside this exploration |

> Source labels describe provenance; they do not grant access.

\`\`\`ts
interface FileReviewClient {
  loadSource(sourceId: string): Promise<FileReviewSnapshot>;
}
\`\`\`

<button>Untrusted HTML action</button>
`,
          },
          hunks: [
            {
              hunkId: 'review-notes-contract',
              header: '@@ -8,8 +8,12 @@ Neutral contracts',
              collapsedBefore: [
                contextLine(5, 'The application chooses an authorized source.'),
                contextLine(6, 'Repository identity remains inside the adapter.'),
                contextLine(7, 'The viewer receives display-safe relative paths.'),
              ],
              lines: [
                contextLine(8, '## Contracts'),
                deletedLine(9, 'The screen reads files from the active worktree.'),
                addedLine(9, 'The screen requests an opaque review source.'),
                addedLine(10, 'The adapter returns display-ready file facts.'),
                contextLine(11, ''),
                contextLine(12, '## Review points'),
                deletedLine(13, '- Confirm the basic diff layout.'),
                addedLine(13, '- Confirm the changed-file information density.'),
                addedLine(14, '- Choose the default unified or split layout.'),
              ],
              collapsedAfter: [
                contextLine(15, '- Confirm how generated material is labeled.'),
                contextLine(16, ''),
              ],
            },
          ],
        },
        {
          fileId: 'file-review-screen',
          displayPath: 'src/features/fileReview/FileReviewScreen.tsx',
          changeKind: 'added',
          additions: 42,
          deletions: 0,
          content: {
            kind: 'text',
            language: 'tsx',
            text: `export function FileReviewScreen({ client }: FileReviewScreenProps) {
  const [sourceId, setSourceId] = useState('');
  const [layout, setLayout] = useState<'unified' | 'split'>('unified');

  return <main aria-label="Files and diffs">{/* read-only presentation */}</main>;
}
`,
          },
          hunks: [
            {
              hunkId: 'new-screen',
              header: '@@ -0,0 +1,8 @@',
              lines: [
                addedLine(1, "import { useState } from 'react';"),
                addedLine(2, ''),
                addedLine(3, 'export function FileReviewScreen() {'),
                addedLine(4, "  const [layout, setLayout] = useState('unified');"),
                addedLine(5, ''),
                addedLine(6, '  return <main>Read-only review</main>;'),
                addedLine(7, '}'),
              ],
            },
          ],
        },
        {
          fileId: 'file-review-renamed',
          displayPath: 'src/application/fileReview.ts',
          previousDisplayPath: 'src/application/diffViewer.ts',
          changeKind: 'renamed',
          additions: 24,
          deletions: 9,
          content: {
            kind: 'text',
            language: 'ts',
            text: `export interface FileReviewClient {
  listSources(): Promise<readonly FileReviewSourceSummary[]>;
  loadSource(sourceId: string): Promise<FileReviewSnapshot>;
}
`,
          },
          hunks: [
            {
              hunkId: 'renamed-contract',
              header: '@@ -1,5 +1,8 @@',
              lines: [
                deletedLine(1, 'export interface DiffViewer {'),
                deletedLine(2, '  read(path: string): Promise<string>;'),
                addedLine(1, 'export interface FileReviewClient {'),
                addedLine(2, '  listSources(): Promise<readonly FileReviewSourceSummary[]>;'),
                addedLine(3, '  loadSource(sourceId: string): Promise<FileReviewSnapshot>;'),
                contextLine(4, '}'),
              ],
            },
          ],
        },
        {
          fileId: 'file-review-binary',
          displayPath: 'docs/review/file-review-walkthrough.mp4',
          changeKind: 'modified',
          additions: 0,
          deletions: 0,
          content: {
            kind: 'binary',
            reason:
              'The adapter identified this file as binary. Playback and byte-level comparison are outside the viewer.',
          },
          hunks: [],
        },
        {
          fileId: 'file-review-unsupported',
          displayPath: 'docs/review/layout-study.sketch',
          changeKind: 'deleted',
          additions: 0,
          deletions: 1,
          content: {
            kind: 'unsupported',
            reason:
              'This file type has no safe text or Markdown renderer. The deletion remains visible in the changed-file list.',
          },
          hunks: [],
        },
      ],
    },
  },
  {
    locator: {
      repository: 'repository:codex-orchestrator',
      worktree: 'worktree:exploration-review',
      revision: 'index',
    },
    snapshot: {
      source: stagedSource,
      files: [
        textFile(
          'staged-contract',
          'src/application/fileReview.ts',
          'modified',
          12,
          2,
          'The staged snapshot is collected independently from working-tree changes.',
        ),
        markdownFile(
          'staged-note',
          'docs/orchestration/review-checklist.md',
          'added',
          9,
          0,
          '# Staged review checklist\n\n- Contracts are neutral\n- No write controls are present\n',
        ),
      ],
    },
  },
  {
    locator: {
      repository: 'repository:codex-orchestrator',
      revision: 'range:main...codex/explore-file-diff-viewer',
    },
    snapshot: {
      source: commitRangeSource,
      files: [
        markdownFile(
          'range-summary',
          'docs/orchestration/exploration-summary.md',
          'modified',
          31,
          8,
          '# Commit range review\n\nThree commits contribute to this bounded exploration.\n',
        ),
        textFile(
          'range-test',
          'src/features/fileReview/FileReviewScreen.test.tsx',
          'added',
          48,
          0,
          'Commit-range facts use the same presentation contract.',
        ),
      ],
    },
  },
  {
    locator: {
      repository: 'repository:codex-orchestrator',
      revision: 'generated:epic-bootstrap-preview',
    },
    snapshot: {
      source: generatedSource,
      files: [
        markdownFile(
          'generated-bootstrap',
          'preview/epic-bootstrap.md',
          'added',
          22,
          0,
          '# Generated Epic bootstrap\n\nThis material is a preview and has not been persisted.\n',
        ),
      ],
    },
  },
];

const recordedPresentationFileReviewClient: FileReviewClient = {
  async listSources() {
    return recordedSources.map(({ snapshot }) => structuredClone(snapshot.source));
  },
  async loadSource(sourceId) {
    const recorded = recordedSources.find(({ snapshot }) => snapshot.source.sourceId === sourceId);
    if (!recorded) throw new Error(`Unknown recorded file-review source: ${sourceId}`);
    return structuredClone(recorded.snapshot);
  },
};

const artifactAccess = recordedProductReadCompositionInput.artifactAccess;
const changedFilesById = new Map(
  artifactAccess.changedFileReferences.map((file) => [file.changedFileReferenceId, file]),
);
const documentsById: ReadonlyMap<string, (typeof artifactAccess.documents)[number]> = new Map(
  artifactAccess.documents.map((document) => [document.documentRefId, document]),
);

const recordedFileReviewDocumentPort: FileReviewDocumentPort = {
  async listDocuments() {
    return artifactAccess.documents.map(toApplicationFileReviewDocument);
  },
  async loadDocument(documentRefId) {
    const document = documentsById.get(documentRefId);
    return document ? toApplicationFileReviewDocument(document) : null;
  },
};

const storedFileReviewArtifacts = [
  storedDocument(
    'doc-ecs-r1',
    'artifact-ecs-r1',
    'document-ecs-r1',
    '# Original ECS-R1 plan\n\nThe first recorded plan established the Sprint surface direction.\n',
  ),
  storedDocument(
    'doc-g1',
    'artifact-g1',
    'document-g1',
    '# G1 feedback and ECS-R2 replan\n\nThe recorded feedback split the original refinement into bounded Work Units.\n',
  ),
  storedDocument(
    'doc-ecs2e-review',
    'artifact-ecs2e-review',
    'document-ecs2e-review',
    '# WU-ECS2E corrected visual review\n\nThe second recorded attempt was accepted after the bounded correction.\n',
  ),
  storedDocument(
    'doc-file-review',
    'artifact-file-review',
    'changed-file-review-doc',
    '# In-app file and diff viewer exploration\n\nThis recorded review material is supplied by the stored-artifact read port.\n',
  ),
  storedDocument(
    'doc-rd-review',
    'artifact-rd-review',
    'document-rd-review',
    '# Sprint detail review evidence\n\nThis recorded Document captures the mixed-state Sprint review composition.\n',
  ),
] as const;
const storedFileReviewArtifactsByDocument = new Map(
  storedFileReviewArtifacts.map((artifact) => [artifact.documentRefId, artifact]),
);

const recordedStoredFileReviewArtifactPort: StoredFileReviewArtifactPort = {
  async loadArtifact(request) {
    const artifact = storedFileReviewArtifactsByDocument.get(request.documentRefId);
    if (!artifact || request.artifactId !== artifact.artifactId) return null;
    return {
      ...artifact,
      bytes: artifact.bytes.slice(),
    };
  },
};

const applicationOwnedFileReviewClient = createApplicationOwnedFileReviewClient(
  recordedFileReviewDocumentPort,
  recordedStoredFileReviewArtifactPort,
);

export const recordedFileReviewClient = combineFileReviewClients([
  recordedPresentationFileReviewClient,
  applicationOwnedFileReviewClient,
]);

/** Development-only tab composition. Product boot does not receive this client or surface. */
export function createRecordedFileReviewApplicationComposition(): AppProps {
  return {
    ...createRecordedDevelopmentApplicationComposition(),
    fileReviewClient: recordedFileReviewClient,
    initialSurface: 'file-review',
  };
}

function toApplicationFileReviewDocument(
  document: (typeof artifactAccess.documents)[number],
): ApplicationFileReviewDocument {
  return {
    documentRefId: document.documentRefId,
    classification: document.classification,
    title: document.title,
    ...(document.summary ? { summary: document.summary } : {}),
    artifactIds: document.artifactIds,
    changedFiles: document.changedFileReferenceIds.map((id) => {
      const changedFile = changedFilesById.get(id);
      if (!changedFile) throw new Error(`Missing recorded changed-file reference: ${id}`);
      return changedFile;
    }),
  };
}

function base64(bytes: Uint8Array) {
  return btoa(String.fromCharCode(...bytes));
}

function storedDocument(
  documentRefId: string,
  artifactId: string,
  changedFileReferenceId: string,
  content: string,
) {
  return {
    documentRefId,
    artifactId,
    bytes: new TextEncoder().encode(
      JSON.stringify({
        contractVersion: STORED_FILE_REVIEW_ARTIFACT_V1,
        documentRefId,
        artifactId,
        files: [
          {
            changedFileReferenceId,
            content: {
              encoding: 'utf-8',
              bytesBase64: base64(new TextEncoder().encode(content)),
            },
            hunks: [
              {
                header: '@@ Sprint start to recorded Document @@',
                lines: [
                  {
                    kind: 'deletion',
                    oldLineNumber: 1,
                    text: 'Document state before this Sprint began.',
                  },
                  ...content
                    .trimEnd()
                    .split('\n')
                    .map((text, index) => ({
                      kind: 'addition',
                      newLineNumber: index + 1,
                      text,
                    })),
                ],
              },
            ],
          },
        ],
      }),
    ),
  };
}

function source(
  sourceId: string,
  kind: FileReviewSourceSummary['kind'],
  label: string,
  detail: string,
): FileReviewSourceSummary {
  return { sourceId, kind, label, detail };
}

function contextLine(lineNumber: number, text: string): FileReviewDiffLine {
  return {
    kind: 'context',
    oldLineNumber: lineNumber,
    newLineNumber: lineNumber,
    text,
  };
}

function addedLine(lineNumber: number, text: string): FileReviewDiffLine {
  return { kind: 'addition', newLineNumber: lineNumber, text };
}

function deletedLine(lineNumber: number, text: string): FileReviewDiffLine {
  return { kind: 'deletion', oldLineNumber: lineNumber, text };
}

function textFile(
  fileId: string,
  displayPath: string,
  changeKind: FileReviewFile['changeKind'],
  additions: number,
  deletions: number,
  text: string,
): FileReviewFile {
  return {
    fileId,
    displayPath,
    changeKind,
    additions,
    deletions,
    content: { kind: 'text', text },
    hunks: [
      {
        hunkId: `${fileId}-hunk`,
        header: '@@ -1 +1 @@',
        lines: [deletedLine(1, 'Previous review material.'), addedLine(1, text)],
      },
    ],
  };
}

function markdownFile(
  fileId: string,
  displayPath: string,
  changeKind: FileReviewFile['changeKind'],
  additions: number,
  deletions: number,
  text: string,
): FileReviewFile {
  return {
    ...textFile(fileId, displayPath, changeKind, additions, deletions, text),
    content: { kind: 'markdown', text },
  };
}
