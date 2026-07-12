import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { AgentSessionScreen } from './AgentSessionScreen';
import {
  createRecordedAgentSessionClient,
  createRecordedAgentSessionStore,
  recordedAgentSessionScenarios,
} from '../../dev/agentSessions';

describe('AgentSessionScreen with recorded scenarios', () => {
  it('shows live processing, tool activity, and intermediate output before completion', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.liveProcessing,
    });
    render(<AgentSessionScreen client={client} />);
    await waitFor(() => expect(screen.getByText('Inspect the repository')).toBeInTheDocument());

    await act(async () => {
      client.advance();
      client.advance();
      client.advance();
      await Promise.resolve();
    });
    expect(screen.getByText('Processing started')).toBeVisible();
    expect(screen.getByText('Reading files')).toBeVisible();
    expect(screen.getByText('Considering the result')).toBeVisible();
    expect(screen.queryByText('The final answer')).not.toBeInTheDocument();
  });

  it('collapses completed processing while keeping the final response prominent and expandable', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.liveProcessing,
    });
    render(<AgentSessionScreen client={client} />);
    await waitFor(() => expect(screen.getByText('Inspect the repository')).toBeInTheDocument());

    await act(async () => {
      client.advanceAll();
      await Promise.resolve();
    });
    await waitFor(() => expect(screen.getByText('The final answer')).toBeVisible());
    const processing = document.querySelector<HTMLDetailsElement>('.processing-disclosure');
    expect(processing).not.toBeNull();
    expect(processing?.open).toBe(false);
    expect(screen.getByText('Reading files')).not.toBeVisible();

    fireEvent.click(processing!.querySelector('summary')!);
    expect(screen.getByText('Reading files')).toBeVisible();
  });

  it('does not let a nonselected-session update replace the selected transcript', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.twoSessions,
    });
    render(<AgentSessionScreen client={client} />);
    await waitFor(() =>
      expect(screen.getByRole('region', { name: 'session-a' })).toBeInTheDocument(),
    );

    await act(async () => {
      client.advance();
      await Promise.resolve();
    });
    expect(screen.getByRole('region', { name: 'session-a' })).toBeInTheDocument();
    expect(screen.queryByText('Background B update')).not.toBeInTheDocument();

    await act(async () => {
      client.advance();
      await Promise.resolve();
    });
    expect(await screen.findByText('Selected A update')).toBeVisible();
  });

  it('shows and dismisses recorded operation errors', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.errors,
    });
    render(<AgentSessionScreen client={client} />);

    expect(await screen.findByRole('alert')).toHaveTextContent('Recorded subscription failed');
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss error' }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('reconstructs multi-invocation durable history after a restart-style remount', async () => {
    const store = createRecordedAgentSessionStore();
    const first = createRecordedAgentSessionClient({
      store,
      scenario: recordedAgentSessionScenarios.outcomes,
    });
    const firstRender = render(<AgentSessionScreen client={first} />);
    await waitFor(() => expect(screen.getByText('Cancel me')).toBeInTheDocument());

    await act(async () => {
      first.advanceAll();
      await Promise.resolve();
    });
    expect(await screen.findByText('This invocation was canceled.')).toBeVisible();
    expect(
      await screen.findByText('This invocation was interrupted before completion.'),
    ).toBeVisible();
    firstRender.unmount();

    const restarted = createRecordedAgentSessionClient({
      store,
      scenario: recordedAgentSessionScenarios.outcomes,
    });
    expect(restarted.stepIndex).toBe(restarted.stepCount);
    expect(restarted.emittedUpdates).toEqual([]);
    render(<AgentSessionScreen client={restarted} />);
    expect(await screen.findByText('Cancel me')).toBeVisible();
    expect(await screen.findByText('Interrupt me')).toBeVisible();
    expect(screen.getByText('This invocation was canceled.')).toBeVisible();
    expect(screen.getByText('This invocation was interrupted before completion.')).toBeVisible();
  });

  it('keeps technical diagnostics and raw output disclosures available', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.diagnostics,
    });
    render(<AgentSessionScreen client={client} />);
    await waitFor(() => expect(screen.getByText('Show diagnostics')).toBeInTheDocument());

    await act(async () => {
      client.advanceAll();
      await Promise.resolve();
    });
    expect(await screen.findByText('FUTURE_EVENT')).toBeVisible();
    expect(screen.getByText('stderr text')).toBeVisible();
    expect(screen.getByText('Technical details (3)')).toBeVisible();
    expect(screen.getAllByText('Raw event').length).toBeGreaterThan(0);
  });

  it('renders GFM through AgentMarkdown without executing raw HTML', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.markdownGfm,
    });
    const { container } = render(<AgentSessionScreen client={client} />);
    await waitFor(() => expect(screen.getByText('Render Markdown')).toBeInTheDocument());

    await act(async () => {
      client.advanceAll();
      await Promise.resolve();
    });
    expect(await screen.findByRole('heading', { name: 'Result' })).toBeVisible();
    expect(screen.getByText('GFM')).toBeVisible();
    expect(container.querySelector('.agent-markdown span')).not.toBeInTheDocument();
    expect(container.querySelector('.agent-markdown script')).not.toBeInTheDocument();
  });

  it('presents canceled and interrupted outcomes without inventing final replies', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.outcomes,
    });
    render(<AgentSessionScreen client={client} />);
    await waitFor(() => expect(screen.getByText('Cancel me')).toBeInTheDocument());

    await act(async () => {
      client.advanceAll();
      await Promise.resolve();
    });
    expect(await screen.findByText('Canceled.')).toBeVisible();
    expect(await screen.findByText('Interrupted.')).toBeVisible();
    expect(screen.queryByText('Agent')).not.toBeInTheDocument();
  });

  it('preserves semantic behavior with long submitted content', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.longContent,
    });
    render(<AgentSessionScreen client={client} />);
    const longText = 'A'.repeat(4096);

    expect(await screen.findByText((content) => content === longText)).toBeVisible();
    expect(screen.getByRole('region', { name: 'long-session' })).toBeInTheDocument();
  });
});
