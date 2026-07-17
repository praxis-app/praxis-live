import { ForumProposalPresentation } from '@/components/forum/forum-proposal-presentation';
import { useAuthData } from '@/hooks/use-auth-data';
import { type ChannelRes } from '@/types/channel.types';
import { type ForumPostRes } from '@/types/forum.types';
import { type QueryKey } from '@tanstack/react-query';

interface Props {
  channel: ChannelRes;
  post: ForumPostRes;
  feedQueryKey: QueryKey;
}

export const ForumPostProposal = ({ channel, post, feedQueryKey }: Props) => {
  const { me } = useAuthData();
  if (!post.proposal) return null;

  return (
    <ForumProposalPresentation
      channel={channel}
      post={post}
      feedQueryKey={feedQueryKey}
      me={me}
    />
  );
};
