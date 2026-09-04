import { api } from '@/client/api-client';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { type UnreadChannelsRes } from '@/types/channel.types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

export const getUnreadChannelsQueryKey = (serverId?: string) => [
  'servers',
  serverId,
  'channels',
  'unread',
];

export const useUnreadChannels = () => {
  const { isMeSuccess } = useAuthData();
  const { serverId } = useServerData();
  const queryClient = useQueryClient();

  const queryKey = getUnreadChannelsQueryKey(serverId);

  const { data } = useQuery({
    queryKey,
    queryFn: () => {
      if (!serverId) {
        throw new Error('Current server not found');
      }
      return api.getUnreadChannels(serverId);
    },
    enabled: !!serverId && isMeSuccess,
  });

  const { mutate: markChannelRead } = useMutation({
    mutationFn: async (channelId: string) => {
      if (!serverId) {
        throw new Error('Current server not found');
      }
      await api.markChannelRead(serverId, channelId);
    },
    onMutate: async (channelId) => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData<UnreadChannelsRes>(queryKey);
      queryClient.setQueryData<UnreadChannelsRes>(queryKey, (current) =>
        current
          ? {
              channelIds: current.channelIds.filter((id) => id !== channelId),
            }
          : current,
      );
      return { previous };
    },
    onError: (_error, _channelId, context) => {
      queryClient.setQueryData(queryKey, context?.previous);
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey });
    },
  });

  return {
    unreadChannelIds: data?.channelIds || [],
    markChannelRead,
  };
};
