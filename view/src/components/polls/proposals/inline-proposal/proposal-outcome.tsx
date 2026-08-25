import { useVotingDeadline } from '@/hooks/use-voting-deadline';
import { getProposalRuleStatus } from '@/lib/poll.utils';
import { type PollRes } from '@/types/poll.types';
import { useTranslation } from 'react-i18next';
import { LuClock3, LuLoaderCircle, LuTrendingUp } from 'react-icons/lu';

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
  const missingRequiredDeadline =
    status.deadlineRequired && !poll.config.closingAt;

  if (poll.stage === 'ratified') {
    return null;
  }

  if (poll.stage === 'closed') {
    if (poll.closedReason) {
      return null;
    }

    // Rules that do not apply to this model already report as met, so only
    // the conditions that actually failed are named here.
    const failedRules = [
      status.agreementMet ? null : t('proposals.labels.approval'),
      status.quorumMet ? null : t('proposals.labels.quorum'),
      status.disagreementsMet ? null : t('proposals.labels.disagreements'),
      status.abstainsMet ? null : t('proposals.labels.abstentions'),
      status.blocksMet ? null : t('proposals.labels.blocks'),
      missingRequiredDeadline ? t('proposals.labels.deadline') : null,
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

  if (status.passes && !missingRequiredDeadline) {
    const EligibilityIcon = poll.config.closingAt ? LuClock3 : LuTrendingUp;

    return (
      <div className="flex items-center gap-2 text-sm text-emerald-700 dark:text-emerald-300">
        <EligibilityIcon className="size-4 shrink-0" aria-hidden="true" />
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
