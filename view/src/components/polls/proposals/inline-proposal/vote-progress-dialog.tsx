import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Progress } from '@/components/ui/progress';
import { ProposalRuleRow } from '@/components/polls/proposals/inline-proposal/proposal-rule-row';
import { getProgressPercentage, getProposalRuleStatus } from '@/lib/poll.utils';
import { type PollClosedReason, type PollConfigRes } from '@/types/poll.types';
import { type VoteRes } from '@/types/vote.types';
import { VisuallyHidden } from '@radix-ui/react-visually-hidden';
import { useTranslation } from 'react-i18next';
import { LuClock3, LuTrendingUp } from 'react-icons/lu';
import { MdHowToVote } from 'react-icons/md';

interface Props {
  votes: VoteRes[];
  config: PollConfigRes;
  memberCount: number;
  closedReason?: PollClosedReason;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

export const VoteProgressDialog = ({
  votes,
  config,
  memberCount,
  closedReason,
  isOpen,
  onOpenChange,
}: Props) => {
  const { t } = useTranslation();
  const status = getProposalRuleStatus(votes, config, memberCount);
  const showAgreement = status.agreementApplies;
  const showLimits = status.limitsApply;
  const requiredAgreements = Math.max(1, status.requiredAgreements);
  const agreementsPercentage = getProgressPercentage(
    status.agreements,
    requiredAgreements,
  );
  const quorumPercentage = getProgressPercentage(
    status.totalVotes,
    status.requiredQuorum,
  );

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogTrigger asChild>
        <button className="flex cursor-pointer items-center gap-1.5">
          <MdHowToVote className="text-muted-foreground" />
          <span>
            {t('proposals.labels.totalVotes', { count: status.totalVotes })}
          </span>
        </button>
      </DialogTrigger>

      <DialogContent className="md:w-lg">
        <DialogHeader>
          <DialogTitle className="mb-0">
            {t('proposals.headers.voteProgress')}
          </DialogTitle>
          <VisuallyHidden>
            <DialogDescription>
              {t('proposals.headers.voteProgress')}
            </DialogDescription>
          </VisuallyHidden>
        </DialogHeader>

        <div className="space-y-6 pt-2">
          {config.decisionMakingModel === 'consent' && (
            <p className="border-border bg-muted/50 text-muted-foreground rounded-md border px-3 py-2 text-sm">
              {t('proposals.descriptions.consentRules')}
            </p>
          )}

          {status.passes && config.closingAt && !status.deadlineReached && (
            <div className="flex items-center gap-2 text-sm text-emerald-700 dark:text-emerald-300">
              <LuClock3 className="size-4 shrink-0" aria-hidden="true" />
              <p>{t('proposals.outcomes.eligibleAtDeadline')}</p>
            </div>
          )}

          {status.passes &&
            !config.closingAt &&
            !status.deadlineRequired && (
              <div className="flex items-center gap-2 text-sm text-emerald-700 dark:text-emerald-300">
                <LuTrendingUp
                  className="size-4 shrink-0"
                  aria-hidden="true"
                />
                <p>{t('proposals.outcomes.eligibleNow')}</p>
              </div>
            )}

          {closedReason === 'event-start-elapsed' && (
            <p className="border-border bg-muted/50 text-muted-foreground rounded-md border px-3 py-2 text-sm">
              {t('proposals.outcomes.eventStartElapsed')}
            </p>
          )}

          {closedReason === 'event-host-ineligible' && (
            <p className="border-border bg-muted/50 text-muted-foreground rounded-md border px-3 py-2 text-sm">
              {t('proposals.outcomes.eventHostIneligible')}
            </p>
          )}

          {showAgreement && (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span className="font-medium">
                  {t('proposals.labels.thresholdProgress')}
                </span>
                <span
                  className={
                    status.agreementMet
                      ? 'text-green-600 dark:text-green-400'
                      : 'text-muted-foreground'
                  }
                >
                  {status.agreementMet
                    ? t('proposals.labels.thresholdMet')
                    : t('proposals.labels.thresholdNotMet')}
                </span>
              </div>
              <Progress value={agreementsPercentage} />
              <p className="text-muted-foreground text-sm">
                {t('proposals.descriptions.thresholdStatus', {
                  current: status.agreements,
                  required: requiredAgreements,
                  threshold: config.agreementThreshold,
                })}
              </p>
            </div>
          )}

          {status.quorumApplies && (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span className="font-medium">
                  {t('proposals.labels.quorumProgress')}
                </span>
                <span
                  className={
                    status.quorumMet
                      ? 'text-green-600 dark:text-green-400'
                      : 'text-muted-foreground'
                  }
                >
                  {status.quorumMet
                    ? t('proposals.labels.quorumMet')
                    : t('proposals.labels.quorumNotMet')}
                </span>
              </div>
              <Progress value={quorumPercentage} />
              <p className="text-muted-foreground text-sm">
                {t('proposals.descriptions.quorumStatus', {
                  current: status.totalVotes,
                  required: status.requiredQuorum,
                  threshold: config.quorumThreshold,
                })}
              </p>
            </div>
          )}

          {showLimits && (
            <div className="grid gap-3 text-sm">
              <ProposalRuleRow
                label={t('proposals.labels.disagreementLimit')}
                value={t('proposals.descriptions.voteLimitStatus', {
                  current: status.disagreements,
                  limit: config.disagreementsLimit ?? 0,
                })}
                met={status.disagreementsMet}
              />
              <ProposalRuleRow
                label={t('proposals.labels.abstentionLimit')}
                value={t('proposals.descriptions.voteLimitStatus', {
                  current: status.abstains,
                  limit: config.abstainsLimit ?? 0,
                })}
                met={status.abstainsMet}
              />
              <ProposalRuleRow
                label={t('proposals.labels.blockStatus')}
                value={t('proposals.descriptions.blockStatus', {
                  count: status.blocks,
                })}
                met={status.blocksMet}
              />
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
};
