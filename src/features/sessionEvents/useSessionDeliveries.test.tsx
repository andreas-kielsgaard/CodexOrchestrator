import { act, renderHook, waitFor } from '@testing-library/react';
import type {
  EventDeliveryRecordDto,
  SessionEventQueryClient,
} from '../../application/sessionEvents';
import { useSessionDeliveries } from './useSessionDeliveries';

it('refreshes on post-store notification, exposes failures and ignores old Session replies', async () => {
  let recorded!: () => void;
  let oldReply!: (value: readonly EventDeliveryRecordDto[]) => void;
  const unsubscribe = vi.fn();
  const client: SessionEventQueryClient = {
    subscribeRecorded: async (listener) => {
      recorded = listener;
      return unsubscribe;
    },
    listDeliveriesForSession: vi.fn(async () => []),
    loadEventGroup: async () => null,
    loadRecordedEvent: async () => null,
    listDeliveriesForGroup: async () => [],
  };
  const { result, rerender, unmount } = renderHook(({ id }) => useSessionDeliveries(client, id), {
    initialProps: { id: 'a' },
  });
  await waitFor(() => expect(client.listDeliveriesForSession).toHaveBeenCalledTimes(1));
  vi.mocked(client.listDeliveriesForSession).mockRejectedValueOnce(new Error('Read failed'));
  act(() => recorded());
  await waitFor(() => expect(result.current.error).toBe('Read failed'));
  act(() => result.current.reload());
  await waitFor(() => expect(result.current.error).toBeNull());
  vi.mocked(client.listDeliveriesForSession).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        oldReply = resolve;
      }),
  );
  act(() => result.current.reload());
  rerender({ id: 'b' });
  await waitFor(() =>
    expect(client.listDeliveriesForSession).toHaveBeenLastCalledWith(
      expect.objectContaining({ id: 'b' }),
    ),
  );
  await act(async () =>
    oldReply([
      {
        deliveryId: { namespace: 'test', kind: 'delivery', id: 'stale' },
      } as EventDeliveryRecordDto,
    ]),
  );
  expect(result.current.deliveries).toEqual([]);
  unmount();
  expect(unsubscribe).toHaveBeenCalledOnce();
});
