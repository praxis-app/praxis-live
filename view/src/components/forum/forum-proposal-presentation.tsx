import { ProposalContent } from '@/components/polls/proposals/inline-proposal/proposal-content';
import { timeAgo } from '@/lib/time.utils';
import { type ChannelRes } from '@/types/channel.types';
import { type ForumPostRes } from '@/types/forum.types';
import { type CurrentUser } from '@/types/user.types';
import { type QueryKey } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

interface Props {
  channel: ChannelRes;
  post: ForumPostRes;
  feedQueryKey: QueryKey;
  me?: CurrentUser;
}

export const ForumProposalPresentation = ({
  channel,
  post,
  feedQueryKey,
  me,
}: Props) => {
  const { t } = useTranslation();
  const proposal = post.proposal;
  if (!proposal) return null;

  const proposalAuthor = proposal.user.displayName || proposal.user.name;

  return (
    <section
      aria-label={t('forums.labels.proposal')}
      className="@container mt-5 space-y-3 border-t pt-5"
    >
      <p className="text-muted-foreground text-xs">
        {t('forums.labels.proposalCreated', {
          author: proposalAuthor,
          time: timeAgo(proposal.createdAt),
        })}
      </p>
      <ProposalContent
        poll={proposal}
        channel={channel}
        feedQueryKey={feedQueryKey}
        me={me}
        variant="forum"
      />
    </section>
  );
};
