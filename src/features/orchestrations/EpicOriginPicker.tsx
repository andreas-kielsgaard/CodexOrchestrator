import { CheckCircle2, Folder, GitBranch, Info, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type {
  EpicOriginBranch,
  EpicOriginProject,
  EpicOriginProjectClient,
} from '../../application/epicOriginProject';
import './styles/epicOriginPicker.css';

const STORAGE_KEY = 'codex-orchestrator:epic-origin-selection:v1';

interface StoredOriginSelection {
  readonly projectPath: string;
  readonly branchName?: string;
}

export function EpicOriginPicker({ client }: { readonly client?: EpicOriginProjectClient }) {
  const [project, setProject] = useState<EpicOriginProject | null>(null);
  const [originBranch, setOriginBranch] = useState<string | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [hydrated, setHydrated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const branchTrigger = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!client) return;
    let active = true;
    const stored = readStoredSelection();
    if (!stored) {
      setHydrated(true);
      return;
    }
    setBusy(true);
    void client
      .inspectProject(stored.projectPath)
      .then((value) => {
        if (!active) return;
        setProject(value);
        setOriginBranch(
          stored.branchName && value.branches.some((branch) => branch.name === stored.branchName)
            ? stored.branchName
            : null,
        );
      })
      .catch(() => {
        if (active) window.localStorage.removeItem(STORAGE_KEY);
      })
      .finally(() => {
        if (active) {
          setBusy(false);
          setHydrated(true);
        }
      });
    return () => {
      active = false;
    };
  }, [client]);

  useEffect(() => {
    if (!hydrated) return;
    if (!project) {
      window.localStorage.removeItem(STORAGE_KEY);
      return;
    }
    const stored: StoredOriginSelection = {
      projectPath: project.path,
      ...(originBranch ? { branchName: originBranch } : {}),
    };
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
  }, [hydrated, originBranch, project]);

  const chooseProject = async () => {
    if (!client || busy) return;
    setBusy(true);
    setError(null);
    try {
      const selected = await client.chooseProject();
      if (!selected) return;
      setProject(selected);
      setOriginBranch(null);
      setPickerOpen(false);
    } catch {
      setError('The project folder could not be inspected.');
    } finally {
      setBusy(false);
      setHydrated(true);
    }
  };

  const selectedBranch = project?.branches.find((branch) => branch.name === originBranch);
  const canChooseBranch = Boolean(project?.gitDetected && project.branches.length > 0);
  const closePicker = useCallback(() => {
    setPickerOpen(false);
    window.requestAnimationFrame(() => branchTrigger.current?.focus());
  }, []);
  const confirmOrigin = useCallback((branchName: string) => {
    setOriginBranch(branchName);
    setPickerOpen(false);
    window.requestAnimationFrame(() => branchTrigger.current?.focus());
  }, []);

  if (!client) return null;

  return (
    <div className="epic-origin-picker">
      <section className="epic-origin-picker__project" aria-labelledby="epic-project-folder-label">
        <h2 id="epic-project-folder-label">Project folder</h2>
        {project ? (
          <div className="epic-origin-picker__project-card">
            <div className="epic-origin-picker__project-title">
              <Folder size={17} aria-hidden="true" />
              <strong>{project.name}</strong>
            </div>
            <span title={project.path}>{project.path}</span>
            <button type="button" disabled={busy} onClick={() => void chooseProject()}>
              {busy ? 'Opening…' : 'Change project'}
            </button>
          </div>
        ) : (
          <button
            className="epic-origin-picker__choose-project"
            type="button"
            disabled={busy}
            onClick={() => void chooseProject()}
          >
            <Folder size={17} aria-hidden="true" />
            {busy ? 'Opening project…' : 'Choose project folder'}
          </button>
        )}
        {project && (
          <p
            className={
              project.gitDetected
                ? 'epic-origin-picker__git-status is-detected'
                : 'epic-origin-picker__git-status'
            }
          >
            {project.gitDetected ? (
              <CheckCircle2 size={16} aria-hidden="true" />
            ) : (
              <Info size={16} aria-hidden="true" />
            )}
            {project.gitDetected ? 'Git repository detected' : 'No Git repository detected'}
          </p>
        )}
      </section>

      <section className="epic-origin-picker__branch" aria-labelledby="epic-origin-branch-label">
        <h2 id="epic-origin-branch-label">Origin branch</h2>
        <output title={selectedBranch?.name}>{selectedBranch?.name ?? 'No origin selected'}</output>
        <button
          ref={branchTrigger}
          className="epic-origin-picker__select-branch"
          type="button"
          disabled={!canChooseBranch || busy}
          aria-haspopup="dialog"
          onClick={() => setPickerOpen(true)}
        >
          <GitBranch size={16} aria-hidden="true" />
          Select branch
        </button>
        {project?.gitDetected && project.branches.length === 0 && (
          <p className="epic-origin-picker__branch-help">
            Create a local branch before selecting an origin.
          </p>
        )}
      </section>

      {error && (
        <p className="epic-origin-picker__error" role="alert">
          {error}
        </p>
      )}

      {pickerOpen && project && (
        <OriginBranchDialog
          project={project}
          selectedBranchName={originBranch}
          onCancel={closePicker}
          onConfirm={confirmOrigin}
        />
      )}
    </div>
  );
}

