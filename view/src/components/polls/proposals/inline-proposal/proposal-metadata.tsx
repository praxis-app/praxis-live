import { MIDDOT_WITH_SPACES } from '@/constants/shared.constants';
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
  onClick,
}: Props) => {
  const { t } = useTranslation();
  const ActionIcon = actionType ? actionIcons[actionType] : null;

  return (
    <button
      type="button"
      className="text-muted-foreground focus-visible:ring-ring flex max-w-full min-w-0 cursor-pointer flex-col items-start gap-1 rounded-sm pr-8 text-left text-sm font-medium transition-colors focus-visible:ring-2 focus-visible:outline-none @sm:flex-row @sm:items-center @sm:gap-0"
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
          <span className="hidden pr-0.25 pl-1.5 @sm:inline" aria-hidden="true">
            {MIDDOT_WITH_SPACES.trim()}
          </span>
        )}
        <LuListCheck className="size-4" aria-hidden="true" />
        <span>{t(MODEL_TRANSLATION_KEYS[decisionMakingModel])}</span>
      </span>
      {createdAt && (
        <span className="flex items-center">
          <span className="hidden px-1.5 @sm:inline" aria-hidden="true">
            {MIDDOT_WITH_SPACES.trim()}
          </span>
          <span>{timeAgo(createdAt)}</span>
        </span>
      )}
    </button>
  );
};
