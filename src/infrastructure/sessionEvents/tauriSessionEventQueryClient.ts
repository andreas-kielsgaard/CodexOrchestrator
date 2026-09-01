import { invoke } from '@tauri-apps/api/core';
import type {
  EventDeliveryRecordDto,
  EventGroupRecordDto,
  SessionEventQueryClient,
  SessionEventResultDto,
} from '../../application/sessionEvents';

export type SessionEventInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriSessionEventQueryClient(
  invokeCommand: SessionEventInvoke = invoke,
): SessionEventQueryClient {
  return {
    loadEventGroup: (eventGroupId) =>
      invokeCommand<EventGroupRecordDto | null>('load_session_event_group', {
        query: { eventGroupId },
      }),
    loadRecordedEvent: (eventGroupId) =>
      invokeCommand<SessionEventResultDto | null>('load_recorded_session_event', {
        query: { eventGroupId },
      }),
    listDeliveriesForGroup: (eventGroupId) =>
      invokeCommand<EventDeliveryRecordDto[]>('list_session_event_deliveries_for_group', {
        query: { eventGroupId },
      }),
    listDeliveriesForSession: (session) =>
      invokeCommand<EventDeliveryRecordDto[]>('list_session_event_deliveries_for_session', {
        query: { session },
      }),
  };
}

export const tauriSessionEventQueryClient = createTauriSessionEventQueryClient();
