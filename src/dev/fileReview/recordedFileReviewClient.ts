import type {
  FileReviewDiffLine,
  FileReviewFile,
  FileReviewSnapshot,
  FileReviewSource,
} from '../../application/fileReview';
import {
  STORED_FILE_REVIEW_ARTIFACT_V1,
  createApplicationOwnedFileReviewSource,
  type ApplicationFileReviewDocument,
  type FileReviewDocumentPort,
  type StoredFileReviewArtifactPort,
} from '../../application/applicationOwnedFileReview';
import type { AppProps } from '../../app/App';
import { createRecordedDevelopmentApplicationComposition } from '../orchestrationSection/recordedOrchestrationClient';
import { recordedProductReadCompositionInput } from '../orchestrationSection/recordedProductReadCompositionInput';

export type RecordedFileReviewFixtureId =
  'working-tree' | 'staged' | 'commit-range' | 'generated' | 'application-owned';

interface RecordedFixture {
  readonly fixtureId: RecordedFileReviewFixtureId;
  readonly label: string;
  /** Adapter-private locator facts are intentionally excluded from the returned snapshot. */
  readonly locator: {
    readonly repository: string;
    readonly worktree?: string;
    readonly revision?: string;
  };
  readonly snapshot: FileReviewSnapshot;
}

const reviewNotesBefore = `# A review surface inside the product

The viewer presents **display-ready facts** from an injected client. It does not resolve a repository, authorize a path, or edit content.

## Review questions

- Is the basic diff layout clear?

| Boundary | Owner |
| --- | --- |
| Repository and worktree selection | Application adapter |
| Markdown and diff presentation | Viewer |
| Writes and edits | Outside this exploration |

> Review material is scoped before it reaches the viewer.

\`\`\`ts
interface FileReviewSource {
  load(): Promise<FileReviewSnapshot>;
}
\`\`\`

<button>Untrusted HTML action</button>
`;

const reviewNotesAfter = `# A review surface inside the product

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

> Review material is scoped before it reaches the viewer.

\`\`\`ts
interface FileReviewSource {
  load(): Promise<FileReviewSnapshot>;
}
\`\`\`

<button>Untrusted HTML action</button>
`;

