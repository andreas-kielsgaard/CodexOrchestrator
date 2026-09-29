import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { WorkflowPromptAgentConfigurationEditor } from './WorkflowPromptAgentConfigurationEditor';

it('edits capitalized session choices and structured include or exclude filters', async () => {
  const user = userEvent.setup();
  const changes = vi.fn();
  function Controlled() {
    const [value, setValue] = useState<Readonly<Record<string, unknown>>>({});
    return (
      <WorkflowPromptAgentConfigurationEditor
        value={value}
        onChange={(next) => {
          changes(next);
          setValue(next);
        }}
      />
    );
  }
  render(<Controlled />);

  expect(screen.getByLabelText('Session mode')).toHaveDisplayValue('Continue a session');
  expect(screen.getByLabelText('Sessions to prompt')).toHaveDisplayValue('One session');
  expect(screen.getByLabelText('Session selection logic')).toHaveDisplayValue('Newest session');
  expect(screen.getByLabelText('If no session matches')).toHaveDisplayValue('Create a new session');

  await user.selectOptions(screen.getByLabelText('Add session filter'), 'created_by_event');
  await user.click(screen.getByRole('button', { name: 'Exclude' }));
  await user.type(screen.getByLabelText('Event ID'), 'event-7');

  expect(changes).toHaveBeenLastCalledWith({
    filters: [{ kind: 'created_by_event', operation: 'exclude', eventId: 'event-7' }],
  });
  expect(
    within(screen.getByLabelText('Add session filter')).queryByRole('option', {
      name: 'Created by event',
    }),
  ).toBeNull();
});

it('hides continuation configuration when starting a new session', async () => {
  const user = userEvent.setup();
  const change = vi.fn();
  render(<WorkflowPromptAgentConfigurationEditor value={{}} onChange={change} />);
  await user.selectOptions(screen.getByLabelText('Session mode'), 'new');
  expect(change).toHaveBeenCalledWith({ mode: 'new' });
});
