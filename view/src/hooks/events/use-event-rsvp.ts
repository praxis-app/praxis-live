import { api } from '@/client/api-client';
import { getEventQueryKey } from '@/hooks/events/use-event-query';
import { getEventsQueryKey } from '@/hooks/events/use-events-query';
import {
  type EventDetailRes,
  type EventRes,
  type EventRsvpStatus,
} from '@/types/event.types';
import { useMutation, useQueryClient } from '@tanstack/react-query';

export const useEventRsvp = (
  serverId: string | undefined,
  eventId: string | undefined,
) => {
  const queryClient = useQueryClient();
  const updateCaches = (event: EventDetailRes) => {
    queryClient.setQueryData(getEventQueryKey(serverId, eventId), { event });
    queryClient.setQueriesData<{ events: EventRes[] }>(
      { queryKey: getEventsQueryKey(serverId) },
      (old) =>
        old
          ? {
              events: old.events.map((item) =>
                item.id === event.id ? event : item,
              ),
            }
          : old,
    );
  };
  const mutation = useMutation({
    mutationFn: (status: EventRsvpStatus | null) => {
      if (!serverId || !eventId)
        throw new Error('Server and event IDs are required');
      return status
        ? api.updateEventRsvp(serverId, eventId, { status })
        : api.clearEventRsvp(serverId, eventId);
    },
    onSuccess: ({ event }) => updateCaches(event),
  });
  return mutation;
};
