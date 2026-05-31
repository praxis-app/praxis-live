import { VoteProgressDialog } from '@/components/polls/proposals/inline-proposal/vote-progress-dialog';
import { ProposalAction } from '@/components/polls/proposals/proposal-actions/proposal-action';
import { ProposalVoteButtons } from '@/components/polls/proposals/proposal-vote-buttons';
import { FormattedText } from '@/components/shared/formatted-text';
import { Badge } from '@/components/ui/badge';
import { Card, CardAction } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { UserAvatar } from '@/components/users/user-avatar';
import { UserProfileDrawer } from '@/components/users/user-profile-drawer';
import { MIDDOT_WITH_SPACES } from '@/constants/shared.constants';
import { truncate } from '@/lib/text.utils';
import { timeAgo, timeFromNow } from '@/lib/time.utils';
import { type ChannelRes } from '@/types/channel.types';
import { type PollRes } from '@/types/poll.types';
import { type CurrentUser } from '@/types/user.types';
import { type QueryKey } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FaClipboard } from 'react-icons/fa';

interface Props {
  poll: PollRes;
  channel: ChannelRes;
  feedQueryKey: QueryKey;
  me?: CurrentUser;
  onPollChange?: () => void;
  onViewCall?: (callId: string) => void;
}

export const InlineProposal = ({
  poll,
  channel,
  feedQueryKey,
  me,
  onPollChange,
  onViewCall,
}: Props) => {
  const { t } = useTranslation();
  const [isDialogOpen, setIsDialogOpen] = useState(false);

  const {
    id,
    body,
    user,
    myVote,
    config,
    action,
    stage,
    votes,
    createdAt,
    memberCount,
  } = poll;

  const name = user.displayName || user.name;
  const truncatedName = truncate(name, 18);
  const formattedDate = timeAgo(createdAt);

  const label = body
    ? `${t('proposals.labels.consensusProposal')}: ${body}`
    : t('proposals.labels.consensusProposal');

  return (
    <article aria-label={label} className="flex max-w-full min-w-0 gap-4 pt-1">
      <UserProfileDrawer
        name={truncatedName}
        userId={user.id}
        me={me}
        trigger={
          <button className="shrink-0 cursor-pointer self-start">
            <UserAvatar
              name={name}
              userId={user.id}
              imageId={user.profilePicture?.id}
              className="mt-0.5"
            />
          </button>
        }
      />

      <div className="max-w-full min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-1.5 pb-1">
          <UserProfileDrawer
            name={truncatedName}
            userId={user.id}
            me={me}
            trigger={
              <button className="cursor-pointer font-medium">
                {truncatedName}
              </button>
            }
          />
          <div className="text-muted-foreground text-sm">{formattedDate}</div>
        </div>

        <Card className="before:border-l-border @container relative max-w-full min-w-0 gap-3.5 rounded-md px-3 py-3.5 before:absolute before:top-0 before:bottom-0 before:left-0 before:mt-[-0.025rem] before:mb-[-0.025rem] before:w-3 before:rounded-l-md before:border-l-3">
          <div className="text-muted-foreground flex min-w-0 items-center gap-1.5 font-medium">
            <FaClipboard className="mb-0.5" />
            {t('proposals.labels.consensusProposal')}
          </div>

          {body && <FormattedText text={body} className="pt-1 pb-2" />}

          {action && <ProposalAction action={action} />}

          <CardAction className="flex w-full flex-wrap gap-2">
            <ProposalVoteButtons
              pollId={id}
              channel={channel}
              feedQueryKey={feedQueryKey}
              myVote={myVote}
              stage={stage}
              onVoteSuccess={onPollChange}
            />
          </CardAction>

          <Separator className="my-1" />

          <div className="flex min-w-0 flex-wrap items-center justify-between gap-2">
            <div className="text-muted-foreground flex min-w-0 flex-wrap gap-3 text-sm">
              <VoteProgressDialog
                votes={votes ?? []}
                config={config}
                memberCount={memberCount}
                isOpen={isDialogOpen}
                onOpenChange={setIsDialogOpen}
              />
              <div className="flex items-center">
                {config?.closingAt ? (
                  timeFromNow(config.closingAt, true)
                ) : (
                  <span className="text-lg">{t('time.infinity')}</span>
                )}
              </div>
            </div>
            <Badge variant="outline">{t(`proposals.labels.${stage}`)}</Badge>
          </div>

          {poll.sourceCallId && stage === 'voting' && (
            <div className="text-muted-foreground text-xs">
              {t('proposals.labels.createdInCall')}
              {MIDDOT_WITH_SPACES}
              <a
                href={`#call-${poll.sourceCallId}`}
                className="text-primary underline-offset-4 hover:underline"
                onClick={(event) => {
                  if (!onViewCall) {
                    return;
                  }
                  event.preventDefault();
                  onViewCall(poll.sourceCallId!);
                }}
              >
                {t('proposals.actions.viewCall')}
              </a>
            </div>
          )}
        </Card>
      </div>
    </article>
  );
};
