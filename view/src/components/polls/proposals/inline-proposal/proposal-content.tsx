import { ProposalMetadata } from '@/components/polls/proposals/inline-proposal/proposal-metadata';
import { ProposalOutcome } from '@/components/polls/proposals/inline-proposal/proposal-outcome';
import { ProposalStatusBadge } from '@/components/polls/proposals/inline-proposal/proposal-status-badge';
import { VoteProgressDialog } from '@/components/polls/proposals/inline-proposal/vote-progress-dialog';
import { ProposalAction } from '@/components/polls/proposals/proposal-actions/proposal-action';
import { ProposalMenu } from '@/components/polls/proposals/proposal-menu';
import { ProposalVoteButtons } from '@/components/polls/proposals/proposal-vote-buttons';
import { FormattedText } from '@/components/shared/formatted-text';
import { CardAction } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { MIDDOT_WITH_SPACES } from '@/constants/shared.constants';
import { cn } from '@/lib/shared.utils';
import { timeFromNow } from '@/lib/time.utils';
import { type CallArtifactRes } from '@/types/call.types';
import { type ChannelRes } from '@/types/channel.types';
import { type PollRes } from '@/types/poll.types';
import { type CurrentUser } from '@/types/user.types';
import { type QueryKey } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export type SourceCallContext = 'in-call' | 'this-call' | 'another-call';

interface Props {
  poll: PollRes;
  channel: ChannelRes;
  feedQueryKey?: QueryKey;
  me?: CurrentUser;
  onPollChange?: () => void;
  onViewCall?: (callId: string) => void;
  onJoinCall?: (callId: string) => void;
  sourceCall?: CallArtifactRes;
  sourceCallContext?: SourceCallContext;
  isJoiningSourceCall?: boolean;
  canMoveToForum?: boolean;
  variant?: 'inline' | 'forum';
  updateCachedProposal?: (update: (proposal: PollRes) => PollRes) => void;
}

export const ProposalContent = ({
  poll,
  channel,
  feedQueryKey,
  me,
  onPollChange,
  onViewCall,
  onJoinCall,
  sourceCall,
  sourceCallContext = 'in-call',
  isJoiningSourceCall = false,
  canMoveToForum = false,
  variant = 'inline',
  updateCachedProposal,
}: Props) => {
  const { t } = useTranslation();
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const { id, body, myVote, config, action, stage, votes, memberCount } = poll;
  const isSourceCallActive =
    sourceCall?.status === 'starting' || sourceCall?.status === 'active';
  const sourceCallLabel = {
    'in-call': t('proposals.labels.createdInCall'),
    'this-call': t('proposals.labels.createdInThisCall'),
    'another-call': t('proposals.labels.createdInAnotherCall'),
  }[sourceCallContext];

  return (
    <>
      {canMoveToForum && feedQueryKey && (
        <ProposalMenu
          poll={poll}
          channel={channel}
          feedQueryKey={feedQueryKey}
          me={me}
        />
      )}
      <ProposalMetadata
        decisionMakingModel={config.decisionMakingModel ?? 'consensus'}
        actionType={action?.actionType}
        createdAt={variant === 'forum' ? poll.createdAt : undefined}
      />

      {body && <FormattedText text={body} className="pt-1 pb-2" />}

      {action && <ProposalAction action={action} />}

      <CardAction className="flex w-full flex-wrap gap-2">
        <ProposalVoteButtons
          pollId={id}
          channel={channel}
          feedQueryKey={feedQueryKey}
          myVote={myVote}
          stage={stage}
          decisionMakingModel={config.decisionMakingModel ?? 'consensus'}
          closingAt={config.closingAt}
          onVoteSuccess={onPollChange}
          updateCachedProposal={updateCachedProposal}
        />
      </CardAction>

      <Separator className="mt-5 mb-2.5" />

      <ProposalOutcome poll={poll} />

      <div
        className={cn(
          'flex min-w-0 flex-wrap items-center justify-between gap-2',
          variant === 'forum' && 'pt-2',
        )}
      >
        <div className="text-muted-foreground flex min-w-0 flex-wrap text-sm">
          <VoteProgressDialog
            votes={votes ?? []}
            config={config}
            memberCount={memberCount}
            isOpen={isDialogOpen}
            onOpenChange={setIsDialogOpen}
          />
          <div className="flex items-center">
            <span className="px-1.5" aria-hidden="true">
              {MIDDOT_WITH_SPACES.trim()}
            </span>
            {config.closingAt ? (
              timeFromNow(config.closingAt, true)
            ) : (
              <span>{t('time.infinity')}</span>
            )}
          </div>
        </div>
        <ProposalStatusBadge
          poll={poll}
          onClick={() => setIsDialogOpen(true)}
        />
      </div>

      {poll.sourceCallId && stage === 'voting' && (
        <div className="text-muted-foreground text-sm">
          {sourceCallLabel}
          {MIDDOT_WITH_SPACES}
          {isSourceCallActive && onJoinCall ? (
            <button
              type="button"
              className="text-primary cursor-pointer underline-offset-4 hover:underline disabled:pointer-events-none disabled:opacity-50"
              aria-label={t('calls.actions.joinActiveVideo')}
              disabled={isJoiningSourceCall}
              onClick={() => onJoinCall(poll.sourceCallId!)}
            >
              {t('calls.actions.joinCall')}
            </button>
          ) : (
            <a
              href={`#call-${poll.sourceCallId}`}
              className="text-primary underline-offset-4 hover:underline"
              onClick={(event) => {
                if (!onViewCall) return;
                event.preventDefault();
                onViewCall(poll.sourceCallId!);
              }}
            >
              {t('proposals.actions.viewCall')}
            </a>
          )}
        </div>
      )}
    </>
  );
};
