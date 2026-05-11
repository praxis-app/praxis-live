import { api } from '@/client/api-client';
import { type JoinCallRes } from '@/types/call.types';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export const useChannelCall = (serverId?: string, channelId?: string) => {
  const [callConfig, setCallConfig] = useState<JoinCallRes | null>(null);
  const isLeavingRef = useRef(false);

  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const joinMutation = useMutation({
    mutationFn: async (callId?: string) => {
      if (!serverId || !channelId) {
        throw new Error('Server ID and channel ID are required');
      }

      if (callId) {
        return api.joinChannelCallById(serverId, channelId, callId);
      }

      return api.joinChannelCall(serverId, channelId);
    },
    onSuccess: (config) => {
      isLeavingRef.current = false;
      setCallConfig(config);
      void queryClient.invalidateQueries({
        queryKey: ['servers', serverId, 'channels', channelId, 'feed'],
      });
    },
    onError: () => {
      toast(t('calls.errors.joinFailed'));
    },
  });

  return {
    callConfig,
    isJoining: joinMutation.isPending,
    joinCall: (callId?: string) => joinMutation.mutate(callId),
    leaveCall: async () => {
      if (isLeavingRef.current) {
        return;
      }

      isLeavingRef.current = true;
      const callId = callConfig?.call.id;
      setCallConfig(null);

      if (!serverId || !channelId || !callId) {
        return;
      }

      try {
        await api.leaveChannelCall(serverId, channelId, callId);
        void queryClient.invalidateQueries({
          queryKey: ['servers', serverId, 'channels', channelId, 'feed'],
        });
      } catch {
        // A failed best-effort leave should not trap the user inside the room.
      } finally {
        isLeavingRef.current = false;
      }
    },
  };
};
