import {
  Braces,
  FileArchive,
  FileCode2,
  FileText,
  FolderGit2,
  PanelLeftClose,
  Rows3,
  ShieldCheck,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type {
  FileReviewClient,
  FileReviewDiffHunk,
  FileReviewDiffLine,
  FileReviewFile,
  FileReviewSourceKind,
} from '../../application/fileReview';
import { AgentMarkdown } from '../agentSessions';
import './fileReview.css';

export interface FileReviewScreenProps {
  readonly client: FileReviewClient;
  readonly initialSourceId?: string;
  readonly fixedSource?: boolean;
}

type ContentMode = 'changes' | 'file';
type DiffLayout = 'unified' | 'split';

export function FileReviewScreen({
  client,
  initialSourceId,
  fixedSource = false,
}: FileReviewScreenProps) {
  const [sources, setSources] = useState<Awaited<ReturnType<FileReviewClient['listSources']>>>([]);
  const [sourcesLoaded, setSourcesLoaded] = useState(false);
  const [selectedSourceId, setSelectedSourceId] = useState('');
  const [snapshot, setSnapshot] = useState<Awaited<
    ReturnType<FileReviewClient['loadSource']>
  > | null>(null);
  const [selectedFileId, setSelectedFileId] = useState('');
  const [contentMode, setContentMode] = useState<ContentMode>('changes');
  const [diffLayout, setDiffLayout] = useState<DiffLayout>('unified');
  const [expandedContext, setExpandedContext] = useState<ReadonlySet<string>>(new Set());
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    void client.listSources().then(
      (nextSources) => {
        if (!active) return;
        setSources(nextSources);
        setSourcesLoaded(true);
        setSelectedSourceId((current) => {
          if (initialSourceId && nextSources.some(({ sourceId }) => sourceId === initialSourceId))
            return initialSourceId;
          return current || nextSources[0]?.sourceId || '';
        });
      },
      () => {
        if (active) {
          setSourcesLoaded(true);
          setError('Review sources could not be loaded.');
        }
      },
    );
    return () => {
      active = false;
    };
  }, [client, initialSourceId]);

  useEffect(() => {
    if (!selectedSourceId) return;
    let active = true;
    setError('');
    setSnapshot(null);
    void client.loadSource(selectedSourceId).then(
      (nextSnapshot) => {
        if (!active) return;
        setSnapshot(nextSnapshot);
        setSelectedFileId(nextSnapshot.files[0]?.fileId ?? '');
        setContentMode(nextSnapshot.source.kind === 'application_owned' ? 'file' : 'changes');
        setExpandedContext(new Set());
      },
      () => {
        if (active) setError('This review source could not be loaded.');
      },
    );
    return () => {
      active = false;
    };
  }, [client, selectedSourceId]);

  const selectedFile = useMemo(
    () =>
      snapshot?.files.find((file) => file.fileId === selectedFileId) ?? snapshot?.files[0] ?? null,
    [selectedFileId, snapshot],
  );
  const totals = useMemo(
    () =>
      snapshot?.files.reduce(
        (sum, file) => ({
          additions: sum.additions + file.additions,
          deletions: sum.deletions + file.deletions,
        }),
        { additions: 0, deletions: 0 },
      ) ?? { additions: 0, deletions: 0 },
    [snapshot],
  );

  const toggleContext = (key: string) => {
    setExpandedContext((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return (
    <main className="file-review-screen" aria-label="Files and diffs">
      <header className="file-review-header">
        <div>
          <p className="eyebrow">Development review surface</p>
          <h1>File and diff review</h1>
          <p>Inspect supplied material without editing or direct filesystem access.</p>
        </div>
        <div className="file-review-header__controls">
          {fixedSource ? (
            <span className="file-review-fixed-source">
              {sources.find(({ sourceId }) => sourceId === selectedSourceId)?.label}
            </span>
          ) : (
            <label>
              <span>Review source</span>
              <select
                aria-label="Review source"
                value={selectedSourceId}
                onChange={(event) => setSelectedSourceId(event.target.value)}
              >
                {sources.map((source) => (
                  <option key={source.sourceId} value={source.sourceId}>
                    {source.label}
                  </option>
                ))}
              </select>
            </label>
          )}
          <span className="file-review-read-only">
            <ShieldCheck size={15} aria-hidden="true" />
            Read only
          </span>
        </div>
      </header>

      {error ? (
        <section className="file-review-state" role="alert">
          <FileArchive size={28} aria-hidden="true" />
          <h2>Review unavailable</h2>
          <p>{error}</p>
        </section>
      ) : !sourcesLoaded ? (
        <section className="file-review-state" role="status">
          <FolderGit2 size={28} aria-hidden="true" />
          <h2>Loading review sources</h2>
        </section>
      ) : sources.length === 0 ? (
        <section className="file-review-state" role="status">
          <FileArchive size={28} aria-hidden="true" />
          <h2>No review sources</h2>
          <p>No authorized review material is currently available.</p>
        </section>
      ) : !snapshot ? (
        <section className="file-review-state" role="status">
          <FolderGit2 size={28} aria-hidden="true" />
          <h2>Loading review material</h2>
        </section>
      ) : (
        <div className="file-review-workspace">
          <aside className="file-review-files" aria-label="Changed files">
            <header>
              <div>
                <strong>{snapshot.files.length} changed files</strong>
                <span>
                  <b>+{totals.additions}</b> <i>−{totals.deletions}</i>
                </span>
              </div>
              <p>
                <SourceKindLabel kind={snapshot.source.kind} />
                <span>{snapshot.source.detail}</span>
              </p>
            </header>
            <nav>
              {snapshot.files.map((file) => (
                <FileNavigationItem
                  key={file.fileId}
                  file={file}
                  selected={file.fileId === selectedFile?.fileId}
                  onSelect={() => {
                    setSelectedFileId(file.fileId);
                    setContentMode('changes');
                    setExpandedContext(new Set());
                  }}
                />
              ))}
            </nav>
          </aside>

          {selectedFile ? (
            <section className="file-review-inspector" aria-label={selectedFile.displayPath}>
              <header className="file-review-toolbar">
                <div className="file-review-path">
                  <FileIcon file={selectedFile} />
                  <div>
                    <strong>{selectedFile.displayPath}</strong>
                    {selectedFile.previousDisplayPath ? (
                      <small>Renamed from {selectedFile.previousDisplayPath}</small>
                    ) : null}
                  </div>
                  <ChangeBadge changeKind={selectedFile.changeKind} />
                </div>
                <div className="file-review-toolbar__modes">
                  <div className="file-review-segment" aria-label="File inspection mode">
                    <button
                      type="button"
                      className={contentMode === 'changes' ? 'active' : undefined}
                      aria-pressed={contentMode === 'changes'}
                      onClick={() => setContentMode('changes')}
                    >
                      {snapshot.source.comparisonLabel ?? 'Changes'}
                    </button>
                    <button
                      type="button"
                      className={contentMode === 'file' ? 'active' : undefined}
                      aria-pressed={contentMode === 'file'}
                      onClick={() => setContentMode('file')}
                    >
                      File
                    </button>
                  </div>
                  {contentMode === 'changes' && selectedFile.hunks.length > 0 ? (
                    <div className="file-review-segment" aria-label="Diff layout">
                      <button
                        type="button"
                        className={diffLayout === 'unified' ? 'active' : undefined}
                        aria-pressed={diffLayout === 'unified'}
                        onClick={() => setDiffLayout('unified')}
                      >
                        <Rows3 size={14} aria-hidden="true" />
                        Unified
                      </button>
                      <button
                        type="button"
                        className={diffLayout === 'split' ? 'active' : undefined}
                        aria-pressed={diffLayout === 'split'}
                        onClick={() => setDiffLayout('split')}
                      >
                        <PanelLeftClose size={14} aria-hidden="true" />
                        Split
                      </button>
                    </div>
                  ) : null}
                </div>
              </header>

              <div className="file-review-inspector__body">
                {contentMode === 'file' ? (
                  <FileContent file={selectedFile} />
                ) : (
                  <DiffContent
                    file={selectedFile}
                    layout={diffLayout}
                    expandedContext={expandedContext}
                    onToggleContext={toggleContext}
                  />
                )}
              </div>
            </section>
          ) : (
            <section className="file-review-state">
              <h2>No changed files</h2>
              <p>The selected source supplied no reviewable file facts.</p>
            </section>
          )}
        </div>
      )}
    </main>
  );
}

function FileNavigationItem({
  file,
  selected,
  onSelect,
}: {
  readonly file: FileReviewFile;
  readonly selected: boolean;
  readonly onSelect: () => void;
}) {
  const { directory, name } = splitDisplayPath(file.displayPath);
  return (
    <button
      type="button"
      className={selected ? 'active' : undefined}
      aria-current={selected ? 'true' : undefined}
      aria-label={`Review ${file.displayPath}`}
      onClick={onSelect}
    >
      <ChangeBadge changeKind={file.changeKind} compact />
      <span>
        <strong>{name}</strong>
        <small>{directory || 'Repository root'}</small>
      </span>
      <span className="file-review-file-counts">
        {file.additions > 0 ? <b>+{file.additions}</b> : null}
        {file.deletions > 0 ? <i>−{file.deletions}</i> : null}
      </span>
    </button>
  );
}

function FileContent({ file }: { readonly file: FileReviewFile }) {
  const content = file.content;
  if (content.kind === 'markdown')
    return (
      <article className="file-review-rendered">
        <header>
          <FileText size={17} aria-hidden="true" />
          Rendered Markdown
        </header>
        <AgentMarkdown>{content.text}</AgentMarkdown>
      </article>
    );
  if (content.kind === 'text')
    return (
      <pre className="file-review-source">
        <code>{content.text}</code>
      </pre>
    );
  return <UnavailableContent kind={content.kind} reason={content.reason} />;
}

function DiffContent({
  file,
  layout,
  expandedContext,
  onToggleContext,
}: {
  readonly file: FileReviewFile;
  readonly layout: DiffLayout;
  readonly expandedContext: ReadonlySet<string>;
  readonly onToggleContext: (key: string) => void;
}) {
  const content = file.content;
  if (content.kind === 'binary' || content.kind === 'unsupported')
    return <UnavailableContent kind={content.kind} reason={content.reason} />;
  if (file.hunks.length === 0)
    return (
      <section className="file-review-state">
        <Rows3 size={28} aria-hidden="true" />
        <h2>No textual changes supplied</h2>
        <p>The selected source did not provide diff hunks for this file.</p>
      </section>
    );

  return (
    <div className={`file-review-diff file-review-diff--${layout}`}>
      {file.hunks.map((hunk) => (
        <DiffHunk
          key={hunk.hunkId}
          hunk={hunk}
          layout={layout}
          expandedContext={expandedContext}
          onToggleContext={onToggleContext}
        />
      ))}
    </div>
  );
}

function DiffHunk({
  hunk,
  layout,
  expandedContext,
  onToggleContext,
}: {
  readonly hunk: FileReviewDiffHunk;
  readonly layout: DiffLayout;
  readonly expandedContext: ReadonlySet<string>;
  readonly onToggleContext: (key: string) => void;
}) {
  const beforeKey = `${hunk.hunkId}:before`;
  const afterKey = `${hunk.hunkId}:after`;
  return (
    <section className="file-review-hunk">
      <header>{hunk.header}</header>
      <ContextControl
        lines={hunk.collapsedBefore}
        placement="above"
        contextKey={beforeKey}
        expanded={expandedContext.has(beforeKey)}
        layout={layout}
        onToggle={onToggleContext}
      />
      {layout === 'unified' ? (
        <UnifiedLines lines={hunk.lines} />
      ) : (
        <SplitLines lines={hunk.lines} />
      )}
      <ContextControl
        lines={hunk.collapsedAfter}
        placement="below"
        contextKey={afterKey}
        expanded={expandedContext.has(afterKey)}
        layout={layout}
        onToggle={onToggleContext}
      />
    </section>
  );
}

function ContextControl({
  lines,
  placement,
  contextKey,
  expanded,
  layout,
  onToggle,
}: {
  readonly lines?: readonly FileReviewDiffLine[];
  readonly placement: 'above' | 'below';
  readonly contextKey: string;
  readonly expanded: boolean;
  readonly layout: DiffLayout;
  readonly onToggle: (key: string) => void;
}) {
  if (!lines?.length) return null;
  return (
    <>
      <button
        type="button"
        className="file-review-context-control"
        onClick={() => onToggle(contextKey)}
      >
        {expanded ? 'Hide' : 'Show'} {lines.length} unchanged lines {placement}
      </button>
      {expanded ? (
        layout === 'unified' ? (
          <UnifiedLines lines={lines} />
        ) : (
          <SplitLines lines={lines} />
        )
      ) : null}
    </>
  );
}

function UnifiedLines({ lines }: { readonly lines: readonly FileReviewDiffLine[] }) {
  return (
    <div className="file-review-unified">
      {lines.map((line, index) => (
        <div
          key={`${line.oldLineNumber ?? 'x'}:${line.newLineNumber ?? 'x'}:${index}`}
          className={`file-review-line file-review-line--${line.kind}`}
        >
          <span className="file-review-line-number">{line.oldLineNumber ?? ''}</span>
          <span className="file-review-line-number">{line.newLineNumber ?? ''}</span>
          <code>
            <span aria-hidden="true">{linePrefix(line)}</span>
            {line.text}
          </code>
        </div>
      ))}
    </div>
  );
}

function SplitLines({ lines }: { readonly lines: readonly FileReviewDiffLine[] }) {
  return (
    <div className="file-review-split">
      {pairSplitLines(lines).map((row, index) => (
        <div key={index} className="file-review-split__row">
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
    <div
      className={`file-review-split__cell ${
        line ? `file-review-line--${line.kind}` : 'file-review-line--empty'
      }`}
    >
      <span className="file-review-line-number">
        {side === 'old' ? line?.oldLineNumber : line?.newLineNumber}
      </span>
      <code>{line?.text ?? ''}</code>
    </div>
  );
}

function UnavailableContent({
  kind,
  reason,
}: {
  readonly kind: 'binary' | 'unsupported';
  readonly reason: string;
}) {
  return (
    <section className="file-review-state">
      {kind === 'binary' ? (
        <FileArchive size={30} aria-hidden="true" />
      ) : (
        <Braces size={30} aria-hidden="true" />
      )}
      <h2>{kind === 'binary' ? 'Binary preview unavailable' : 'File type not supported'}</h2>
      <p>{reason}</p>
    </section>
  );
}

function SourceKindLabel({ kind }: { readonly kind: FileReviewSourceKind }) {
  const labels: Record<FileReviewSourceKind, string> = {
    working_tree: 'Working tree',
    staged: 'Staged changes',
    commit_range: 'Commit range',
    generated_material: 'Generated material',
    application_owned: 'Application-owned',
  };
  return <strong>{labels[kind]}</strong>;
}

function ChangeBadge({
  changeKind,
  compact = false,
}: {
  readonly changeKind: FileReviewFile['changeKind'];
  readonly compact?: boolean;
}) {
  const labels: Record<FileReviewFile['changeKind'], string> = {
    added: compact ? 'A' : 'Added',
    modified: compact ? 'M' : 'Modified',
    deleted: compact ? 'D' : 'Deleted',
    renamed: compact ? 'R' : 'Renamed',
  };
  return (
    <span className={`file-review-change file-review-change--${changeKind}`}>
      {labels[changeKind]}
    </span>
  );
}

function FileIcon({ file }: { readonly file: FileReviewFile }) {
  if (file.content.kind === 'markdown') return <FileText size={17} aria-hidden="true" />;
  if (file.content.kind === 'text') return <FileCode2 size={17} aria-hidden="true" />;
  return <FileArchive size={17} aria-hidden="true" />;
}

function splitDisplayPath(path: string): { directory: string; name: string } {
  const separator = path.lastIndexOf('/');
  return separator < 0
    ? { directory: '', name: path }
    : { directory: path.slice(0, separator), name: path.slice(separator + 1) };
}

function linePrefix(line: FileReviewDiffLine): string {
  return line.kind === 'addition' ? '+' : line.kind === 'deletion' ? '−' : ' ';
}

interface SplitRow {
  readonly left?: FileReviewDiffLine;
  readonly right?: FileReviewDiffLine;
}

function pairSplitLines(lines: readonly FileReviewDiffLine[]): readonly SplitRow[] {
  const rows: SplitRow[] = [];
  let index = 0;
  while (index < lines.length) {
    const current = lines[index];
    if (current.kind === 'context') {
      rows.push({ left: current, right: current });
      index += 1;
      continue;
    }
    const changed: FileReviewDiffLine[] = [];
    while (index < lines.length && lines[index].kind !== 'context') {
      changed.push(lines[index]);
      index += 1;
    }
    const deletions = changed.filter((line) => line.kind === 'deletion');
    const additions = changed.filter((line) => line.kind === 'addition');
    const rowCount = Math.max(deletions.length, additions.length);
    for (let changedIndex = 0; changedIndex < rowCount; changedIndex += 1) {
      rows.push({ left: deletions[changedIndex], right: additions[changedIndex] });
    }
  }
  return rows;
}
