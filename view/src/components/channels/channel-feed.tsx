import { CallArtifact } from '@/components/calls/call-artifact';
import { WelcomeMessage } from '@/components/invites/welcome-message';
import { BotMessage } from '@/components/messages/bot-message';
import { Message } from '@/components/messages/message';
import { InlinePoll } from '@/components/polls/inline-poll';
import { InlineProposal } from '@/components/polls/proposals/inline-proposal/inline-proposal';
import { LocalStorageKeys } from '@/constants/shared.constants';
import { useAuthData } from '@/hooks/use-auth-data';
import { useInView } from '@/hooks/use-in-view';
import { useScrollDirection } from '@/hooks/use-scroll-direction';
import { useServerData } from '@/hooks/use-server-data';
import { cn, debounce, throttle } from '@/lib/shared.utils';
import { useAppStore } from '@/store/app.store';
import { type ChannelRes, type FeedItemRes } from '@/types/channel.types';
import { type QueryKey } from '@tanstack/react-query';
import {
  type RefObject,
  type UIEvent,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

const LOAD_MORE_THROTTLE_MS = 1500;
const IN_VIEW_THRESHOLD = 50;

type FeedScrollMode = 'bottom-anchored' | 'natural';

interface Props {
  channel?: ChannelRes;
  feed: FeedItemRes[];
  feedQueryKey: QueryKey;
  feedBoxRef: RefObject<HTMLDivElement | null>;
  isLastPage: boolean;
  onLoadMore: () => void;
  isJoiningCall?: boolean;
  onJoinCall?: (callId: string) => void;
  scrollMode?: FeedScrollMode;
}

export const ChannelFeed = ({
  channel,
  feed,
  feedQueryKey,
  feedBoxRef,
  isLastPage,
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
  const [scrollPosition, setScrollPosition] = useState(0);

  const scrollDirection = useScrollDirection(feedBoxRef);
  const feedTopRef = useRef<HTMLDivElement>(null);
  const onLoadMoreRef = useRef(onLoadMore);

  // Create throttled function once and reuse it
  const throttledOnLoadMore = useRef(
    throttle(() => {
      onLoadMoreRef.current();
    }, LOAD_MORE_THROTTLE_MS),
  ).current;

  const { setViewed } = useInView(feedTopRef, `${IN_VIEW_THRESHOLD}px`, () => {
    if (!isBottomAnchored) {
      return;
    }

    if (scrollPosition < -IN_VIEW_THRESHOLD && scrollDirection === 'up') {
      setViewed(false);

      if (!isLastPage) {
        throttledOnLoadMore();
      }
    }
  });

  // Debounced scroll handler to improve performance
  const debouncedSetScrollPosition = useMemo(
    () => debounce((position: number) => setScrollPosition(position), 16),
    [setScrollPosition],
  );

  const handleScroll = (e: UIEvent<HTMLDivElement>) => {
    const target = e.target as HTMLDivElement;
    debouncedSetScrollPosition(target.scrollTop);
  };

  const visibleFeed = useMemo(
    () => (isBottomAnchored ? feed : [...feed].reverse()),
    [feed, isBottomAnchored],
  );

  // Cleanup debounced function on unmount
  useEffect(() => {
    return () => {
      debouncedSetScrollPosition.clear();
    };
  }, [debouncedSetScrollPosition]);

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
      className={cn(
        'flex min-w-0 flex-1 gap-4.5 overflow-x-hidden overflow-y-scroll px-3.5 pt-2.5 pb-4',
        isBottomAnchored ? 'flex-col-reverse' : 'flex-col',
      )}
      onScroll={handleScroll}
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
            return (
              <InlineProposal
                key={`poll-${item.id}`}
                poll={item}
                channel={channel}
                feedQueryKey={feedQueryKey}
                me={me}
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
