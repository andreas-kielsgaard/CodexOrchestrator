import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SessionInteractions } from './SessionInteractions';
import type { SessionInteractionDto } from '../../application/agentSessions';

const request: SessionInteractionDto = {
  id: 'approval',
  invocationId: 'turn',
  sequence: 3,
  kind: 'request',
  state: 'pending',
  result: null,
  content: {
    title: 'Allow this command?',
    choices: [
      { label: 'Allow once', response: { decision: 'accept' } },
      { label: 'Decline', response: { decision: 'decline' } },
    ],
  },
};

it('explains saved-rule scope and submits the exact offered response', async () => {
  const response = {
    decision: { acceptWithExecpolicyAmendment: { execpolicy_amendment: ['git', 'status'] } },
  };
  const respond = vi.fn().mockResolvedValue(undefined);
  render(
    <SessionInteractions
      interactions={[
        {
          ...request,
          content: {
            ...request.content,
            choices: [
              {
                label: 'Allow and save rule',
                description: 'Allow future matching commands.',
                scope: 'git status',
                response,
              },
            ],
          },
        },
      ]}
      onRespond={respond}
    />,
  );
  expect(screen.getByRole('button', { name: 'Allow and save rule' })).toHaveAccessibleDescription(
    'Allow future matching commands.',
  );
  await userEvent.click(screen.getByText('Rule scope'));
  expect(screen.getByText('git status')).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: 'Allow and save rule' }));
  expect(respond).toHaveBeenCalledWith('turn', 'approval', response);
});

it('sends an explicit offered approval response and disables settled requests', async () => {
  const respond = vi.fn().mockResolvedValue(undefined);
  const { rerender } = render(<SessionInteractions interactions={[request]} onRespond={respond} />);
  await userEvent.click(screen.getByRole('button', { name: 'Allow once' }));
  expect(respond).toHaveBeenCalledWith('turn', 'approval', { decision: 'accept' });
  rerender(
    <SessionInteractions interactions={[{ ...request, state: 'answered' }]} onRespond={respond} />,
  );
  expect(screen.getByRole('button', { name: 'Allow once' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Decline' })).toBeDisabled();
});

it('shows rejected or uncertain steering as durable text without response controls', () => {
  render(
    <SessionInteractions
      interactions={[
        {
          id: 'input',
          invocationId: 'turn',
          sequence: 4,
          kind: 'steering',
          state: 'uncertain',
          content: { text: 'Keep this correction' },
          result: 'No acknowledgement',
        },
      ]}
    />,
  );
  expect(screen.getByText('Keep this correction')).toBeInTheDocument();
  expect(screen.getByText('No acknowledgement')).toBeInTheDocument();
  expect(screen.queryByRole('button')).toBeNull();
});
