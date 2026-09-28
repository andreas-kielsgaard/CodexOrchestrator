import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GroupedElementPicker } from './GroupedElementPicker';

const groups = [
  {
    id: 'a',
    label: 'Package A',
    items: [
      { value: 'a/tool/one', label: 'Continue' },
      { value: 'a/tool/two', label: 'Second output' },
    ],
  },
  { id: 'b', label: 'Package B', items: [{ value: 'b/tool', label: 'Continue' }] },
  { id: 'empty', label: 'Package Empty', items: [], emptyLabel: 'No triggers' },
];

it('commits one selected item separately from details preview', async () => {
  const user = userEvent.setup();
  const apply = vi.fn();
  render(
    <GroupedElementPicker
      title="Set trigger"
      groups={groups}
      selected={['a/tool/one']}
      selectionMode="single"
      renderDetails={(id) => <p>Details: {id}</p>}
      onConfirm={apply}
      onClose={vi.fn()}
    />,
  );
  await user.click(screen.getAllByRole('button', { name: 'Continue' })[1]);
  expect(
    within(screen.getByRole('region', { name: 'Element details' })).getByText('Details: b/tool'),
  ).toBeVisible();
  expect(screen.getAllByRole('radio', { name: 'Select Continue' })[1]).toBeChecked();
  await user.click(screen.getByRole('button', { name: 'Set trigger' }));
  expect(apply).toHaveBeenCalledWith(['b/tool']);
});

it('toggles groups in multiple mode and preserves unavailable selections until confirmed', async () => {
  const user = userEvent.setup();
  const apply = vi.fn();
  render(
    <GroupedElementPicker
      title="Set tools"
      groups={groups}
      selected={['missing']}
      selectionMode="multiple"
      renderDetails={(id) => id}
      onConfirm={apply}
      onClose={vi.fn()}
    />,
  );
  expect(screen.getByText('Unavailable selections')).toBeVisible();
  await user.click(screen.getByRole('checkbox', { name: 'Toggle Package A' }));
  await user.click(screen.getByRole('button', { name: 'Apply selection' }));
  expect(apply).toHaveBeenCalledWith(['missing', 'a/tool/one', 'a/tool/two']);
});

it('renders an empty group without selection or collapse controls', () => {
  render(
    <GroupedElementPicker
      title="Set trigger"
      groups={groups}
      selected={[]}
      selectionMode="single"
      renderDetails={(id) => id}
      onConfirm={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  expect(screen.getByText('Package Empty')).toBeVisible();
  expect(screen.getByText('No triggers')).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Package Empty' })).toBeNull();
  expect(screen.queryByRole('checkbox', { name: 'Toggle Package Empty' })).toBeNull();
});
