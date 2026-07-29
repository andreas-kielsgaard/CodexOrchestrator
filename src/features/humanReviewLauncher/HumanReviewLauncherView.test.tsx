import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewOperationProgress,
} from '../../application/humanReviewLauncher';
import { HumanReviewLauncherView } from './HumanReviewLauncherView';

describe('HumanReviewLauncherView', () => {
  it('prepares and opens a named instance through semantic controls without infrastructure details', async () => {
    const client = new FakeClient();
    render(<HumanReviewLauncherView client={client} />);

    await screen.findByRole('option', { name: /codex\/feature/ });
    fireEvent.change(screen.getByLabelText('Window name'), {
      target: { value: 'Checkout review' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Prepare' }));

    const card = await screen.findByRole('heading', { name: 'Checkout review' });
    const review = card.closest('article');
    expect(review).not.toBeNull();
    expect(review).not.toHaveTextContent('C:\\repos');
    expect(review).not.toHaveTextContent('18200');
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Build' }));
    await waitFor(() =>
      expect(within(review as HTMLElement).getByText('passed')).toBeInTheDocument(),
    );
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Open' }));
    await waitFor(() =>
      expect(within(review as HTMLElement).getByText('running')).toBeInTheDocument(),
    );
    expect(
      within(review as HTMLElement).getByRole('button', { name: 'Focus window' }),
    ).toBeEnabled();

    client.interrupt();
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Check status' }));
    await waitFor(() =>
      expect(within(review as HTMLElement).getByText('closed')).toBeInTheDocument(),
    );
    const recover = within(review as HTMLElement).getByRole('button', { name: 'Recover' });
    expect(recover).toBeEnabled();
    fireEvent.click(recover);
    await waitFor(() =>
      expect(within(review as HTMLElement).getByText('recovered')).toBeInTheDocument(),
    );
  });

  it('renders typed long-build progress and quiet evidence without calling it stalled', async () => {
    const client = new LongBuildClient();
    render(<HumanReviewLauncherView client={client} />);
    const card = (await screen.findByRole('heading', { name: 'Long build' })).closest('article')!;
    fireEvent.click(within(card).getByRole('button', { name: 'Build' }));
    expect(await within(card).findByText('Checking TypeScript')).toBeVisible();
    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 550));
    });
    expect(within(card).getByText('Building the application interface')).toBeVisible();
    expect(within(card).getByText(/No new evidence/)).toBeVisible();
    expect(within(card).queryByText(/stalled/i)).toBeNull();
    expect(within(card).getByText('Compiling application crate')).toBeVisible();
    client.finish();
    await waitFor(() => expect(within(card).getByText('passed')).toBeVisible());
  });

  it('reflects an application-owned background operation without button-history inference', async () => {
    const client = new FakeClient();
    client.listProgress = async () => [
      {
        operationRef: 'review-operation-background-proof',
        operation: 'start',
        state: 'pending',
        stage: 'waiting-for-window',
        stageLabel: 'Waiting for a usable worktree-build window',
        activity: 'working',
        elapsedMs: 12_000,
        evidenceAgeMs: 200,
        recentOutput: ['Owned services are ready; waiting for the review window.'],
      },
    ];
    render(<HumanReviewLauncherView client={client} />);

    const operation = await screen.findByRole('region', {
      name: 'Current application-owned review operation',
    });
    expect(within(operation).getByText('Waiting for a usable worktree-build window')).toBeVisible();
    expect(within(operation).getByText(/Owned services are ready/)).toBeVisible();
  });
});

class LongBuildClient implements HumanReviewLauncherClient {
  private progressCalls = 0;
  private resolveBuild!: (value: HumanReviewInstance) => void;
  private instance = instance('Long build', 'prepared', 'not-built');
  listSources = async () => [
    { sourceRef: 'opaque', label: 'codex/long', revision: 'abcdef012345' },
  ];
  listInstances = async () => [this.instance];
  prepare = async () => this.instance;
  build = async () =>
    new Promise<HumanReviewInstance>((resolve) => {
      this.resolveBuild = resolve;
    });
  start = async () => this.instance;
  status = async () => this.instance;
  focus = async () => this.instance;
  stop = async () => this.instance;
  recover = async () => this.instance;
  listProgress = async (): Promise<readonly HumanReviewOperationProgress[]> => [];
  progress = async (operationRef: string) => {
    const first = this.progressCalls++ === 0;
    return {
      operationRef,
      operation: 'build' as const,
      state: 'pending' as const,
      stage: first ? 'typecheck' : 'frontend-build',
      stageLabel: first ? 'Checking TypeScript' : 'Building the application interface',
      activity: first ? ('working' as const) : ('quiet' as const),
      elapsedMs: first ? 3_000 : 25_000,
      evidenceAgeMs: first ? 100 : 21_000,
      recentOutput: first ? ['Type checking application'] : ['Compiling application crate'],
    };
  };
  finish() {
    this.instance = instance('Long build', 'prepared', 'passed');
    this.resolveBuild(this.instance);
  }
}

class FakeClient implements HumanReviewLauncherClient {
  private instance: HumanReviewInstance | undefined;
  listSources = async () => [
    { sourceRef: 'opaque', label: 'codex/feature - review', revision: 'abcdef012345' },
  ];
  listInstances = async () => (this.instance ? [this.instance] : []);
  prepare = async (_operationRef: string, _sourceRef: string, name: string) =>
    this.set(name, 'prepared', 'not-built');
  build = async () => this.set(this.instance!.name, 'prepared', 'passed');
  start = async () => this.set(this.instance!.name, 'running', 'passed');
  progress = async (operationRef: string) => ({
    operationRef,
    operation: operationRef.startsWith('prepare')
      ? ('prepare' as const)
      : operationRef.startsWith('start')
        ? ('start' as const)
        : ('build' as const),
    state: 'succeeded' as const,
    stage: 'complete',
    stageLabel: 'Finished',
    activity: 'finished' as const,
    elapsedMs: 1_000,
    evidenceAgeMs: 0,
    recentOutput: ['Safe progress'],
  });
  status = async () => this.instance!;
  focus = async () => this.instance!;
  stop = async () => this.set(this.instance!.name, 'stopped', 'passed');
  recover = async () => this.set(this.instance!.name, 'recovered', 'passed');
  listProgress = async (): Promise<readonly HumanReviewOperationProgress[]> => [];

  interrupt() {
    this.instance = { ...this.instance!, health: 'closed', canFocus: false };
  }

  private set(name: string, phase: string, build: HumanReviewInstance['build']) {
    this.instance = instance(name, phase, build);
    return this.instance;
  }
}

function instance(
  name: string,
  phase: string,
  build: HumanReviewInstance['build'],
): HumanReviewInstance {
  return {
    instanceRef: 'opaque-instance',
    name,
    sourceLabel: 'codex/feature - review',
    phase,
    health: phase === 'running' ? 'healthy' : 'unknown',
    stale: false,
    build,
    canFocus: phase === 'running',
  };
}
