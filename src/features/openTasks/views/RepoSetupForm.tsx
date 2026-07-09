import { type FormEvent } from 'react';
import { GitBranch, RefreshCw } from 'lucide-react';
import type {
  DiscoveredRepoOption,
  RepoSetupFormViewModel,
} from '../viewModels/repoSetupViewModel';

export interface RepoSetupFormProps {
  form: RepoSetupFormViewModel;
  discoveredRepos: DiscoveredRepoOption[];
  addBusy: boolean;
  scanBusy: boolean;
  available: boolean;
  scanAvailable: boolean;
  onChange(form: RepoSetupFormViewModel): void;
  onSubmit(event: FormEvent<HTMLFormElement>): void;
  onScan(event: FormEvent<HTMLFormElement>): void;
  onAddDiscovered(path: string): void;
}

export function RepoSetupForm({
  form,
  discoveredRepos,
  addBusy,
  scanBusy,
  available,
  scanAvailable,
  onChange,
  onSubmit,
  onScan,
  onAddDiscovered,
}: RepoSetupFormProps) {
  const canAdd = available && !addBusy && form.repoRootPath.trim().length > 0;
  const canScan = scanAvailable && !scanBusy && form.scanRootPath.trim().length > 0;

  return (
    <section className="repo-setup" aria-label="Repo setup">
      <form className="setup-panel repo-add-panel" onSubmit={onSubmit} aria-label="Add repo">
        <input
          value={form.repoRootPath}
          onChange={(event) => onChange({ ...form, repoRootPath: event.target.value })}
          disabled={!available || addBusy}
          placeholder="Repo root path"
          aria-label="Repo root path"
        />
        <input
          value={form.projectName}
          onChange={(event) => onChange({ ...form, projectName: event.target.value })}
          disabled={!available || addBusy}
          placeholder="Project name"
          aria-label="Project name"
        />
        <button className="primary-action" type="submit" disabled={!canAdd}>
          <GitBranch size={17} aria-hidden="true" />
          Add repo
        </button>
      </form>

      <form className="setup-panel repo-scan-panel" onSubmit={onScan} aria-label="Scan for repos">
        <input
          value={form.scanRootPath}
          onChange={(event) => onChange({ ...form, scanRootPath: event.target.value })}
          disabled={!scanAvailable || scanBusy}
          placeholder="Search root folder"
          aria-label="Search root folder"
        />
        <button className="primary-action" type="submit" disabled={!canScan}>
          <RefreshCw size={17} aria-hidden="true" />
          Scan
        </button>
      </form>

      {discoveredRepos.length > 0 && (
        <div className="repo-results" aria-label="Discovered repos">
          {discoveredRepos.map((repo) => (
            <button
              className="repo-result"
              key={repo.path}
              type="button"
              onClick={() => onAddDiscovered(repo.path)}
              disabled={addBusy}
            >
              <GitBranch size={16} aria-hidden="true" />
              <span>{repo.name}</span>
              <small title={repo.path}>{repo.compactPath}</small>
            </button>
          ))}
        </div>
      )}
    </section>
  );
}