function OriginBranchDialog({
  project,
  selectedBranchName,
  onCancel,
  onConfirm,
}: {
  readonly project: EpicOriginProject;
  readonly selectedBranchName: string | null;
  readonly onCancel: () => void;
  readonly onConfirm: (branchName: string) => void;
}) {
  const [pendingBranchName, setPendingBranchName] = useState(selectedBranchName);
  const closeButton = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLElement>(null);
  const pendingBranch = project.branches.find((branch) => branch.name === pendingBranchName);
  const baselineBranch = project.branches.find((branch) => branch.isBaseline);
  const tree = useMemo(() => buildBranchTree(project.branches), [project.branches]);

  useEffect(() => {
    const background = document.querySelector<HTMLElement>('.epic-plan-builder');
    const previousAriaHidden = background?.getAttribute('aria-hidden') ?? null;
    const previousInert = background?.inert ?? false;
    const previousOverflow = document.body.style.overflow;
    if (background) {
      background.inert = true;
      background.setAttribute('aria-hidden', 'true');
    }
    document.body.style.overflow = 'hidden';
    closeButton.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onCancel();
        return;
      }
      if (event.key !== 'Tab' || !dialog.current) return;
      const focusable = Array.from(
        dialog.current.querySelectorAll<HTMLElement>(
          'button:not(:disabled), [href], input:not(:disabled), [tabindex]:not([tabindex="-1"])',
        ),
      );
      const first = focusable[0];
      const last = focusable.at(-1);
      if (!first || !last) return;
      if (
        event.shiftKey &&
        (document.activeElement === first || !dialog.current.contains(document.activeElement))
      ) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('keydown', onKeyDown);
      document.body.style.overflow = previousOverflow;
      if (background) {
        background.inert = previousInert;
        if (previousAriaHidden === null) background.removeAttribute('aria-hidden');
        else background.setAttribute('aria-hidden', previousAriaHidden);
      }
    };
  }, [onCancel]);

  return createPortal(
    <div className="origin-branch-dialog__backdrop">
      <section
        ref={dialog}
        className="origin-branch-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="origin-branch-dialog-title"
      >
        <header className="origin-branch-dialog__header">
          <div>
            <h2 id="origin-branch-dialog-title">Select origin branch</h2>
            <strong>{project.name}</strong>
            <p>Choose where this epic starts. New worktrees will branch from this origin.</p>
          </div>
          <button ref={closeButton} type="button" onClick={onCancel}>
            <X size={16} aria-hidden="true" />
            Close
          </button>
        </header>

        <div className="origin-branch-dialog__columns">
          <section className="origin-branch-dialog__map" aria-label="Branch map">
            <h3>Branch map</h3>
            <div className="origin-branch-dialog__tree">
              {tree.related.map((branch) => (
                <OriginBranchNode
                  key={branch.name}
                  branch={branch}
                  children={tree.children}
                  selectedBranchName={pendingBranchName}
                  depth={0}
                  onSelect={setPendingBranchName}
                />
              ))}
              {tree.unrelated.length > 0 && (
                <div className="origin-branch-dialog__unrelated">
                  <p>Other Git histories</p>
                  {tree.unrelated.map((branch) => (
                    <OriginBranchButton
                      key={branch.name}
                      branch={branch}
                      selected={branch.name === pendingBranchName}
                      onSelect={setPendingBranchName}
                    />
                  ))}
                </div>
              )}
            </div>
          </section>

          <aside className="origin-branch-dialog__details" aria-live="polite">
            <h3>Selected origin</h3>
            {pendingBranch ? (
              <>
                <strong className="origin-branch-dialog__selected-name">
                  {pendingBranch.name}
                </strong>
                <dl>
                  <div>
                    <dt>Since {baselineBranch?.name ?? 'baseline'}</dt>
                    <dd>
                      {pendingBranch.relationship === 'unrelated'
                        ? 'No common ancestor'
                        : `${pendingBranch.ahead} ${pendingBranch.ahead === 1 ? 'commit' : 'commits'} ahead${pendingBranch.behind ? `, ${pendingBranch.behind} behind` : ''}`}
                    </dd>
                  </div>
                  <div>
                    <dt>Fork revision</dt>
                    <dd>{pendingBranch.forkRevision}</dd>
                  </div>
                  <div>
                    <dt>Current revision</dt>
                    <dd>{pendingBranch.revision}</dd>
                  </div>
                </dl>
                <p className="origin-branch-dialog__notice">
                  <Info size={16} aria-hidden="true" />
                  Development happens in descendant worktrees, not on this branch.
                </p>
              </>
            ) : (
              <p className="origin-branch-dialog__empty">Select a branch to use as the origin.</p>
            )}
          </aside>
        </div>

        <footer>
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
          <button
            className="origin-branch-dialog__confirm"
            type="button"
            disabled={!pendingBranchName}
            onClick={() => pendingBranchName && onConfirm(pendingBranchName)}
          >
            Use as origin
          </button>
        </footer>
      </section>
    </div>,
    document.body,
  );
}

