import { MIDDOT_WITH_SPACES } from '@/constants/shared.constants';
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
}

const actionIcons: Record<PollActionType, typeof LuMessageSquare> = {
  general: LuMessageSquare,
  'change-settings': LuSettings,
  'change-role': LuUserCog,
  'create-role': LuUserPlus,
  'plan-event': LuCalendarDays,
  test: LuFlaskConical,
};

const actionTranslationKeys = {
  general: 'proposals.actionTypes.general',
  'change-settings': 'proposals.actionTypes.changeSettings',
  'change-role': 'proposals.actionTypes.changeRole',
  'create-role': 'proposals.actionTypes.createRole',
  'plan-event': 'proposals.actionTypes.planEvent',
  test: 'proposals.actionTypes.test',
} as const satisfies Record<PollActionType, string>;

const modelTranslationKeys = {
  consent: 'proposals.labels.consent',
  consensus: 'proposals.labels.consensus',
  'majority-vote': 'proposals.labels.majority',
} as const;

export const ProposalMetadata = ({
  decisionMakingModel,
  actionType,
  createdAt,
}: Props) => {
  const { t } = useTranslation();
  const ActionIcon = actionType ? actionIcons[actionType] : null;

  return (
    <div className="text-muted-foreground flex min-w-0 flex-wrap items-center gap-y-1 text-sm font-medium">
      {actionType && ActionIcon && (
        <span className="flex items-center gap-1.5">
          <ActionIcon className="size-4" aria-hidden="true" />
          <span>{t(actionTranslationKeys[actionType])}</span>
        </span>
      )}

      <span className="flex items-center gap-1.5">
        {actionType && ActionIcon && (
          <span className="pr-0.25 pl-1.5" aria-hidden="true">
            {MIDDOT_WITH_SPACES.trim()}
          </span>
        )}
        <LuListCheck className="size-4" aria-hidden="true" />
        <span>{t(modelTranslationKeys[decisionMakingModel])}</span>
      </span>
      {createdAt && (
        <span className="flex items-center">
          <span className="px-1.5" aria-hidden="true">
            {MIDDOT_WITH_SPACES.trim()}
          </span>
          <span>{timeAgo(createdAt)}</span>
        </span>
      )}
    </div>
  );
};
