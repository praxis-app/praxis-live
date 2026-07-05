import { Badge } from '@/components/ui/badge';
import { getProposalRuleStatus } from '@/lib/poll.utils';
import { cn } from '@/lib/shared.utils';
import { type PollRes, type PollStage } from '@/types/poll.types';
import { useTranslation } from 'react-i18next';
import {
  LuCircleCheck,
  LuCircleX,
  LuOctagonAlert,
  LuPencil,
  LuVote,
} from 'react-icons/lu';

interface Props {
  poll: PollRes;
  onClick: () => void;
}

const stageStyles: Record<PollStage, string> = {
  voting: 'border-blue-500/30 bg-blue-500/10 text-blue-600 dark:text-blue-300',
  ratified:
    'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300',
  revision:
    'border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300',
  closed: 'border-destructive/30 bg-destructive/10 text-destructive',
};

const stageIcons: Record<PollStage, typeof LuVote> = {
  voting: LuVote,
  ratified: LuCircleCheck,
  revision: LuPencil,
  closed: LuCircleX,
};

export const ProposalStatusBadge = ({ poll, onClick }: Props) => {
  const { t } = useTranslation();
  const { stage } = poll;
  const status = getProposalRuleStatus(
    poll.votes ?? [],
    poll.config,
    poll.memberCount,
  );
  const limitReached =
    stage === 'voting' &&
    poll.config.decisionMakingModel !== 'majority-vote' &&
    (!status.disagreementsMet || !status.abstainsMet || !status.blocksMet);
  const Icon = limitReached ? LuOctagonAlert : stageIcons[stage];

  return (
    <Badge
      asChild
      variant="outline"
      className={cn(
        'cursor-pointer gap-1.5 transition-colors hover:brightness-95 dark:hover:brightness-110',
        limitReached
          ? 'border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-400'
          : stageStyles[stage],
      )}
    >
      <button type="button" onClick={onClick}>
        <Icon aria-hidden="true" />
        {t(`proposals.labels.${stage}`)}
        {limitReached && ` · ${t('proposals.labels.limitReached')}`}
      </button>
    </Badge>
  );
};
