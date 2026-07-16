import { ChannelTopNav } from '@/components/channels/channel-top-nav';
import { ForumPostDetail } from '@/components/forum/forum-post-detail';
import { ForumPostList } from '@/components/forum/forum-post-list';
import { LeftNavDesktop } from '@/components/nav/left-nav-desktop';
import { useAuthData } from '@/hooks/use-auth-data';
import { useChannelCall } from '@/hooks/use-channel-call';
import { useInstanceCapabilitiesQuery } from '@/hooks/use-instance-capabilities-query';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useServerData } from '@/hooks/use-server-data';
import { useSubscription } from '@/hooks/use-subscription';
import { channelPubSubTopic } from '@/lib/pub-sub.utils';
import { type ChannelRes } from '@/types/channel.types';
import { useQueryClient } from '@tanstack/react-query';
import { useParams } from 'react-router-dom';

interface Props {
  channel: ChannelRes;
}

export const ForumChannelView = ({ channel }: Props) => {
  const queryClient = useQueryClient();
  const isDesktop = useIsDesktop();
  const { postId } = useParams();
  const { me } = useAuthData();
  const { data: capabilities } = useInstanceCapabilitiesQuery();
  const { server, serverId } = useServerData();
  const {
    callConfig,
    callPreferences,
    cancelPreJoin,
    confirmJoinCall,
    isJoining,
    isPreJoinOpen,
    joinCall,
    leaveCall,
  } = useChannelCall(serverId, channel.id);

  useSubscription(
    channelPubSubTopic('forum-posts', serverId, channel.id, me?.id),
    {
      onMessage: () => {
        void queryClient.invalidateQueries({
          queryKey: ['servers', serverId, 'channels', channel.id, 'forum'],
        });
      },
      enabled: !!serverId && !!me,
    },
  );

  useSubscription(
    channelPubSubTopic('new-message', serverId, channel.id, me?.id),
    {
      onMessage: () => {
        void queryClient.invalidateQueries({
          queryKey: ['servers', serverId, 'channels', channel.id, 'forum'],
        });
      },
      enabled: !!serverId && !!me,
    },
  );

  return (
    <div className="fixed inset-0 flex">
      {isDesktop && <LeftNavDesktop me={me} />}

      <div className="flex min-w-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col">
          <ChannelTopNav
            channel={channel}
            callConfig={callConfig}
            callPreferences={callPreferences}
            serverName={server?.name}
            isJoiningCall={isJoining}
            isPreJoinOpen={isPreJoinOpen}
            videoCallsEnabled={capabilities?.videoCallsEnabled === true}
            onCancelPreJoin={cancelPreJoin}
            onConfirmJoinCall={confirmJoinCall}
            onJoinCall={joinCall}
            onLeaveCall={leaveCall}
          />

          {isDesktop || !postId ? (
            <ForumPostList channel={channel} selectedPostId={postId} />
          ) : (
            <ForumPostDetail channel={channel} postId={postId} />
          )}
        </div>

        {isDesktop && postId && (
          <ForumPostDetail channel={channel} postId={postId} isPane />
        )}
      </div>
    </div>
  );
};