const recordedPresentationFixtures: readonly RecordedFixture[] = [
  {
    fixtureId: 'working-tree',
    label: 'Working tree changes',
    locator: {
      repository: 'repository:codex-orchestrator',
      worktree: 'worktree:exploration-review',
    },
    snapshot: {
      files: [
        completeMarkdownFile({
          fileId: 'file-review-notes',
          displayPath: 'docs/orchestration/file-diff-viewer-exploration.md',
          changeKind: 'modified',
          previousText: reviewNotesBefore,
          text: reviewNotesAfter,
          collapseLeadingContext: 3,
          collapseTrailingContext: 2,
        }),
        completeTextFile({
          fileId: 'file-review-screen',
          displayPath: 'src/features/fileReview/FileReviewScreen.tsx',
          changeKind: 'added',
          language: 'tsx',
          text: `import { useState } from 'react';

export function FileReviewScreen() {
  const [layout, setLayout] = useState('unified');

  return <main>Read-only review</main>;
}
`,
        }),
        completeTextFile({
          fileId: 'file-review-renamed',
          displayPath: 'src/application/fileReview.ts',
          previousDisplayPath: 'src/application/diffViewer.ts',
          changeKind: 'renamed',
          language: 'ts',
          previousText: `export interface FileReviewSelection {
  select(sourceId: string): Promise<FileReviewSnapshot>;
}
`,
          text: `export interface FileReviewSource {
  load(): Promise<FileReviewSnapshot>;
}
`,
        }),
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
    fixtureId: 'staged',
    label: 'Staged changes',
    locator: {
      repository: 'repository:codex-orchestrator',
      worktree: 'worktree:exploration-review',
      revision: 'index',
    },
    snapshot: {
      files: [
        completeTextFile({
          fileId: 'staged-contract',
          displayPath: 'src/application/fileReview.ts',
          changeKind: 'modified',
          previousText: 'Review material comes from the active checkout.\n',
          text: 'The staged snapshot is collected independently from working-tree changes.\n',
        }),
        completeMarkdownFile({
          fileId: 'staged-note',
          displayPath: 'docs/orchestration/review-checklist.md',
          changeKind: 'added',
          text: '# Staged review checklist\n\n- Contracts are neutral\n- No write controls are present\n',
        }),
      ],
    },
  },
  {
    fixtureId: 'commit-range',
    label: 'Commit range',
    locator: {
      repository: 'repository:codex-orchestrator',
      revision: 'range:main...codex/explore-file-diff-viewer',
    },
    snapshot: {
      files: [
        completeMarkdownFile({
          fileId: 'range-summary',
          displayPath: 'docs/orchestration/exploration-summary.md',
          changeKind: 'modified',
          previousText:
            '# Commit range review\n\nTwo commits contribute to this bounded exploration.\n',
          text: '# Commit range review\n\nThree commits contribute to this bounded exploration.\n',
        }),
        completeTextFile({
          fileId: 'range-test',
          displayPath: 'src/features/fileReview/FileReviewScreen.test.tsx',
          changeKind: 'added',
          text: 'Commit-range facts use the same presentation contract.\n',
        }),
      ],
    },
  },
  {
    fixtureId: 'generated',
    label: 'Generated material',
    locator: {
      repository: 'repository:codex-orchestrator',
      revision: 'generated:epic-bootstrap-preview',
    },
    snapshot: {
      files: [
        completeMarkdownFile({
          fileId: 'generated-bootstrap',
          displayPath: 'preview/epic-bootstrap.md',
          changeKind: 'added',
          text: '# Generated Epic bootstrap\n\nThis material is a preview and has not been persisted.\n',
        }),
      ],
    },
  },
];

const artifactAccess = recordedProductReadCompositionInput.artifactAccess;
const changedFilesById = new Map(
  artifactAccess.changedFileReferences.map((file) => [file.changedFileReferenceId, file]),
);
const documentsById: ReadonlyMap<string, (typeof artifactAccess.documents)[number]> = new Map(
  artifactAccess.documents.map((document) => [document.documentRefId, document]),
);

const recordedFileReviewDocumentPort: FileReviewDocumentPort = {
  async loadDocument() {
    const document = documentsById.get('doc-file-review');
    return document ? toApplicationFileReviewDocument(document) : null;
  },
};

const applicationOwnedReviewText = `# In-app file and diff viewer exploration

## Exact next product slice

Resolve an authorized changed-files Document.
Load its stored diff artifact through a read-only port.
`;

const storedFileReviewArtifact = {
  documentRefId: 'doc-file-review',
  artifactId: 'artifact-file-review',
  bytes: new TextEncoder().encode(
    JSON.stringify({
      contractVersion: STORED_FILE_REVIEW_ARTIFACT_V1,
      documentRefId: 'doc-file-review',
      artifactId: 'artifact-file-review',
      files: [
        {
          changedFileReferenceId: 'changed-file-review-doc',
          content: {
            encoding: 'utf-8',
            bytesBase64: base64(new TextEncoder().encode(applicationOwnedReviewText)),
          },
          hunks: [
            {
              header: '@@ -1,5 +1,6 @@',
              lines: [
                {
                  kind: 'context',
                  oldLineNumber: 1,
                  newLineNumber: 1,
                  text: '# In-app file and diff viewer exploration',
                },
                {
                  kind: 'context',
                  oldLineNumber: 2,
                  newLineNumber: 2,
                  text: '',
                },
                {
                  kind: 'context',
                  oldLineNumber: 3,
                  newLineNumber: 3,
                  text: '## Exact next product slice',
                },
                {
                  kind: 'context',
                  oldLineNumber: 4,
                  newLineNumber: 4,
                  text: '',
                },
                {
                  kind: 'deletion',
                  oldLineNumber: 5,
                  text: 'Use a recorded application-owned source.',
                },
                {
                  kind: 'addition',
                  newLineNumber: 5,
                  text: 'Resolve an authorized changed-files Document.',
                },
                {
                  kind: 'addition',
                  newLineNumber: 6,
                  text: 'Load its stored diff artifact through a read-only port.',
                },
              ],
            },
          ],
        },
      ],
    }),
  ),
} as const;

const recordedStoredFileReviewArtifactPort: StoredFileReviewArtifactPort = {
  async loadArtifact(request) {
    if (
      request.documentRefId !== storedFileReviewArtifact.documentRefId ||
      request.artifactId !== storedFileReviewArtifact.artifactId
    )
      return null;
    return {
      ...storedFileReviewArtifact,
      bytes: storedFileReviewArtifact.bytes.slice(),
    };
  },
};

const applicationOwnedFileReviewSource = createApplicationOwnedFileReviewSource(
  recordedFileReviewDocumentPort,
  recordedStoredFileReviewArtifactPort,
);

export const recordedFileReviewFixtures: readonly {
  readonly fixtureId: RecordedFileReviewFixtureId;
  readonly label: string;
}[] = [
  ...recordedPresentationFixtures.map(({ fixtureId, label }) => ({ fixtureId, label })),
  { fixtureId: 'application-owned', label: 'Application-owned Document' },
];

export function createRecordedFileReviewSource(
  fixtureId: RecordedFileReviewFixtureId = 'working-tree',
): FileReviewSource {
  if (fixtureId === 'application-owned') return applicationOwnedFileReviewSource;
  const fixture = recordedPresentationFixtures.find(
    (candidate) => candidate.fixtureId === fixtureId,
  );
  if (!fixture) throw new Error(`Unknown recorded file-review fixture: ${fixtureId}`);
  return {
    async load() {
      return structuredClone(fixture.snapshot);
    },
  };
}

/** Development-only tab composition. Product boot does not receive this client or surface. */
export function createRecordedFileReviewApplicationComposition(
  fixtureId: RecordedFileReviewFixtureId = 'working-tree',
): AppProps {
  return {
    ...createRecordedDevelopmentApplicationComposition(),
    fileReviewSource: createRecordedFileReviewSource(fixtureId),
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

function contextLine(
  oldLineNumber: number,
  newLineNumber: number,
  text: string,
): FileReviewDiffLine {
  return {
    kind: 'context',
    oldLineNumber,
    newLineNumber,
    text,
  };
}

function addedLine(lineNumber: number, text: string): FileReviewDiffLine {
  return { kind: 'addition', newLineNumber: lineNumber, text };
}

function deletedLine(lineNumber: number, text: string): FileReviewDiffLine {
  return { kind: 'deletion', oldLineNumber: lineNumber, text };
}

interface CompleteTextFileInput {
  readonly fileId: string;
  readonly displayPath: string;
  readonly previousDisplayPath?: string;
  readonly changeKind: FileReviewFile['changeKind'];
  readonly previousText?: string;
  readonly text: string;
  readonly language?: string;
  readonly collapseLeadingContext?: number;
  readonly collapseTrailingContext?: number;
}

function completeTextFile(input: CompleteTextFileInput): FileReviewFile {
  const previousLines = contentLines(input.previousText ?? '');
  const nextLines = contentLines(input.text);
  let commonPrefix = 0;
  while (
    commonPrefix < previousLines.length &&
    commonPrefix < nextLines.length &&
    previousLines[commonPrefix] === nextLines[commonPrefix]
  )
    commonPrefix += 1;

  let commonSuffix = 0;
  while (
    commonSuffix < previousLines.length - commonPrefix &&
    commonSuffix < nextLines.length - commonPrefix &&
    previousLines[previousLines.length - 1 - commonSuffix] ===
      nextLines[nextLines.length - 1 - commonSuffix]
  )
    commonSuffix += 1;

  const lines: FileReviewDiffLine[] = [];
  for (let index = 0; index < commonPrefix; index += 1)
    lines.push(contextLine(index + 1, index + 1, nextLines[index]));
  for (let index = commonPrefix; index < previousLines.length - commonSuffix; index += 1)
    lines.push(deletedLine(index + 1, previousLines[index]));
  for (let index = commonPrefix; index < nextLines.length - commonSuffix; index += 1)
    lines.push(addedLine(index + 1, nextLines[index]));
  for (let offset = commonSuffix; offset > 0; offset -= 1) {
    const oldIndex = previousLines.length - offset;
    const newIndex = nextLines.length - offset;
    lines.push(contextLine(oldIndex + 1, newIndex + 1, nextLines[newIndex]));
  }

  const collapsedBeforeCount = Math.min(input.collapseLeadingContext ?? 0, commonPrefix);
  const collapsedAfterCount = Math.min(input.collapseTrailingContext ?? 0, commonSuffix);
  const additions = lines.filter(({ kind }) => kind === 'addition').length;
  const deletions = lines.filter(({ kind }) => kind === 'deletion').length;
  return {
    fileId: input.fileId,
    displayPath: input.displayPath,
    ...(input.previousDisplayPath ? { previousDisplayPath: input.previousDisplayPath } : {}),
    changeKind: input.changeKind,
    additions,
    deletions,
    content: {
      kind: 'text',
      text: input.text,
      ...(input.language ? { language: input.language } : {}),
    },
    hunks: [
      {
        hunkId: `${input.fileId}-complete`,
        header: `@@ -${hunkRange(previousLines.length)} +${hunkRange(nextLines.length)} @@`,
        ...(collapsedBeforeCount > 0
          ? { collapsedBefore: lines.slice(0, collapsedBeforeCount) }
          : {}),
        lines: lines.slice(collapsedBeforeCount, lines.length - collapsedAfterCount),
        ...(collapsedAfterCount > 0
          ? { collapsedAfter: lines.slice(lines.length - collapsedAfterCount) }
          : {}),
      },
    ],
  };
}

function completeMarkdownFile(input: CompleteTextFileInput): FileReviewFile {
  return {
    ...completeTextFile(input),
    content: {
      kind: 'markdown',
      text: input.text,
      ...(input.language ? { language: input.language } : {}),
    },
  };
}

function contentLines(text: string): readonly string[] {
  if (!text) return [];
  return (text.endsWith('\n') ? text.slice(0, -1) : text).split('\n');
}

function hunkRange(lineCount: number): string {
  if (lineCount === 0) return '0,0';
  return lineCount === 1 ? '1' : `1,${lineCount}`;
}
