import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type {
  WorkflowRecipeDraftDto,
  WorkflowRecipeStateDto,
} from '../../application/workflowAuthoring';
import { DraftWorkspace } from '../../components/draftWorkspace';
import { WorkflowAuthoringScreen } from './WorkflowAuthoringScreen';
import { otpCatalogue, repairClients } from './testFixtures';

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

it('activates saved text without discarding the dirty draft and retains it across remount', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  const workspace = new DraftWorkspace<WorkflowRecipeDraftDto>();
  const activate = vi.spyOn(fixture.authoring, 'activateRecipe');
  const element = (
    <WorkflowAuthoringScreen
      client={fixture.authoring}
      readOtpCatalogue={async () => otpCatalogue}
      executionConfigurationClient={fixture.configuration}
      workspace={workspace}
    />
  );
  const mounted = render(element);
  const name = await screen.findByRole('textbox', { name: 'Workflow name' });
  fireEvent.change(name, { target: { value: 'Unsaved new name' } });
  await user.click(screen.getByRole('button', { name: 'Activate saved draft' }));
  await waitFor(() => expect(activate).toHaveBeenCalledWith('review', 1));
  expect(name).toHaveValue('Unsaved new name');
  expect(fixture.states[0].active?.name).toBe('Review workflow');
  mounted.unmount();
  render(element);
  expect(await screen.findByRole('textbox', { name: 'Workflow name' })).toHaveValue(
    'Unsaved new name',
  );
});

it('a late save keeps later typing and does not return to the previous recipe', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  const response = deferred<WorkflowRecipeStateDto>();
  fixture.authoring.saveDraft = vi.fn(() => response.promise);
  render(
    <WorkflowAuthoringScreen
      client={fixture.authoring}
      readOtpCatalogue={async () => otpCatalogue}
      executionConfigurationClient={fixture.configuration}
    />,
  );
  const name = await screen.findByRole('textbox', { name: 'Workflow name' });
  fireEvent.change(name, { target: { value: 'Submitted' } });
  await user.click(screen.getByRole('button', { name: 'Save draft' }));
  fireEvent.change(name, { target: { value: 'Later typing' } });
  await user.click(screen.getByRole('button', { name: /Other workflow Draft/ }));
  await waitFor(() =>
    expect(screen.getByRole('textbox', { name: 'Workflow name' })).toHaveValue('Other workflow'),
  );
  await act(async () =>
    response.resolve({
      ...fixture.states[0],
      draft: { ...fixture.states[0].draft, name: 'Submitted', revision: 2 },
    }),
  );
  expect(screen.getByRole('textbox', { name: 'Workflow name' })).toHaveValue('Other workflow');
  await user.click(screen.getByRole('button', { name: /Review workflow Draft/ }));
  await waitFor(() =>
    expect(screen.getByRole('textbox', { name: 'Workflow name' })).toHaveValue('Later typing'),
  );
  expect(screen.getByText(/draft revision 2/)).toBeInTheDocument();
});

it('uses the canvas for adding, connecting, copying, moving and undoing node edits', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  const save = vi.spyOn(fixture.authoring, 'saveDraft');
  render(
    <WorkflowAuthoringScreen
      client={fixture.authoring}
      readOtpCatalogue={async () => otpCatalogue}
      executionConfigurationClient={fixture.configuration}
    />,
  );
  await screen.findByRole('button', { name: 'Configure Author' });
  await user.click(screen.getByRole('button', { name: 'Add node' }));
  const canvas = screen.getByRole('region', { name: 'Workflow canvas' });
  fireEvent.click(canvas, { clientX: 200, clientY: 340 });
  await user.click(screen.getByRole('button', { name: 'Close editor' }));
  await user.click(screen.getByRole('button', { name: 'Connect' }));
  await user.click(screen.getByRole('button', { name: 'Configure Author' }));
  await user.click(screen.getByRole('button', { name: 'Configure Node 3' }));
  expect(screen.getByRole('button', { name: 'Edit Author → Node 3' })).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Close editor' }));
  await user.click(screen.getByRole('button', { name: 'Copy node' }));
  await user.click(screen.getByRole('button', { name: 'Configure Author' }));
  fireEvent.click(canvas, { clientX: 400, clientY: 340 });
  await user.click(screen.getByRole('button', { name: 'Close editor' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Configure Author copy' }), {
    key: 'ArrowRight',
    altKey: true,
  });
  await user.click(screen.getByRole('button', { name: 'Undo' }));
  await user.click(screen.getByRole('button', { name: 'Save draft' }));
  await waitFor(() => expect(save).toHaveBeenCalled());
  const saved = save.mock.calls[0][0];
  expect(saved.nodes).toHaveLength(4);
  expect(saved.connections).toHaveLength(1);
  expect(saved.nodes[3].positionX).toBe(400);
  expect(saved.nodes[3].nodeProfile).toEqual(saved.nodes[0].nodeProfile);
  expect(saved.nodes[3].nodeProfile).not.toBe(saved.nodes[0].nodeProfile);
});

it('limits node choices to the selected profile and shows a removed default', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  render(
    <WorkflowAuthoringScreen
      client={fixture.authoring}
      readOtpCatalogue={async () => otpCatalogue}
      executionConfigurationClient={fixture.configuration}
    />,
  );
  await user.click(await screen.findByRole('button', { name: 'Configure Author' }));
  expect(screen.queryByRole('checkbox', { name: 'model-b' })).not.toBeInTheDocument();
  expect(screen.getByRole('checkbox', { name: /workspace/i })).toBeDisabled();
  await user.click(screen.getByRole('checkbox', { name: 'model-a' }));
  expect(screen.getByRole('option', { name: /model-a \(unavailable\)/ })).toBeDisabled();
  expect(screen.getByText(/Default model .* is not exposed/)).toBeInTheDocument();
});
