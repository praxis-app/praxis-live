import { api } from '@/client/api-client';
import { TextChannelView } from '@/components/channels/text-channel-view';
import { ForumChannelView } from '@/components/forum/forum-channel-view';
import { NavigationPaths } from '@/constants/shared.constants';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { useAuthStore } from '@/store/auth.store';
import {
  type RightPanel,
  type StandaloneRightPanel,
} from '@/types/right-panel.types';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { Navigate, useNavigate, useParams } from 'react-router-dom';

export const ChannelPage = () => {
  const [standaloneRightPanel, setStandaloneRightPanel] =
    useState<StandaloneRightPanel | null>(null);

  const { inviteToken } = useAuthStore();
  const { isRegistered } = useAuthData();
  const { serverId } = useServerData();

  const { channelId, postId, serverSlug } = useParams();
  const navigate = useNavigate();

  const rightPanel: RightPanel = postId
    ? { type: 'forumPost', postId }
    : standaloneRightPanel;

  const channelPath =
    serverSlug && channelId ? `/s/${serverSlug}/c/${channelId}` : undefined;

  useEffect(() => {
    setStandaloneRightPanel(null);
  }, [serverId]);

  useEffect(() => {
    if (postId) {
      setStandaloneRightPanel(null);
    }
  }, [postId]);

  const closeRightPanel = () => {
    if (rightPanel?.type === 'forumPost' && channelPath) {
      void navigate(channelPath);
    }
    setStandaloneRightPanel(null);
  };

  const openRightPanel = (panel: StandaloneRightPanel) => {
    if (postId && channelPath) {
      void navigate(channelPath);
    }
    setStandaloneRightPanel(panel);
  };

  const toggleDecisionsPanel = () => {
    if (rightPanel?.type === 'activeDecisions') {
      closeRightPanel();
      return;
    }
    openRightPanel({ type: 'activeDecisions' });
  };

  const { data: channelData, error: channelError } = useQuery({
    queryKey: ['servers', serverId, 'channels', channelId],
    queryFn: async () => {
      try {
        if (!serverId || !channelId) {
          throw new Error('Missing server or channel id');
        }
        const result = await api.getChannel(serverId, channelId, inviteToken);
        return result;
      } catch (error) {
        await navigate(
          isRegistered ? NavigationPaths.Root : NavigationPaths.Explore,
        );
        console.error(error);
        return null;
      }
    },
    enabled: !!channelId && !!serverId,
  });

  const channel = channelData?.channel;
  const channelServerSlug = channel?.server?.slug;

  if (channelServerSlug && serverSlug !== channelServerSlug) {
    return <Navigate to={`/s/${channelServerSlug}/c/${channel.id}`} replace />;
  }

  if (channelError) {
    throw new Error(channelError.message);
  }

  if (channel?.channelType === 'forum') {
    return (
      <ForumChannelView
        channel={channel}
        rightPanel={rightPanel}
        onCloseRightPanel={closeRightPanel}
        onToggleDecisionsPanel={toggleDecisionsPanel}
      />
    );
  }

  return (
    <TextChannelView
      channel={channel}
      isDecisionsPanelOpen={rightPanel?.type === 'activeDecisions'}
      onCloseDecisionsPanel={closeRightPanel}
      onToggleDecisionsPanel={toggleDecisionsPanel}
    />
  );
};
