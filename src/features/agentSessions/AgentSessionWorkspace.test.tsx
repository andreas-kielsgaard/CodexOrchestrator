import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import { AgentSessionHeaderActionsProvider, AgentSessionWorkspace } from './AgentSessionWorkspace';
import { projectAgentSessionTranscript } from './transcriptProjector';
import type { AgentSessionWorkspaceController } from './useAgentSessionController';
import { sessionDetails } from './testFixtures';

describe('AgentSessionWorkspace', () => {
  it('mounts from a supplied session controller without collection or navigation dependencies', () => {
    const details = sessionDetails('completed');
    const controller: AgentSessionWorkspaceController = {
      selectedSessionId: 'session-1',
      details,
      transcript: projectAgentSessionTranscript(details),
      draft: '',
      workingDirectory: 'C:/workspace',
      loading: false,
      sending: false,
      canceling: false,
      error: null,
      expandedProcessing: new Set(),
      setDraft: () => undefined,
      setWorkingDirectory: () => undefined,
      send: async () => undefined,
      cancel: async () => undefined,
      reload: async () => undefined,
      toggleProcessing: () => undefined,
      clearError: () => undefined,
    };

    render(<AgentSessionWorkspace controller={controller} />);

    expect(screen.getByRole('heading', { name: 'Durable session' })).toBeVisible();
    expect(screen.getByText('Do the work')).toBeVisible();
    expect(screen.getByLabelText('Message')).toBeInTheDocument();
    expect(screen.queryByLabelText('Working directory')).toBeNull();
    expect(screen.queryByLabelText('Agent Sessions')).not.toBeInTheDocument();
  });

  it('only shows working-directory input when a presentation explicitly opts in', () => {
    const details = sessionDetails('completed');
    const controller: AgentSessionWorkspaceController = {
      selectedSessionId: null,
      details,
      transcript: projectAgentSessionTranscript(details),
      draft: '',
      workingDirectory: 'C:/workspace',
      loading: false,
      sending: false,
      canceling: false,
      error: null,
      expandedProcessing: new Set(),
      setDraft: () => undefined,
      setWorkingDirectory: () => undefined,
      send: async () => undefined,
      cancel: async () => undefined,
      reload: async () => undefined,
      toggleProcessing: () => undefined,
      clearError: () => undefined,
    };

    render(
      <AgentSessionWorkspace
        controller={controller}
        presentation={{ composer: { showWorkingDirectory: true } }}
      />,
    );

    expect(screen.getByLabelText('Working directory')).toBeVisible();
    expect(screen.getByText('C:/workspace')).toBeVisible();
  });

  it('uses its shared flexible conversation layout when the session header is hidden', () => {
    const details = sessionDetails('completed');
    const controller: AgentSessionWorkspaceController = {
      selectedSessionId: 'session-1',
      details,
      transcript: projectAgentSessionTranscript(details),
      draft: '',
      workingDirectory: 'C:/workspace',
      loading: false,
      sending: false,
      canceling: false,
      error: null,
      expandedProcessing: new Set(),
      setDraft: () => undefined,
      setWorkingDirectory: () => undefined,
      send: async () => undefined,
      cancel: async () => undefined,
      reload: async () => undefined,
      toggleProcessing: () => undefined,
      clearError: () => undefined,
    };

    render(
      <AgentSessionWorkspace
        controller={controller}
        presentation={{ showHeader: false, ariaLabel: 'Managed conversation' }}
      />,
    );

    const workspace = screen.getByLabelText('Managed conversation');
    const conversation = workspace.querySelector('.agent-session-conversation');
    expect(workspace).toHaveClass('agent-session-workspace--header-hidden');
    expect(conversation).not.toBeNull();
    expect(conversation?.lastElementChild).toHaveClass('agent-session-composer');
  });

  it('places application-owned Session settings between the header and conversation', () => {
    const details = sessionDetails('completed');
    render(
      <AgentSessionHeaderActionsProvider
        actions={<button type="button">Manage</button>}
        settings={<div aria-label="Session settings">Model and effort</div>}
      >
        <AgentSessionWorkspace controller={workspaceController(details)} />
      </AgentSessionHeaderActionsProvider>,
    );

    const workspace = screen.getByLabelText('Durable session');
    expect(workspace).toHaveClass('agent-session-workspace--with-settings');
    expect(screen.getByLabelText('Session settings').parentElement).toHaveClass(
      'agent-session-settings',
    );
    expect(screen.getByRole('button', { name: 'Manage' })).toBeVisible();
  });

  it('copies a complete sanitized plain-text session from normal and embedded views', async () => {
    const details = sessionDetails('completed');
    details.invocations[0].invocation.submittedText = 'Use password=hunter2';
    details.invocations[0].invocation.diagnostics.push({
      source: 'runtime',
      severity: 'warning',
      code: 'fixture',
      message: 'authorization: secret-value',
      details: { raw: 'excluded' },
      recordedAt: details.session.updatedAt,
    });
    const controller = workspaceController(details);
    const writeText = vi.fn().mockResolvedValue(undefined);
    const { rerender } = render(
      <AgentSessionWorkspace controller={controller} clipboard={{ writeText }} />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Copy entire session' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledOnce());
    expect(writeText.mock.calls[0][0]).toContain('Title: Durable session');
    expect(writeText.mock.calls[0][0]).toContain('Session ID: session-1');
    expect(writeText.mock.calls[0][0]).toContain('Status: completed');
    expect(writeText.mock.calls[0][0]).toContain('User\nUse password=[REDACTED]');
    expect(writeText.mock.calls[0][0]).toContain('authorization=[REDACTED]');
    expect(writeText.mock.calls[0][0]).not.toContain('hunter2');
    expect(writeText.mock.calls[0][0]).not.toContain('secret-value');
    expect(screen.getByRole('button', { name: 'Copied' })).toBeVisible();

    rerender(
      <AgentSessionWorkspace
        controller={controller}
        presentation={{ showHeader: false }}
        clipboard={{ writeText }}
      />,
    );
    expect(screen.getByRole('button', { name: 'Copied' })).toBeVisible();
  });

  it('reports clipboard failure without claiming success', async () => {
    const details = sessionDetails('completed');
    render(
      <AgentSessionWorkspace
        controller={workspaceController(details)}
        clipboard={{ writeText: vi.fn().mockRejectedValue(new Error('denied')) }}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Copy entire session' }));
    expect(await screen.findByText('Session could not be copied.')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Copied' })).toBeNull();
  });
});

function workspaceController(
  details: ReturnType<typeof sessionDetails>,
): AgentSessionWorkspaceController {
  return {
    selectedSessionId: details.session.id,
    details,
    transcript: projectAgentSessionTranscript(details),
    draft: '',
    workingDirectory: 'C:/workspace',
    loading: false,
    sending: false,
    canceling: false,
    error: null,
    expandedProcessing: new Set(),
    setDraft: () => undefined,
    setWorkingDirectory: () => undefined,
    send: async () => undefined,
    cancel: async () => undefined,
    reload: async () => undefined,
    toggleProcessing: () => undefined,
    clearError: () => undefined,
  };
}
