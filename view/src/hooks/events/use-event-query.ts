import { api } from '@/client/api-client';
import { useQuery } from '@tanstack/react-query';

export const getEventQueryKey = (
  serverId: string | undefined,
  eventId: string | undefined,
) => ['servers', serverId, 'events', eventId];

export const useEventQuery = (
  serverId: string | undefined,
  eventId: string | undefined,
) =>
  useQuery({
    queryKey: getEventQueryKey(serverId, eventId),
    queryFn: () => {
      if (!serverId || !eventId)
        throw new Error('Server and event IDs are required');
      return api.getEvent(serverId, eventId);
    },
    enabled: !!serverId && !!eventId,
  });
