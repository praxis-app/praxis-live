import { Progress } from '@/components/ui/progress';
import { timeFromNow } from '@/lib/time.utils';
import { truncate } from '@/lib/text.utils';
import { type ActiveDecisionRes } from '@/types/decision.types';
import { useTranslation } from 'react-i18next';
import { LuCheck, LuClock3 } from 'react-icons/lu';
import { MdForum, MdTag } from 'react-icons/md';
import { Link } from 'react-router-dom';

interface Props {
  decision: ActiveDecisionRes;
  serverPath: string;
  onOpenForumDecision: () => void;
}

export const DecisionPanelItem = ({
  decision,
  serverPath,
  onOpenForumDecision,
}: Props) => {
  const { t } = useTranslation();

  const ChannelIcon = decision.channelType === 'forum' ? MdForum : MdTag;
  const opensForumDecision = decision.channelType === 'forum';

  const responsePercentage =
    decision.memberCount > 0
      ? Math.min(100, (decision.responseCount / decision.memberCount) * 100)
      : 0;

  const decisionPath =
    opensForumDecision && decision.forumPostId
      ? `${serverPath}/c/${decision.channelId}/posts/${decision.forumPostId}`
      : `${serverPath}/c/${decision.channelId}`;

  return (
    <Link
      to={decisionPath}
      className="hover:bg-accent/60 focus-visible:ring-ring block rounded-lg border p-3 transition-colors focus-visible:ring-2 focus-visible:outline-none"
      aria-label={t('decisions.actions.openDecision', {
        type: t(`decisions.labels.${decision.pollType}`),
        channel: decision.channelName,
      })}
      onClick={opensForumDecision ? onOpenForumDecision : undefined}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
          {t(`decisions.labels.${decision.pollType}`)}
        </span>
        {decision.hasResponded && (
          <span className="text-primary flex items-center gap-1 text-xs">
            <LuCheck className="size-3.5" />
            {t('decisions.labels.responded')}
          </span>
        )}
      </div>

      <p className="mt-2 line-clamp-3 text-sm leading-5 font-medium">
        {truncate(decision.body || t('decisions.labels.untitled'), 180)}
      </p>

      <div className="text-muted-foreground mt-2 flex items-center gap-1 text-xs">
        <ChannelIcon className="size-3.5 shrink-0" />
        <span className="truncate">{decision.channelName}</span>
      </div>

      <div className="mt-3 space-y-1.5">
        <div className="text-muted-foreground flex items-center justify-between text-xs">
          <span>{t('decisions.labels.responses')}</span>
          <span>
            {decision.responseCount}/{decision.memberCount}
          </span>
        </div>
        <Progress value={responsePercentage} className="h-1.5" />
      </div>

      <div className="text-muted-foreground mt-3 flex items-center gap-1 text-xs">
        <LuClock3 className="size-3.5 shrink-0" />
        <span>
          {decision.closingAt
            ? timeFromNow(decision.closingAt, true)
            : t('decisions.labels.noDeadline')}
        </span>
      </div>
    </Link>
  );
};
