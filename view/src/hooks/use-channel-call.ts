import { api } from '@/client/api-client';
import { type JoinCallRes } from '@/types/call.types';
import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export const useChannelCall = (serverId?: string, channelId?: string) => {
  const [callConfig, setCallConfig] = useState<JoinCallRes | null>(null);
  const { t } = useTranslation();

  const joinMutation = useMutation({
    mutationFn: async () => {
      if (!serverId || !channelId) {
        throw new Error('Server ID and channel ID are required');
      }

      return api.joinChannelCall(serverId, channelId);
    },
    onSuccess: (config) => {
      setCallConfig(config);
    },
    onError: () => {
      toast(t('calls.errors.joinFailed'));
    },
  });

  return {
    callConfig,
    isJoining: joinMutation.isPending,
    joinCall: joinMutation.mutate,
    leaveCall: () => setCallConfig(null),
  };
};
