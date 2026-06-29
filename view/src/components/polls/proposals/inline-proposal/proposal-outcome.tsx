import { useVotingDeadline } from '@/hooks/use-voting-deadline';
import { getProposalRuleStatus } from '@/lib/poll.utils';
import { type PollRes } from '@/types/poll.types';
import { useTranslation } from 'react-i18next';

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
    return (
      <p className="text-sm text-green-700 dark:text-green-400">
        {t('proposals.outcomes.ratified')}
      </p>
    );
  }

  if (poll.stage === 'closed') {
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
      <p className="text-destructive text-sm">
        {t('proposals.outcomes.closedWithoutConsensus')}
        {failedRules.length > 0 &&
          ` ${t('proposals.outcomes.failedRules', { rules: failedRules.join(', ') })}`}
      </p>
    );
  }

  if (deadlineHasPassed) {
    return (
      <p className="text-muted-foreground text-sm">
        {t('proposals.outcomes.finalizing')}
      </p>
    );
  }

  if (status.passes) {
    return (
      <p className="text-sm text-green-700 dark:text-green-400">
        {poll.config.closingAt
          ? t('proposals.outcomes.eligibleAtDeadline')
          : t('proposals.outcomes.eligibleNow')}
      </p>
    );
  }

  return (
    <p className="text-muted-foreground text-sm">
      {poll.config.closingAt
        ? t('proposals.outcomes.waitingForDeadline')
        : t('proposals.outcomes.votingInProgress')}
    </p>
  );
};
