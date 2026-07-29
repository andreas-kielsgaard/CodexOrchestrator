import { FileArchive, FileCode2, FileText, PanelLeftClose, Rows3, ShieldCheck } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type {
  FileReviewDiffLine,
  FileReviewFile,
  FileReviewSource,
} from '../../application/fileReview';
import { AgentMarkdown } from '../agentSessions/AgentMarkdown';
import './fileReview.css';

export function FileReviewScreen({ source }: { readonly source: FileReviewSource }) {
  const [snapshot, setSnapshot] = useState<Awaited<ReturnType<FileReviewSource['load']>> | null>(
    null,
  );
  const [selectedId, setSelectedId] = useState('');
  const [mode, setMode] = useState<'changes' | 'file'>('changes');
  const [layout, setLayout] = useState<'unified' | 'split'>('unified');
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    setSnapshot(null);
    setError('');
    void source.load().then(
      (value) => {
        if (!active) return;
        setSnapshot(value);
        setSelectedId(value.files[0]?.fileId ?? '');
      },
      () => active && setError('This comparison could not be loaded.'),
    );
    return () => {
      active = false;
    };
  }, [source]);

  const selected =
    snapshot?.files.find(({ fileId }) => fileId === selectedId) ?? snapshot?.files[0];
  const totals = useMemo(
    () =>
      snapshot?.files.reduce(
        (result, file) => ({
          additions: result.additions + file.additions,
          deletions: result.deletions + file.deletions,
        }),
        { additions: 0, deletions: 0 },
      ) ?? { additions: 0, deletions: 0 },
    [snapshot],
  );

  return (
    <main className="file-review-screen" aria-label="Files and diffs">
      <header className="file-review-header">
        <div>
          <h1>File and diff review</h1>
          <p>Inspect the scoped worktree comparison without editing or filesystem access.</p>
        </div>
        <span className="file-review-read-only">
          <ShieldCheck size={15} />
          Read only
        </span>
      </header>
      {error ? (
        <p role="alert">{error}</p>
      ) : !snapshot ? (
        <p role="status">Loading review material…</p>
      ) : (
        <div className="file-review-workspace">
          <aside className="file-review-files" aria-label="Changed files">
            <header>
              <strong>{snapshot.files.length} changed files</strong>
              <span>
                <b>+{totals.additions}</b> <i>−{totals.deletions}</i>
              </span>
            </header>
            <nav>
              {snapshot.files.map((file) => (
                <button
                  type="button"
                  key={file.fileId}
                  className={file.fileId === selected?.fileId ? 'active' : ''}
                  onClick={() => {
                    setSelectedId(file.fileId);
                    setMode('changes');
                  }}
                >
                  <ChangeBadge file={file} compact />
                  <span>
                    <strong>{file.displayPath.split('/').at(-1)}</strong>
                    <small>{file.displayPath}</small>
                  </span>
                  <span>
                    <b>+{file.additions}</b> <i>−{file.deletions}</i>
                  </span>
                </button>
              ))}
            </nav>
          </aside>
          {selected ? (
            <section className="file-review-inspector" aria-label={selected.displayPath}>
              <header className="file-review-toolbar">
                <div className="file-review-path">
                  <FileIcon file={selected} />
                  <div>
                    <strong>{selected.displayPath}</strong>
                    <Provenance file={selected} />
                  </div>
                  <ChangeBadge file={selected} />
                </div>
                <div className="file-review-toolbar__modes">
                  <div
                    className="file-review-segment"
                    role="group"
                    aria-label="File inspection mode"
                  >
                    <button
                      type="button"
                      className={mode === 'changes' ? 'active' : ''}
                      aria-pressed={mode === 'changes'}
                      onClick={() => setMode('changes')}
                    >
                      Changes
                    </button>
                    <button
                      type="button"
                      className={mode === 'file' ? 'active' : ''}
                      aria-pressed={mode === 'file'}
                      onClick={() => setMode('file')}
                    >
                      File
                    </button>
                  </div>
                  <div className="file-review-layout-slot">
                    {mode === 'changes' && selected.hunks.length > 0 && (
                      <div className="file-review-segment" role="group" aria-label="Diff layout">
                        <button
                          type="button"
                          className={layout === 'unified' ? 'active' : ''}
                          aria-pressed={layout === 'unified'}
                          onClick={() => setLayout('unified')}
                        >
                          <Rows3 size={14} />
                          Unified
                        </button>
                        <button
                          type="button"
                          className={layout === 'split' ? 'active' : ''}
                          aria-pressed={layout === 'split'}
                          onClick={() => setLayout('split')}
                        >
                          <PanelLeftClose size={14} />
                          Split
                        </button>
                      </div>
                    )}
                  </div>
                </div>
              </header>
              <div className="file-review-body">
                {mode === 'file' ? (
                  <FileContent file={selected} />
                ) : (
                  <Changes file={selected} layout={layout} />
                )}
              </div>
            </section>
          ) : (
            <p>No changed files.</p>
          )}
        </div>
      )}
    </main>
  );
}

