import { ProposalContent } from '@/components/polls/proposals/inline-proposal/proposal-content';
import { type ChannelRes } from '@/types/channel.types';
import { type PollRes } from '@/types/poll.types';
import { type CurrentUser } from '@/types/user.types';
import { type QueryKey } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

interface Props {
  channel: ChannelRes;
  proposal: PollRes;
  feedQueryKey: QueryKey;
  me?: CurrentUser;
}

export const ForumProposalPresentation = ({
  channel,
  proposal,
  feedQueryKey,
  me,
}: Props) => {
  const { t } = useTranslation();

  return (
    <section
      aria-label={t('forums.labels.proposal')}
      className="@container mt-5 space-y-3 border-t pt-5"
    >
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
