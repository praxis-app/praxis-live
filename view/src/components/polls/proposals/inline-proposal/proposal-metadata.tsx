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
}: Props) => {
  const { t } = useTranslation();
  const ActionIcon = actionType ? actionIcons[actionType] : null;

  return (
    <div className="text-muted-foreground flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs font-medium">
      {actionType && ActionIcon && (
        <>
          <span className="flex items-center gap-1.5">
            <ActionIcon className="size-3.5" aria-hidden="true" />
            <span>{t(actionTranslationKeys[actionType])}</span>
          </span>
          <span className="bg-border h-3.5 w-px" aria-hidden="true" />
        </>
      )}

      <span className="flex items-center gap-1.5">
        <LuListCheck className="size-3.5" aria-hidden="true" />
        <span>{t(modelTranslationKeys[decisionMakingModel])}</span>
      </span>
    </div>
  );
};