function Changes({
  file,
  layout,
}: {
  readonly file: FileReviewFile;
  readonly layout: 'unified' | 'split';
}) {
  if (file.content.kind === 'binary' || file.content.kind === 'unsupported')
    return <Unavailable file={file} />;
  if (!file.hunks.length) return <p>No textual changes are available.</p>;
  return (
    <div className={`file-review-diff file-review-diff--${layout}`}>
      {file.hunks.map((hunk) => (
        <section key={hunk.hunkId}>
          <header>{hunk.header}</header>
          {layout === 'unified' ? <Unified lines={hunk.lines} /> : <Split lines={hunk.lines} />}
        </section>
      ))}
    </div>
  );
}

function Unified({ lines }: { readonly lines: readonly FileReviewDiffLine[] }) {
  return (
    <div>
      {lines.map((line, index) => (
        <div key={index} className={`file-review-line file-review-line--${line.kind}`}>
          <span>{line.oldLineNumber ?? ''}</span>
          <span>{line.newLineNumber ?? ''}</span>
          <code>
            {line.kind === 'addition' ? '+' : line.kind === 'deletion' ? '−' : ' '}
            {line.text}
          </code>
        </div>
      ))}
    </div>
  );
}

function Split({ lines }: { readonly lines: readonly FileReviewDiffLine[] }) {
  const rows: { left?: FileReviewDiffLine; right?: FileReviewDiffLine }[] = [];
  let index = 0;
  while (index < lines.length) {
    if (lines[index].kind === 'context') {
      rows.push({ left: lines[index], right: lines[index] });
      index += 1;
      continue;
    }
    const changed: FileReviewDiffLine[] = [];
    while (index < lines.length && lines[index].kind !== 'context') changed.push(lines[index++]);
    const left = changed.filter(({ kind }) => kind === 'deletion');
    const right = changed.filter(({ kind }) => kind === 'addition');
    for (let row = 0; row < Math.max(left.length, right.length); row += 1)
      rows.push({ left: left[row], right: right[row] });
  }
  return (
    <div className="file-review-split">
      {rows.map((row, key) => (
        <div key={key}>
          <SplitCell line={row.left} side="old" />
          <SplitCell line={row.right} side="new" />
        </div>
      ))}
    </div>
  );
}

function SplitCell({
  line,
  side,
}: {
  readonly line?: FileReviewDiffLine;
  readonly side: 'old' | 'new';
}) {
  return (
    <div className={line ? `file-review-line--${line.kind}` : ''}>
      <span>{side === 'old' ? line?.oldLineNumber : line?.newLineNumber}</span>
      <code>{line?.text ?? ''}</code>
    </div>
  );
}

function FileContent({ file }: { readonly file: FileReviewFile }) {
  if (file.content.kind === 'markdown')
    return (
      <article className="file-review-rendered">
        <AgentMarkdown>{file.content.text}</AgentMarkdown>
      </article>
    );
  if (file.content.kind === 'text')
    return (
      <pre className="file-review-source">
        <code>{file.content.text}</code>
      </pre>
    );
  return <Unavailable file={file} />;
}

function Unavailable({ file }: { readonly file: FileReviewFile }) {
  const content = file.content;
  return (
    <section className="file-review-state">
      <FileArchive size={28} />
      <h2>{content.kind === 'binary' ? 'Binary preview unavailable' : 'File unavailable'}</h2>
      <p>{content.kind === 'binary' || content.kind === 'unsupported' ? content.reason : ''}</p>
    </section>
  );
}

function Provenance({ file }: { readonly file: FileReviewFile }) {
  return (
    <small>
      {file.provenance
        ?.map((value) =>
          value === 'committed-divergence' ? 'Committed divergence' : 'Uncommitted change',
        )
        .join(' + ')}
    </small>
  );
}

function ChangeBadge({
  file,
  compact = false,
}: {
  readonly file: FileReviewFile;
  readonly compact?: boolean;
}) {
  const labels = {
    added: ['Added', 'A'],
    modified: ['Modified', 'M'],
    deleted: ['Deleted', 'D'],
    renamed: ['Renamed', 'R'],
  } as const;
  return (
    <span className={`file-review-change file-review-change--${file.changeKind}`}>
      {labels[file.changeKind][compact ? 1 : 0]}
    </span>
  );
}

function FileIcon({ file }: { readonly file: FileReviewFile }) {
  return file.content.kind === 'markdown' ? (
    <FileText size={17} />
  ) : file.content.kind === 'text' ? (
    <FileCode2 size={17} />
  ) : (
    <FileArchive size={17} />
  );
}
