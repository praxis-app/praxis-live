import {
  decisionTitle,
  responseCount,
} from '@/components/calls/call-decision-panel/call-decision-utils';
import { type PollRes } from '@/types/poll.types';
import { useTranslation } from 'react-i18next';

interface Props {
  decision?: PollRes | null;
  onOpen: () => void;
}

export const CallDecisionBanner = ({ decision, onOpen }: Props) => {
  const { t } = useTranslation();
  const title = decisionTitle(decision);
  const count = responseCount(decision);

  if (!decision) {
    return null;
  }

  return (
    <button
      type="button"
      className="bg-card hover:bg-accent mx-auto flex max-w-xl cursor-pointer items-center gap-2 rounded-full border px-3 py-2 text-sm shadow-sm transition-colors"
      onClick={onOpen}
    >
      <span className="font-medium">{t('calls.labels.decisionInProgress')}</span>
      <span className="text-muted-foreground min-w-0 truncate">{title}</span>
      {count && <span className="text-muted-foreground shrink-0">{count}</span>}
    </button>
  );
};
