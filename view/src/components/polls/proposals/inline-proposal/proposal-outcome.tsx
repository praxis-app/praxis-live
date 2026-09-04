import { useVotingDeadline } from '@/hooks/use-voting-deadline';
import { getProposalRuleStatus } from '@/lib/poll.utils';
import { type PollRes } from '@/types/poll.types';
import { useTranslation } from 'react-i18next';
import { LuLoaderCircle } from 'react-icons/lu';

interface Props {
  poll: PollRes;
}

export const ProposalOutcome = ({ poll }: Props) => {
  const { t } = useTranslation();
  const deadlineHasPassed = useVotingDeadline(poll.config.closingAt);
  const status = getProposalRuleStatus(
    poll.votes ?? [],
    poll.config,
    poll.memberCount,
  );

  if (poll.stage === 'ratified') {
    return null;
  }

  if (poll.stage === 'closed') {
    if (poll.closedReason || status.ignoredBlocks === 0) {
      return null;
    }

    return (
      <p className="text-muted-foreground text-sm">
        {t('proposals.descriptions.ignoredBlocks', {
          count: status.ignoredBlocks,
        })}
      </p>
    );
  }

  if (deadlineHasPassed) {
    return (
      <div className="text-muted-foreground flex items-center gap-2 text-sm">
        <LuLoaderCircle
          className="size-4 shrink-0 animate-spin"
          aria-hidden="true"
        />
        <p>{t('proposals.outcomes.finalizing')}</p>
      </div>
    );
  }

  if (status.ignoredBlocks > 0) {
    return (
      <p className="text-muted-foreground text-sm">
        {t('proposals.descriptions.ignoredBlocks', {
          count: status.ignoredBlocks,
        })}
      </p>
    );
  }

  return null;
};
