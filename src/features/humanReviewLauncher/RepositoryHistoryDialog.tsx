import { AlertTriangle, FolderGit2, GitBranch, GitCompare, Search, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { HumanReviewSource } from '../../application/humanReviewLauncher';

export function RepositoryHistoryDialog({
  sources,
  loading,
  attaching,
  onClose,
  onAttach,
  onUse,
  onCompare,
}: {
  readonly sources: readonly HumanReviewSource[];
  readonly loading: boolean;
  readonly attaching: boolean;
  readonly onClose: () => void;
  readonly onAttach: (sourceRef: string) => void;
  readonly onUse: (sourceRef: string) => void;
  readonly onCompare: (sourceRef: string, trigger: HTMLButtonElement) => void;
}) {
  const [query, setQuery] = useState('');
  const [selectedRef, setSelectedRef] = useState('');
  const closeRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLElement>(null);
  const visible = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    return sources
      .filter((source) => !source.detached && !source.isMain)
      .filter((source) => !normalized || `${source.branch ?? ''} ${source.label}`.toLocaleLowerCase().includes(normalized))
      .sort((left, right) => (left.branch ?? '').localeCompare(right.branch ?? ''));
  }, [query, sources]);
  const selected = visible.find((source) => source.sourceRef === selectedRef) ?? visible[0];
  const main = sources.find((source) => source.isMain);

  useEffect(() => {
    const background = document.querySelector<HTMLElement>('.human-review');
    const previousAriaHidden = background?.getAttribute('aria-hidden') ?? null;
    const previousInert = background?.inert ?? false;
    const previousOverflow = document.body.style.overflow;
    if (background) {
      background.inert = true;
      background.setAttribute('aria-hidden', 'true');
    }
    document.body.style.overflow = 'hidden';
    closeRef.current?.focus();
    const escape = (event: KeyboardEvent) => {
      if (document.querySelector('.commit-history')) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== 'Tab' || !dialogRef.current) return;
      const focusable = Array.from(
        dialogRef.current.querySelectorAll<HTMLElement>(
          'button:not(:disabled), select:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
        ),
      );
      const first = focusable[0];
      const last = focusable.at(-1);
      if (!first || !last) return;
      if (event.shiftKey && (document.activeElement === first || !dialogRef.current.contains(document.activeElement))) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener('keydown', escape);
    return () => {
      document.removeEventListener('keydown', escape);
      document.body.style.overflow = previousOverflow;
      if (background) {
        background.inert = previousInert;
        if (previousAriaHidden === null) background.removeAttribute('aria-hidden');
        else background.setAttribute('aria-hidden', previousAriaHidden);
      }
    };
  }, [onClose]);

  return createPortal(
    <div className="repository-explorer__backdrop">
      <section ref={dialogRef} className="repository-explorer" role="dialog" aria-modal="true" aria-labelledby="repository-explorer-title">
        <header>
          <div>
            <p>Repository explorer</p>
            <h2 id="repository-explorer-title">Repository history</h2>
          </div>
          <button ref={closeRef} type="button" aria-label="Close repository history" onClick={onClose}><X size={18} /></button>
        </header>

        <div className="repository-explorer__controls">
          <label>
            <Search size={17} />
            <span className="sr-only">Search branches and tags</span>
            <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search branches and tags" />
          </label>
          <select
            aria-label="Choose repository version"
            value={selected?.sourceRef ?? ''}
            disabled={loading || visible.length === 0}
            onChange={(event) => setSelectedRef(event.target.value)}
          >
            {visible.map((source) => <option key={source.sourceRef} value={source.sourceRef}>{source.branch ?? source.label}</option>)}
          </select>
          <span>Focused lineage</span>
        </div>

        {loading ? (
          <div className="repository-explorer__loading" role="status">Loading repository history…</div>
        ) : selected ? (
          <>
            <div className="repository-explorer__graph" aria-label={`Focused lineage for ${selected.branch ?? selected.label}`}>
              <div className="repository-explorer__base"><code>{selected.forkRevision}</code><span>Common base</span></div>
              <HistoryLane label={main?.branch ?? selected.comparisonBranch} tone="main" count={selected.behind} suffix="later commits" revision={main?.revision} />
              <HistoryLane
                label={selected.branch ?? selected.label}
                tone="selected"
                count={selected.ahead}
                suffix="unique commits"
                revision={selected.revision}
                status={selected.attached ? 'Review worktree ready' : 'Review worktree needed'}
              />
            </div>

            <section className="repository-explorer__summary">
              <header>
                <GitBranch size={20} />
                <div><p>{readableKind(selected.refKind)}</p><h3>{selected.branch ?? selected.label}</h3></div>
              </header>
              <div className="repository-explorer__facts">
                <span>{selected.mergedDirectly ? 'Included directly' : 'Not merged directly'}</span>
                <span>{selected.ahead} unique commits</span>
                <span>{selected.equivalentPatches ? `${selected.equivalentPatches} equivalent patches` : 'No equivalent patches found'}</span>
                <span>{selected.attached ? 'Review worktree ready' : 'No review worktree'}</span>
              </div>
              <footer>
                {!selected.mergedDirectly && <p><AlertTriangle size={17} /> Later work may have replaced this implementation; Git cannot prove that.</p>}
                <div>
                  <button type="button" className="secondary" disabled={selected.relationship === 'unrelated'} onClick={(event) => onCompare(selected.sourceRef, event.currentTarget)}><GitCompare size={16} /> Compare with {selected.comparisonBranch}</button>
                  {selected.attached ? (
                    <button type="button" onClick={() => onUse(selected.sourceRef)}><FolderGit2 size={16} /> Use attached worktree</button>
                  ) : (
                    <button type="button" disabled={attaching} onClick={() => onAttach(selected.sourceRef)}><FolderGit2 size={16} /> {attaching ? 'Creating worktree…' : 'Create review worktree'}</button>
                  )}
                </div>
              </footer>
            </section>
          </>
        ) : (
          <div className="repository-explorer__loading">No matching branches or tags.</div>
        )}
      </section>
    </div>,
    document.body,
  );
}

function HistoryLane({ label, tone, count, suffix, revision, status }: { readonly label: string; readonly tone: 'main' | 'selected'; readonly count: number; readonly suffix: string; readonly revision?: string; readonly status?: string }) {
  return (
    <div className={`repository-explorer__lane is-${tone}`}>
      <strong>{label}</strong>
      <span className="repository-explorer__rail"><i /><i /><i /><i /><b>{count} {suffix}</b><i className="head" /></span>
      <code>{revision}</code>
      {status && <em className={status.endsWith('ready') ? 'is-ready' : undefined}><FolderGit2 size={14} /> {status}</em>}
    </div>
  );
}

function readableKind(kind: HumanReviewSource['refKind']) {
  if (kind === 'archive') return 'Archived branch';
  if (kind === 'remote_branch') return 'Remote branch';
  if (kind === 'tag') return 'Tag';
  return 'Branch';
}
