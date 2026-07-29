import { CallArtifact } from '@/components/calls/call-artifact';
import { WelcomeMessage } from '@/components/invites/welcome-message';
import { BotMessage } from '@/components/messages/bot-message';
import { Message } from '@/components/messages/message';
import { InlinePoll } from '@/components/polls/inline-poll';
import { InlineProposal } from '@/components/polls/proposals/inline-proposal/inline-proposal';
import { ProposalForumReference } from '@/components/polls/proposals/proposal-forum-reference';
import { LocalStorageKeys } from '@/constants/shared.constants';
import { useAuthData } from '@/hooks/use-auth-data';
import { useInfiniteScroll } from '@/hooks/use-infinite-scroll';
import { useServerData } from '@/hooks/use-server-data';
import { cn } from '@/lib/shared.utils';
import { useAppStore } from '@/store/app.store';
import { type ChannelRes, type FeedItemRes } from '@/types/channel.types';
import { type QueryKey } from '@tanstack/react-query';
import { type RefObject, useEffect, useMemo, useState } from 'react';

const IN_VIEW_THRESHOLD = 50;

type FeedScrollMode = 'bottom-anchored' | 'natural';

interface Props {
  channel?: ChannelRes;
  feed: FeedItemRes[];
  feedQueryKey: QueryKey;
  feedBoxRef: RefObject<HTMLDivElement | null>;
  isLastPage: boolean;
  isLoadingMore: boolean;
  onLoadMore: () => void;
  isJoiningCall?: boolean;
  onJoinCall?: (callId: string) => void;
  scrollMode?: FeedScrollMode;
}

export const Feed = ({
  channel,
  feed,
  feedQueryKey,
  feedBoxRef,
  isLastPage,
  isLoadingMore,
  onLoadMore,
  isJoiningCall,
  onJoinCall,
  scrollMode = 'bottom-anchored',
}: Props) => {
  const { isAppLoading } = useAppStore();
  const { me, isAnon, isLoggedIn } = useAuthData();
  const { serverId } = useServerData();
  const isBottomAnchored = scrollMode === 'bottom-anchored';

  const [showWelcomeMessage, setShowWelcomeMessage] = useState(false);
  const [openCallDetailsId, setOpenCallDetailsId] = useState<string | null>(
    null,
  );

  const feedTopRef = useInfiniteScroll({
    hasNextPage: isBottomAnchored && feed.length > 0 && !isLastPage,
    isLoadingMore,
    onLoadMore,
    rootMargin: `${IN_VIEW_THRESHOLD}px`,
  });

  const visibleFeed = useMemo(
    () => (isBottomAnchored ? feed : [...feed].reverse()),
    [feed, isBottomAnchored],
  );
  const callsById = useMemo(() => {
    return new Map(
      feed
        .filter((item) => item.type === 'call')
        .map((call) => [call.id, call]),
    );
  }, [feed]);

  useEffect(() => {
    if (
      !isAppLoading &&
      (!isLoggedIn || isAnon) &&
      !localStorage.getItem(LocalStorageKeys.HideWelcomeMessage)
    ) {
      setShowWelcomeMessage(true);
    }
  }, [isLoggedIn, isAppLoading, isAnon]);

  return (
    <div
      ref={feedBoxRef}
      data-testid="feed"
      className={cn(
        'flex min-w-0 flex-1 gap-4.5 overflow-x-hidden overflow-y-scroll px-3.5 pt-2.5 pb-4',
        isBottomAnchored ? 'flex-col-reverse' : 'flex-col',
      )}
    >
      {showWelcomeMessage && (
        <WelcomeMessage onDismiss={() => setShowWelcomeMessage(false)} />
      )}

      {visibleFeed.map((item) => {
        if (!channel) {
          return null;
        }
        if (item.type === 'poll') {
          if (item.pollType === 'proposal') {
            const sourceCall = item.sourceCallId
              ? callsById.get(item.sourceCallId)
              : undefined;

            return (
              <InlineProposal
                key={`poll-${item.id}`}
                poll={item}
                channel={channel}
                feedQueryKey={feedQueryKey}
                me={me}
                sourceCall={sourceCall}
                isJoiningSourceCall={isJoiningCall}
                onJoinCall={onJoinCall}
                onViewCall={setOpenCallDetailsId}
                canMoveToForum
              />
            );
          }
          return (
            <InlinePoll
              key={`poll-${item.id}`}
              poll={item}
              channel={channel}
              feedQueryKey={feedQueryKey}
              me={me}
            />
          );
        }
        if (item.type === 'proposalMoved') {
          return (
            <ProposalForumReference
              key={`proposal-moved-${item.id}`}
              reference={item}
              me={me}
            />
          );
        }
        if (item.type === 'call') {
          return (
            <CallArtifact
              key={`call-${item.id}`}
              call={item}
              channel={channel}
              serverId={serverId}
              me={me}
              isJoining={isJoiningCall}
              onJoinCall={onJoinCall}
              detailsOpen={openCallDetailsId === item.id}
              onDetailsOpenChange={(open) =>
                setOpenCallDetailsId(open ? item.id : null)
              }
            />
          );
        }
        if (item.bot) {
          return <BotMessage key={`message-${item.id}`} message={item} />;
        }
        return (
          <Message
            key={`message-${item.id}`}
            channelId={channel?.id}
            serverId={serverId}
            message={item}
            me={me}
          />
        );
      })}

      {/* Bottom is top due to `column-reverse` in the channel feed. */}
      <div ref={feedTopRef} className="pb-0.5" />
    </div>
  );
};
