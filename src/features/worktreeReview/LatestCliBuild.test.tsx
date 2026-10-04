import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { LatestCliBuild } from './LatestCliBuild';

it('offers the registered build and opens it without rebuilding', async () => {
  const client = {
    latest: vi.fn().mockResolvedValue({ executable: 'app.exe', profile: 'debug' }),
    open: vi.fn().mockResolvedValue(undefined),
  };
  render(<LatestCliBuild path="C:/worktree" client={client} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Open latest build' }));
  await waitFor(() => expect(client.open).toHaveBeenCalledWith('C:/worktree'));
  expect(client.latest).toHaveBeenCalledWith('C:/worktree');
});

it('does not offer a build for a worktree without output', async () => {
  const client = { latest: vi.fn().mockResolvedValue(null), open: vi.fn() };
  render(<LatestCliBuild path="C:/empty" client={client} />);
  await waitFor(() => expect(client.latest).toHaveBeenCalled());
  expect(screen.queryByRole('button')).toBeNull();
});

it('reports launch failures', async () => {
  const client = {
    latest: vi.fn().mockResolvedValue({ executable: 'app.exe', profile: 'release' }),
    open: vi.fn().mockRejectedValue('Build disappeared'),
  };
  render(<LatestCliBuild path="C:/worktree" client={client} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Open latest build' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Build disappeared');
});
