import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { App } from './App';
import { createRecordedDevelopmentApplicationComposition } from '../dev/orchestrationSection/recordedOrchestrationClient';
import { repairClients } from '../features/workflowAuthoring/testFixtures';

it('keeps recipe edits while visiting Capability Profiles and returning through product navigation', async () => {
  const fixture = repairClients();
  render(
    <App
      {...createRecordedDevelopmentApplicationComposition({ initialSurface: 'workflows' })}
      workflowAuthoringClient={fixture.authoring}
      executionConfigurationClient={fixture.configuration}
    />,
  );
  const name = await screen.findByRole('textbox', { name: 'Workflow name' });
  fireEvent.change(name, { target: { value: 'Keep my edits' } });
  fireEvent.click(screen.getByRole('button', { name: 'Capability Profiles' }));
  fireEvent.click(await screen.findByRole('button', { name: 'New profile' }));
  await screen.findByRole('textbox', { name: 'Capability profile name' });
  fireEvent.click(screen.getByRole('button', { name: 'Workflow' }));
  await waitFor(() =>
    expect(screen.getByRole('textbox', { name: 'Workflow name' })).toHaveValue('Keep my edits'),
  );
});
