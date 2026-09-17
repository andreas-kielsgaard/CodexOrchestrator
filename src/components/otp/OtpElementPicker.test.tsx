import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { OtpElementPicker } from './OtpElementPicker';

const groups = [
  {
    id: 'a',
    label: 'Package A',
    items: [
      { id: 'a/tool/one', label: 'Continue' },
      { id: 'a/tool/two', label: 'Second output' },
    ],
  },
  { id: 'b', label: 'Package B', items: [{ id: 'b/tool', label: 'Continue' }] },
];
it('previews distinct package/output identities without applying until confirmation', async () => {
  const user = userEvent.setup();
  const apply = vi.fn();
  const close = vi.fn();
  render(
    <OtpElementPicker
      title="Set trigger"
      groups={groups}
      selected={['a/tool/one']}
      renderDetails={(id) => <p>Details: {id}</p>}
      onConfirm={apply}
      onClose={close}
    />,
  );
  expect(screen.getByRole('button', { name: /Package A/ })).toHaveAttribute(
    'aria-expanded',
    'true',
  );
  await user.click(screen.getByRole('button', { name: /Package B/ }));
  await user.click(screen.getAllByRole('button', { name: 'Continue' })[1]);
  expect(
    within(screen.getByRole('region', { name: 'Element details' })).getByText('Details: b/tool'),
  ).toBeVisible();
  expect(apply).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Set trigger' }));
  expect(apply).toHaveBeenCalledWith(['b/tool']);
});
it('keeps checked choices separate from preview and cancellation', async () => {
  const user = userEvent.setup();
  const apply = vi.fn();
  const close = vi.fn();
  render(
    <OtpElementPicker
      title="Set tools"
      multiple
      groups={groups}
      selected={['a/tool/one', 'missing']}
      renderDetails={(id) => id}
      onConfirm={apply}
      onClose={close}
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Second output' }));
  expect(screen.getByRole('checkbox', { name: 'Include Second output' })).not.toBeChecked();
  await user.click(screen.getByRole('checkbox', { name: 'Include Second output' }));
  expect(screen.getByText('Unavailable selections')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(close).toHaveBeenCalled();
  expect(apply).not.toHaveBeenCalled();
});
it('explains an empty catalogue and supports inspecting one option', async () => {
  const user = userEvent.setup();
  const props = {
    title: 'Set action',
    selected: [],
    renderDetails: (id: string) => id,
    onConfirm: vi.fn(),
    onClose: vi.fn(),
  };
  const view = render(<OtpElementPicker {...props} groups={[]} />);
  expect(screen.getByText('No eligible elements are available.')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Set action' })).toBeDisabled();
  view.rerender(<OtpElementPicker {...props} groups={[groups[1]]} />);
  await user.click(screen.getByRole('button', { name: /Package B/ }));
  await user.click(screen.getByRole('button', { name: 'Continue' }));
  expect(screen.getByRole('button', { name: 'Set action' })).toBeEnabled();
});
