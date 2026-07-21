import { MIDDOT_WITH_SPACES } from '@/constants/shared.constants';
import { cn } from '@/lib/shared.utils';
import {
  ACTION_TRANSLATION_KEYS,
  MODEL_TRANSLATION_KEYS,
} from '@/components/polls/proposals/inline-proposal/proposal-metadata.constants';
import { timeAgo } from '@/lib/time.utils';
import { type PollActionType } from '@/types/poll-action.types';
import { type DecisionMakingModel } from '@/types/poll.types';
import { useTranslation } from 'react-i18next';
import {
  LuCalendarDays,
  LuFlaskConical,
  LuListCheck,
  LuMessageSquare,
  LuSettings,
  LuUserCog,
  LuUserPlus,
} from 'react-icons/lu';

interface Props {
  decisionMakingModel: DecisionMakingModel;
  actionType?: PollActionType;
  createdAt?: string;
  variant?: 'inline' | 'forum';
  onClick: () => void;
}

const actionIcons: Record<PollActionType, typeof LuMessageSquare> = {
  general: LuMessageSquare,
  'change-settings': LuSettings,
  'change-role': LuUserCog,
  'create-role': LuUserPlus,
  'plan-event': LuCalendarDays,
  test: LuFlaskConical,
};

export const ProposalMetadata = ({
  decisionMakingModel,
  actionType,
  createdAt,
  variant = 'inline',
  onClick,
}: Props) => {
  const { t } = useTranslation();
  const ActionIcon = actionType ? actionIcons[actionType] : null;

  return (
    <button
      type="button"
      className={cn(
        'text-muted-foreground focus-visible:ring-ring flex max-w-full min-w-0 cursor-pointer items-start rounded-sm pr-8 text-left text-sm font-medium transition-colors focus-visible:ring-2 focus-visible:outline-none',
        variant === 'forum'
          ? 'flex-row items-center gap-0'
          : 'flex-col gap-1 @sm:flex-row @sm:items-center @sm:gap-0',
      )}
      onClick={onClick}
    >
      {actionType && ActionIcon && (
        <span className="flex items-center gap-1.5">
          <ActionIcon className="size-4" aria-hidden="true" />
          <span>{t(ACTION_TRANSLATION_KEYS[actionType])}</span>
        </span>
      )}

      <span className="flex items-center gap-1.5">
        {actionType && ActionIcon && (
          <span
            className={cn(
              'pr-0.25 pl-1.5',
              variant !== 'forum' && 'hidden @sm:inline',
            )}
            aria-hidden="true"
          >
            {MIDDOT_WITH_SPACES.trim()}
          </span>
        )}
        <LuListCheck className="size-4" aria-hidden="true" />
        <span>{t(MODEL_TRANSLATION_KEYS[decisionMakingModel])}</span>
      </span>
      {createdAt && (
        <span className="flex items-center">
          <span
            className={cn(
              'px-1.5',
              variant !== 'forum' && 'hidden @sm:inline',
            )}
            aria-hidden="true"
          >
            {MIDDOT_WITH_SPACES.trim()}
          </span>
          <span>{timeAgo(createdAt)}</span>
        </span>
      )}
    </button>
  );
};
