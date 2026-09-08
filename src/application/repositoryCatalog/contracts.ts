export type RepositoryId = string & { readonly __kind?: 'repository' };

export interface RepositoryDiscoveryStatus {
  readonly state: 'ready' | 'unavailable';
  readonly message: string;
}

export interface GitHubConnectionStatus {
  readonly state: 'connected' | 'cli_unavailable' | 'not_authenticated' | 'unavailable';
  readonly login?: string;
  readonly message: string;
}

export interface GitHubRepositoryRegistrationFacts {
  readonly repositoryId: string;
  readonly nameWithOwner: string;
  readonly visibility: 'public' | 'private';
  readonly webUrl: string;
}

export interface LocalRepositoryRegistrationCandidate {
  readonly repositoryId: RepositoryId;
  readonly name: string;
  readonly locationLabel: string;
  readonly registered: boolean;
  readonly disclosures: readonly ('manual_directory' | 'codex_task')[];
}

export interface RepositoryRegistrationCandidate {
  readonly catalogId: string;
  readonly name: string;
  readonly github?: GitHubRepositoryRegistrationFacts;
  readonly localInstances: readonly LocalRepositoryRegistrationCandidate[];
}

export interface RepositoryCatalogOverview {
  readonly codex: RepositoryDiscoveryStatus;
  readonly github: GitHubConnectionStatus;
  readonly repositories: readonly RepositoryRegistrationCandidate[];
}

export interface RegisteredRepository {
  readonly repositoryId: RepositoryId;
  readonly name: string;
  readonly locationLabel: string;
}
