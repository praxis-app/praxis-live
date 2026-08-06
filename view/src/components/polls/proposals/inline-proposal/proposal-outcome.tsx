import { useVotingDeadline } from '@/hooks/use-voting-deadline';
import { getProposalRuleStatus } from '@/lib/poll.utils';
import { type PollRes } from '@/types/poll.types';
import { useTranslation } from 'react-i18next';
import {
  LuLoaderCircle,
  LuTrendingUp,
} from 'react-icons/lu';

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
    if (poll.closedReason === 'event-start-elapsed') {
      return (
        <p className="text-destructive text-sm">
          {t('proposals.outcomes.eventStartElapsed')}
        </p>
      );
    }

    const failedRules = [
      !status.agreementMet && poll.config.decisionMakingModel !== 'consent'
        ? t('proposals.labels.approval')
        : null,
      !status.quorumMet ? t('proposals.labels.quorum') : null,
      !status.disagreementsMet &&
      poll.config.decisionMakingModel !== 'majority-vote'
        ? t('proposals.labels.disagreements')
        : null,
      !status.abstainsMet && poll.config.decisionMakingModel !== 'majority-vote'
        ? t('proposals.labels.abstentions')
        : null,
      !status.blocksMet && poll.config.decisionMakingModel !== 'majority-vote'
        ? t('proposals.labels.blocks')
        : null,
    ].filter((rule): rule is string => !!rule);

    return (
      failedRules.length > 0 && (
        <p className="text-destructive text-sm">
          {t('proposals.outcomes.failedRules', {
            rules: failedRules.join(', '),
          })}
        </p>
      )
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

  if (status.passes) {
    return (
      <div className="flex items-center gap-2 text-sm text-emerald-700 dark:text-emerald-300">
        <LuTrendingUp className="size-4 shrink-0" aria-hidden="true" />
        <p>
          {poll.config.closingAt
            ? t('proposals.outcomes.eligibleAtDeadline')
            : t('proposals.outcomes.eligibleNow')}
        </p>
      </div>
    );
  }

  return null;
};
