import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type {
  AgentSessionClient,
  AgentSessionDetailsDto,
} from '../../../application/agentSessions';
import { projectAgentSessionTranscript } from '../../agentSessions';
import { runtimeEvent, sessionDetails } from '../../agentSessions/testFixtures';
import {
  SharedAgentSessionPanel,
  type SharedAgentSessionPresentation,
} from './SharedAgentSessionPanel';

const recordedSession = namedSession('session-1', 'Completed session', 'Final response');
const presentation: SharedAgentSessionPresentation = {
  sessionId: recordedSession.session.id,
  title: recordedSession.session.title,
  transcript: projectAgentSessionTranscript(recordedSession),
};

describe('SharedAgentSessionPanel controller composition', () => {
  it('uses the injected client/controller path and requires explicit writability for a composer', async () => {
    const client = sessionClient();
    const { rerender } = render(
      <SharedAgentSessionPanel
        ariaLabel="Embedded session"
        conversationAriaLabel="Embedded conversation"
        session={presentation}
        composition={{ client }}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Open Agent Session' }));
    await waitFor(() => expect(screen.getByText('Final response')).toBeVisible());
    expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();

    rerender(
      <SharedAgentSessionPanel
        ariaLabel="Embedded session"
        conversationAriaLabel="Embedded conversation"
        session={presentation}
        composition={{ client, writableSessionIds: new Set([presentation.sessionId]) }}
      />,
    );
    expect(await screen.findByRole('textbox', { name: 'Message' })).toBeVisible();
  });

  it('does not fall back to recorded transcript or composer when the injected client cannot load', async () => {
    const client = {
      ...sessionClient(),
      loadSession: async () => {
        throw new Error('Session unavailable');
      },
    };
    const missing: SharedAgentSessionPresentation = {
      sessionId: 'missing-session',
      title: 'Missing session',
      transcript: projectAgentSessionTranscript(sessionDetails('completed')),
    };
    render(
      <SharedAgentSessionPanel
        ariaLabel="Missing embedded session"
        conversationAriaLabel="Missing conversation"
        session={missing}
        composition={{ client, writableSessionIds: new Set(['missing-session']) }}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Open Agent Session' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Session unavailable');
    expect(screen.queryByText('Do the work')).toBeNull();
    expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();
    expect(within(screen.getByLabelText('Missing conversation')).queryByRole('list')).toBeNull();
  });

  it('does not let an out-of-order embedded load replace the selected session', async () => {
    const alpha = namedSession('session-alpha', 'Alpha session', 'Alpha response');
    const beta = namedSession('session-beta', 'Beta session', 'Beta response');
    const deferred = deferredClient({ 'session-alpha': alpha, 'session-beta': beta });
    const base = (id: string, title: string): SharedAgentSessionPresentation => ({
      sessionId: id,
      title,
      transcript: projectAgentSessionTranscript(alpha),
    });
    const composition = {
      client: deferred.client,
      writableSessionIds: new Set(['session-alpha', 'session-beta']),
    };
    const { rerender } = render(
      <SharedAgentSessionPanel
        ariaLabel="Switching embedded session"
        conversationAriaLabel="Switching conversation"
        session={base('session-alpha', 'Alpha session')}
        composition={composition}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Open Agent Session' }));
    await waitFor(() => expect(deferred.requestedIds).toContain('session-alpha'));

    rerender(
      <SharedAgentSessionPanel
        ariaLabel="Switching embedded session"
        conversationAriaLabel="Switching conversation"
        session={base('session-beta', 'Beta session')}
        composition={composition}
      />,
    );
    await waitFor(() => expect(deferred.requestedIds).toContain('session-beta'));
    deferred.resolve('session-beta');
    expect(await screen.findByText('Beta response')).toBeVisible();
    deferred.resolve('session-alpha');
    await waitFor(() => expect(screen.queryByText('Alpha response')).toBeNull());
    expect(screen.getByRole('textbox', { name: 'Message' })).toBeVisible();
  });

  it('keeps inspection read-only even when the underlying Session is writable', async () => {
    const client = sessionClient();
    render(
      <SharedAgentSessionPanel
        ariaLabel="Inspection session"
        conversationAriaLabel="Inspection conversation"
        session={presentation}
        composition={{ client, writableSessionIds: new Set([presentation.sessionId]) }}
        inspection={{ invocationId: recordedSession.invocations[0].invocation.id }}
        defaultExpanded
      />,
    );

    expect(await screen.findByText('Complete recorded turn')).toBeVisible();
    expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Send' })).toBeNull();
    expect(screen.getByText(recordedSession.invocations[0].invocation.submittedText)).toBeVisible();
  });
});

function namedSession(id: string, title: string, response: string): AgentSessionDetailsDto {
  const details = structuredClone(
    sessionDetails('completed', [runtimeEvent(1, 'agent_message', response, { role: 'final' })]),
  );
  details.session.id = id;
  details.session.title = title;
  details.invocations[0].invocation.id = `${id}-invocation`;
  details.invocations[0].invocation.sessionId = id;
  details.invocations[0].events[0].invocationId = `${id}-invocation`;
  return details;
}

function deferredClient(detailsById: Record<string, AgentSessionDetailsDto>) {
  const pending = new Map<string, (details: AgentSessionDetailsDto) => void>();
  const requestedIds: string[] = [];
  const client: AgentSessionClient = {
    createSession: async () => detailsById[Object.keys(detailsById)[0]].session,
    listSessions: async () => [],
    loadSession: async ({ sessionId }) =>
      new Promise((resolve) => {
        requestedIds.push(sessionId);
        pending.set(sessionId, resolve);
      }),
    reloadSession: async ({ sessionId }) =>
      new Promise((resolve) => pending.set(sessionId, resolve)),
    subscribeUpdates: async () => () => undefined,
    sendMessage: async ({ sessionId }) => ({
      sessionId: sessionId ?? Object.keys(detailsById)[0],
      invocationId: 'embedded-invocation',
    }),
    cancelInvocation: async () =>
      detailsById[Object.keys(detailsById)[0]].invocations[0].invocation,
    disconnectUpdates: async () => undefined,
  };
  return {
    client,
    requestedIds,
    resolve(id: string) {
      pending.get(id)?.(detailsById[id]);
      pending.delete(id);
    },
  };
}

function sessionClient(): AgentSessionClient {
  return {
    createSession: vi.fn(),
    listSessions: async () => [],
    loadSession: async () => recordedSession,
    reloadSession: async () => recordedSession,
    subscribeUpdates: async () => () => undefined,
    disconnectUpdates: async () => undefined,
    sendMessage: vi.fn(),
    cancelInvocation: vi.fn(),
  };
}
