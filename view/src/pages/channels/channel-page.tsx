import { api } from '@/client/api-client';
import { TextChannelView } from '@/components/channels/text-channel-view';
import { ForumChannelView } from '@/components/forum/forum-channel-view';
import {
  LocalStorageKeys,
  NavigationPaths,
} from '@/constants/shared.constants';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import {
  type RightPanel,
  type StandaloneRightPanel,
} from '@/types/right-panel.types';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { Navigate, useNavigate, useParams } from 'react-router-dom';

const LARGE_DESKTOP_MEDIA_QUERY = '(min-width: 1200px)';

const getDefaultStandaloneRightPanel = (): StandaloneRightPanel | null => {
  const storedPreference = localStorage.getItem(
    LocalStorageKeys.DecisionsPanelOpen,
  );
  const isOpen =
    storedPreference === 'true' ||
    (storedPreference !== 'false' &&
      window.matchMedia(LARGE_DESKTOP_MEDIA_QUERY).matches);

  return isOpen ? { type: 'activeDecisions' } : null;
};

export const ChannelPage = () => {
  const [standaloneRightPanel, setStandaloneRightPanel] =
    useState<StandaloneRightPanel | null>(getDefaultStandaloneRightPanel);

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
    setStandaloneRightPanel(getDefaultStandaloneRightPanel());
  }, [serverId]);

  useEffect(() => {
    if (postId) {
      setStandaloneRightPanel(null);
    } else {
      setStandaloneRightPanel(getDefaultStandaloneRightPanel());
    }
  }, [postId]);

  const closeDecisionsPanel = () => {
    localStorage.setItem(LocalStorageKeys.DecisionsPanelOpen, 'false');
    setStandaloneRightPanel(null);
  };

  const openRightPanel = (panel: StandaloneRightPanel) => {
    if (postId && channelPath) {
      void navigate(channelPath);
    }
    localStorage.setItem(LocalStorageKeys.DecisionsPanelOpen, 'true');
    setStandaloneRightPanel(panel);
  };

  const toggleDecisionsPanel = () => {
    if (rightPanel?.type === 'activeDecisions') {
      closeDecisionsPanel();
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
        const result = await api.getChannel(serverId, channelId);
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
        onCloseDecisionsPanel={closeDecisionsPanel}
        onToggleDecisionsPanel={toggleDecisionsPanel}
      />
    );
  }

  return (
    <TextChannelView
      channel={channel}
      isDecisionsPanelOpen={rightPanel?.type === 'activeDecisions'}
      onCloseDecisionsPanel={closeDecisionsPanel}
      onToggleDecisionsPanel={toggleDecisionsPanel}
    />
  );
};