function OriginBranchNode({
  branch,
  children,
  selectedBranchName,
  depth,
  onSelect,
}: {
  readonly branch: EpicOriginBranch;
  readonly children: ReadonlyMap<string, readonly EpicOriginBranch[]>;
  readonly selectedBranchName: string | null;
  readonly depth: number;
  readonly onSelect: (branchName: string) => void;
}) {
  const descendants = children.get(branch.name) ?? [];
  return (
    <div className="origin-branch-dialog__branch" data-depth={depth}>
      <OriginBranchButton
        branch={branch}
        selected={branch.name === selectedBranchName}
        onSelect={onSelect}
      />
      {descendants.length > 0 && (
        <div className="origin-branch-dialog__children">
          {descendants.map((child) => (
            <OriginBranchNode
              key={child.name}
              branch={child}
              children={children}
              selectedBranchName={selectedBranchName}
              depth={depth + 1}
              onSelect={onSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function OriginBranchButton({
  branch,
  selected,
  onSelect,
}: {
  readonly branch: EpicOriginBranch;
  readonly selected: boolean;
  readonly onSelect: (branchName: string) => void;
}) {
  return (
    <button
      type="button"
      className={selected ? 'is-selected' : undefined}
      aria-pressed={selected}
      onClick={() => onSelect(branch.name)}
    >
      <span className="origin-branch-dialog__radio" aria-hidden="true" />
      <GitBranch size={16} aria-hidden="true" />
      <span>
        <strong>{branch.name}</strong>
        <small>
          {branch.revision}
          {branch.isCurrent ? ' · current checkout' : ''}
        </small>
      </span>
      {branch.isBaseline && <em>baseline</em>}
    </button>
  );
}

function buildBranchTree(branches: readonly EpicOriginBranch[]) {
  const related = branches.filter((branch) => branch.relationship === 'related');
  const unrelated = branches.filter((branch) => branch.relationship === 'unrelated');
  const names = new Set(related.map((branch) => branch.name));
  const children = new Map<string, EpicOriginBranch[]>();
  const roots: EpicOriginBranch[] = [];
  for (const branch of related) {
    if (!branch.parentName || !names.has(branch.parentName)) {
      roots.push(branch);
      continue;
    }
    children.set(branch.parentName, [...(children.get(branch.parentName) ?? []), branch]);
  }
  const sort = (items: EpicOriginBranch[]) =>
    items.sort(
      (left, right) =>
        Number(right.isBaseline) - Number(left.isBaseline) || left.name.localeCompare(right.name),
    );
  sort(roots);
  children.forEach(sort);
  return { related: roots, unrelated: sort([...unrelated]), children };
}

function readStoredSelection(): StoredOriginSelection | null {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<StoredOriginSelection>;
    if (typeof parsed.projectPath !== 'string' || !parsed.projectPath.trim()) return null;
    return {
      projectPath: parsed.projectPath,
      ...(typeof parsed.branchName === 'string' && parsed.branchName.trim()
        ? { branchName: parsed.branchName }
        : {}),
    };
  } catch {
    return null;
  }
}
