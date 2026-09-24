import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { StandaloneAgentSessionScreen } from './AgentSessionScreen';
import { repairSessionClients } from './profileTestFixtures';

it('uses slash choices for the first message and refreshes discovery in the created Session context', async () => {
  const user = userEvent.setup();
  const fixture = repairSessionClients(false);
  const load = vi.spyOn(fixture.profiles, 'loadQuickFeatures');
  const start = vi.spyOn(fixture.profiles, 'startDirectUserSession');
  render(
    <StandaloneAgentSessionScreen client={fixture.sessions} profileClient={fixture.profiles} />,
  );
  let input = await screen.findByRole('textbox', { name: 'Message' });
  await user.type(input, '/model');
  await screen.findByRole('option', { name: /Model/ });
  await user.keyboard('{Enter}');
  await user.type(input, 'model-b{Enter}');
  expect(start).not.toHaveBeenCalled();
  expect(load).toHaveBeenCalledWith({ sessionId: null, workingDirectory: null });
  await user.type(input, 'First{Enter}');
  await waitFor(() =>
    expect(start).toHaveBeenCalledWith(
      expect.objectContaining({ submittedText: 'First', model: 'model-b' }),
    ),
  );
  input = screen.getByRole('textbox', { name: 'Message' });
  await user.type(input, '/reasoning');
  await screen.findByRole('option', { name: /Reasoning/ });
  expect(load).toHaveBeenLastCalledWith({
    sessionId: 'session-1',
    workingDirectory: fixture.details.session.workingDirectory,
  });
  await user.keyboard('{Enter}');
  await user.type(input, 'medium{Enter}');
  await user.click(screen.getByRole('button', { name: /Message and Session configuration/ }));
  expect(screen.getByLabelText('Model')).toHaveValue('model-a');
  expect(screen.getByLabelText('Reasoning')).toHaveValue('medium');
});

it('uses the profiled route for both the first and next message, with message-local choices', async () => {
  const user = userEvent.setup();
  const fixture = repairSessionClients(false);
  const start = vi.spyOn(fixture.profiles, 'startDirectUserSession');
  const send = vi.spyOn(fixture.profiles, 'sendDirectUserMessage');
  const generic = vi.spyOn(fixture.sessions, 'sendMessage');
  render(
    <StandaloneAgentSessionScreen client={fixture.sessions} profileClient={fixture.profiles} />,
  );
  fireEvent.change(await screen.findByRole('textbox', { name: 'Message' }), {
    target: { value: 'First' },
  });
  await user.click(screen.getByRole('button', { name: 'Send' }));
  await waitFor(() =>
    expect(start).toHaveBeenCalledWith(
      expect.objectContaining({ submittedText: 'First', model: null }),
    ),
  );
  await user.click(
    await screen.findByRole('button', { name: /Message and Session configuration/ }),
  );
  fireEvent.change(screen.getByLabelText('Model'), { target: { value: 'model-b' } });
  fireEvent.change(screen.getByLabelText('Reasoning'), { target: { value: 'medium' } });
  fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'Next' } });
  await user.click(screen.getByRole('button', { name: 'Send' }));
  await waitFor(() =>
    expect(send).toHaveBeenCalledWith({
      sessionId: 'session-1',
      submittedText: 'Next',
      model: 'model-b',
      reasoningMode: 'medium',
    }),
  );
  expect(screen.getByLabelText('Model')).toHaveValue('model-a');
  expect(generic).not.toHaveBeenCalled();
});

it('keeps old unprofiled history readable and disables sending rather than replacing its profile', async () => {
  const fixture = repairSessionClients();
  fixture.profiles.loadPinnedProfile = vi.fn(async () => {
    throw new Error('No pinned profile');
  });
  render(
    <StandaloneAgentSessionScreen client={fixture.sessions} profileClient={fixture.profiles} />,
  );
  expect(await screen.findByText('Do the work')).toBeVisible();
  fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), {
    target: { value: 'Cannot send' },
  });
  await waitFor(() => expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled());
  expect(await screen.findByText(/Its history is still readable/)).toBeInTheDocument();
});

it('edits the Session identity without changing the node or pinned profile', async () => {
  const user = userEvent.setup();
  const fixture = repairSessionClients();
  const update = vi.spyOn(fixture.sessions, 'updateIdentity');
  render(
    <StandaloneAgentSessionScreen client={fixture.sessions} profileClient={fixture.profiles} />,
  );
  await user.click(await screen.findByRole('button', { name: 'Edit identity' }));
  fireEvent.change(screen.getByLabelText('Identity display name'), { target: { value: 'Alex' } });
  await user.click(screen.getByRole('button', { name: 'Apply identity' }));
  await waitFor(() =>
    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: 'session-1',
        assignedIdentity: expect.objectContaining({ displayName: 'Alex' }),
      }),
    ),
  );
  expect(fixture.profile.creationResolution.digest).toBe('fixture-digest');
});
