import { useCallback, useMemo, useState, type FormEvent } from 'react';
import type {
  DiscoveredTaskRepo,
  RegisterTaskRepoInput,
  RepoOnboardingCapability,
} from '../../../capabilities/repoOnboarding';
import type {
  TaskDashboardSnapshot,
} from '../../../capabilities/openTaskDashboard';
import { errorMessage } from '../../../app/viewModels/formatting';
import {
  normalizeRepoSetupInput,
  type RepoSetupDraft,
} from '../viewModels/repoSetupViewModel';

export interface RepoOnboardingDraft extends RepoSetupDraft {
  scanRootPath: string;
}

export type RepoOnboardingBusyAction = 'register-repo' | 'discover-repos' | null;

export interface RepoOnboardingControllerState {
  draft: RepoOnboardingDraft;
  discoveredRepos: DiscoveredTaskRepo[];
  busyAction: RepoOnboardingBusyAction;
  error: string | null;
  registerAvailable: boolean;
  discoverAvailable: boolean;
  canRegister: boolean;
  canDiscover: boolean;
}

export interface RepoOnboardingControllerActions {
  setDraft(draft: RepoOnboardingDraft): void;
  patchDraft(patch: Partial<RepoOnboardingDraft>): void;
  submitRegister(event?: FormEvent<HTMLFormElement>): void;
  submitDiscover(event?: FormEvent<HTMLFormElement>): void;
  registerRepo(input?: RegisterTaskRepoInput): Promise<boolean>;
  registerDiscoveredRepo(repo: DiscoveredTaskRepo): Promise<boolean>;
  discoverRepos(): Promise<boolean>;
  clearDiscoveredRepos(): void;
  clearError(): void;
}

export interface RepoOnboardingController {
  state: RepoOnboardingControllerState;
  actions: RepoOnboardingControllerActions;
}

export interface UseRepoOnboardingControllerInput {
  client: RepoOnboardingCapability;
  onSnapshot(snapshot: TaskDashboardSnapshot): void;
  initialDraft?: Partial<RepoOnboardingDraft>;
  discoverMaxDepth?: number;
}

export const initialRepoOnboardingDraft: RepoOnboardingDraft = {
  projectName: '',
  repoRootPath: '',
  scanRootPath: '',
};

export function useRepoOnboardingController({
  client,
  onSnapshot,
  initialDraft,
  discoverMaxDepth = 5,
}: UseRepoOnboardingControllerInput): RepoOnboardingController {
  const [draft, setDraft] = useState<RepoOnboardingDraft>(() => ({
    ...initialRepoOnboardingDraft,
    ...initialDraft,
  }));
  const [discoveredRepos, setDiscoveredRepos] = useState<DiscoveredTaskRepo[]>([]);
  const [busyAction, setBusyAction] = useState<RepoOnboardingBusyAction>(null);
  const [error, setError] = useState<string | null>(null);

  const registerAvailable = Boolean(client.registerRepo);
  const discoverAvailable = Boolean(client.discoverRepos);
  const canRegister =
    registerAvailable && busyAction === null && draft.repoRootPath.trim().length > 0;
  const canDiscover =
    discoverAvailable && busyAction === null && draft.scanRootPath.trim().length > 0;

  const patchDraft = useCallback((patch: Partial<RepoOnboardingDraft>) => {
    setDraft((current) => ({ ...current, ...patch }));
  }, []);

  const registerRepo = useCallback(
    async (input?: RegisterTaskRepoInput): Promise<boolean> => {
      if (!client.registerRepo || busyAction !== null) {
        return false;
      }

      const normalizedInput = input ?? normalizeRepoSetupInput(draft);

      if (!normalizedInput) {
        return false;
      }

      setBusyAction('register-repo');
      setError(null);

      try {
        const nextSnapshot = await client.registerRepo(normalizedInput);

        onSnapshot(nextSnapshot);
        setDraft((current) => ({
          ...current,
          repoRootPath: normalizedInput.repoRootPath,
          projectName: current.projectName.trim(),
        }));
        setDiscoveredRepos((current) =>
          current.filter((repo) => repo.path !== normalizedInput.repoRootPath),
        );
        return true;
      } catch (caught) {
        setError(errorMessage(caught));
        return false;
      } finally {
        setBusyAction(null);
      }
    },
    [busyAction, client, draft, onSnapshot],
  );

  const discoverRepos = useCallback(async (): Promise<boolean> => {
    const rootPath = draft.scanRootPath.trim();

    if (!rootPath || !client.discoverRepos || busyAction !== null) {
      return false;
    }

    setBusyAction('discover-repos');
    setError(null);

    try {
      const repos = await client.discoverRepos({ rootPath, maxDepth: discoverMaxDepth });

      setDiscoveredRepos(repos);
      return true;
    } catch (caught) {
      setError(errorMessage(caught));
      return false;
    } finally {
      setBusyAction(null);
    }
  }, [busyAction, client, discoverMaxDepth, draft.scanRootPath]);

  const submitRegister = useCallback(
    (event?: FormEvent<HTMLFormElement>) => {
      event?.preventDefault();
      void registerRepo();
    },
    [registerRepo],
  );

  const submitDiscover = useCallback(
    (event?: FormEvent<HTMLFormElement>) => {
      event?.preventDefault();
      void discoverRepos();
    },
    [discoverRepos],
  );

  const actions = useMemo<RepoOnboardingControllerActions>(
    () => ({
      setDraft,
      patchDraft,
      submitRegister,
      submitDiscover,
      registerRepo,
      registerDiscoveredRepo: (repo) => registerRepo({ repoRootPath: repo.path }),
      discoverRepos,
      clearDiscoveredRepos: () => setDiscoveredRepos([]),
      clearError: () => setError(null),
    }),
    [discoverRepos, patchDraft, registerRepo, submitDiscover, submitRegister],
  );

  return {
    state: {
      draft,
      discoveredRepos,
      busyAction,
      error,
      registerAvailable,
      discoverAvailable,
      canRegister,
      canDiscover,
    },
    actions,
  };
}
