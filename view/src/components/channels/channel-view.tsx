import { ChatChannelView } from '@/components/channels/chat-channel-view';
import { ForumChannelView } from '@/components/forum/forum-channel-view';
import { type ChannelRes } from '@/types/channel.types';

interface Props {
  channel?: ChannelRes;
}

export const ChannelView = ({ channel }: Props) => {
  if (channel?.channelType === 'forum') {
    return <ForumChannelView channel={channel} />;
  }

  return <ChatChannelView channel={channel} />;
};
