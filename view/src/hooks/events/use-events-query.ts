import { api } from '@/client/api-client';
import { type EventsQuery } from '@/types/event.types';
import { useQuery } from '@tanstack/react-query';

export const getEventsQueryKey = (serverId: string | undefined) => [
  'servers',
  serverId,
  'events',
];

export const useEventsQuery = (
  serverId: string | undefined,
  query: EventsQuery,
) =>
  useQuery({
    queryKey: [...getEventsQueryKey(serverId), query],
    queryFn: () => {
      if (!serverId) throw new Error('Server ID is required');
      return api.getEvents(serverId, query);
    },
    enabled: !!serverId,
  });
