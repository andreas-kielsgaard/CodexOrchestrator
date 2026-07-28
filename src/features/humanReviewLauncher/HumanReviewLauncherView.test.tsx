import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
} from '../../application/humanReviewLauncher';
import { HumanReviewLauncherView } from './HumanReviewLauncherView';

describe('HumanReviewLauncherView', () => {
  it('prepares and opens a named instance through semantic controls without infrastructure details', async () => {
    const client = new FakeClient();
    render(<HumanReviewLauncherView client={client} />);

    await screen.findByRole('option', { name: /codex\/feature/ });
    fireEvent.change(screen.getByLabelText('Window name'), { target: { value: 'Checkout review' } });
    fireEvent.click(screen.getByRole('button', { name: 'Prepare' }));

    const card = await screen.findByRole('heading', { name: 'Checkout review' });
    const review = card.closest('article');
    expect(review).not.toBeNull();
    expect(review).not.toHaveTextContent('C:\\repos');
    expect(review).not.toHaveTextContent('18200');
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Build' }));
    await waitFor(() => expect(within(review as HTMLElement).getByText('passed')).toBeInTheDocument());
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Open' }));
    await waitFor(() => expect(within(review as HTMLElement).getByText('running')).toBeInTheDocument());
    expect(within(review as HTMLElement).getByRole('button', { name: 'Focus window' })).toBeEnabled();

    client.interrupt();
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Check status' }));
    await waitFor(() => expect(within(review as HTMLElement).getByText('closed')).toBeInTheDocument());
    const recover = within(review as HTMLElement).getByRole('button', { name: 'Recover' });
    expect(recover).toBeEnabled();
    fireEvent.click(recover);
    await waitFor(() => expect(within(review as HTMLElement).getByText('recovered')).toBeInTheDocument());
  });
});

class FakeClient implements HumanReviewLauncherClient {
  private instance: HumanReviewInstance | undefined;
  listSources = async () => [{ sourceRef: 'opaque', label: 'codex/feature - review', revision: 'abcdef012345' }];
  listInstances = async () => this.instance ? [this.instance] : [];
  prepare = async (_sourceRef: string, name: string) => this.set(name, 'prepared', 'not-built');
  build = async () => this.set(this.instance!.name, 'prepared', 'passed');
  start = async () => this.set(this.instance!.name, 'running', 'passed');
  status = async () => this.instance!;
  focus = async () => this.instance!;
  stop = async () => this.set(this.instance!.name, 'stopped', 'passed');
  recover = async () => this.set(this.instance!.name, 'recovered', 'passed');

  interrupt() {
    this.instance = { ...this.instance!, health: 'closed', canFocus: false };
  }

  private set(name: string, phase: string, build: HumanReviewInstance['build']) {
    this.instance = {
      instanceRef: 'opaque-instance', name, sourceLabel: 'codex/feature - review', phase,
      health: phase === 'running' ? 'healthy' : 'unknown', stale: false, build,
      canFocus: phase === 'running',
    };
    return this.instance;
  }
}
