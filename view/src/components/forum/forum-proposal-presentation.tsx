import { ProposalContent } from '@/components/polls/proposals/inline-proposal/proposal-content';
import { type ChannelRes } from '@/types/channel.types';
import { type ForumPostRes } from '@/types/forum.types';
import { type PollRes } from '@/types/poll.types';
import { type CurrentUser } from '@/types/user.types';
import { type QueryKey, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

interface Props {
  channel: ChannelRes;
  proposal: PollRes;
  postQueryKey: QueryKey;
  me?: CurrentUser;
  votingDisabled?: boolean;
  votingDisabledReason?: string;
}

export const ForumProposalPresentation = ({
  channel,
  proposal,
  postQueryKey,
  me,
  votingDisabled = false,
  votingDisabledReason,
}: Props) => {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const updateCachedProposal = (
    update: (cachedProposal: PollRes) => PollRes,
  ) => {
    queryClient.setQueryData<{ post: ForumPostRes }>(postQueryKey, (old) => {
      if (!old?.post.proposal) return old;
      return {
        ...old,
        post: {
          ...old.post,
          proposal: update(old.post.proposal),
        },
      };
    });
    void queryClient.invalidateQueries({ queryKey: postQueryKey.slice(0, 5) });
  };

  return (
    <section
      aria-label={t('forums.labels.proposal')}
      className="@container relative mt-5 space-y-3 border-t pt-5"
    >
      <ProposalContent
        poll={proposal}
        channel={channel}
        me={me}
        variant="forum"
        votingDisabled={votingDisabled}
        votingDisabledReason={votingDisabledReason}
        updateCachedProposal={updateCachedProposal}
      />
    </section>
  );
};
