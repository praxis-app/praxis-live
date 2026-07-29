import { api } from '@/client/api-client';
import { Feed } from '@/components/feeds/feed';
import { ChannelTopNav } from '@/components/channels/channel-top-nav';
import { DecisionsPanel } from '@/components/decisions/decisions-panel';
import { MessageForm } from '@/components/messages/message-form';
import { LeftNavDesktop } from '@/components/nav/left-nav-desktop';
import { MESSAGES_PAGE_SIZE } from '@/constants/message.constants';
import { useAuthData } from '@/hooks/use-auth-data';
import { useChannelCall } from '@/hooks/use-channel-call';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useInstanceCapabilitiesQuery } from '@/hooks/use-instance-capabilities-query';
import { useServerData } from '@/hooks/use-server-data';
import { useSubscription } from '@/hooks/use-subscription';
import { useAuthStore } from '@/store/auth.store';
import {
  type ChannelRes,
  type FeedItemRes,
  type FeedQuery,
  type FeedQueryPage,
} from '@/types/channel.types';
import { type CallArtifactRes } from '@/types/call.types';
import { type MessageRes } from '@/types/message.types';
import { type PollRes } from '@/types/poll.types';
import { type ProposalForumReferenceRes } from '@/types/forum.types';
import { type PubSubMessage } from '@/types/shared.types';
import { PubSubMessageType } from '@/constants/pub-sub.constants';
import {
  preserveFeedImages,
  preserveFeedItemImages,
  replaceProposalWithForumReference,
} from '@/lib/feed.utils';
import { channelPubSubTopic } from '@/lib/pub-sub.utils';
import { useInfiniteQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';

interface NewMessagePayload {
  type: PubSubMessageType.MESSAGE;
  message: MessageRes;
}

interface NewPollPayload {
  type: PubSubMessageType.POLL;
  poll: PollRes;
}

interface ProposalMovedPayload {
  type: PubSubMessageType.PROPOSAL_MOVED;
  reference: ProposalForumReferenceRes;
}

interface NewCallPayload {
  type: PubSubMessageType.CALL;
  call: CallArtifactRes;
}

interface ImageMessagePayload {
  type: PubSubMessageType.IMAGE;
  isPlaceholder: boolean;
  messageId: string;
  imageId: string;
}

interface Props {
  channel?: ChannelRes;
  isDecisionsPanelOpen: boolean;
  onCloseDecisionsPanel: () => void;
  onToggleDecisionsPanel: () => void;
}

export const TextChannelView = ({
  channel,
  isDecisionsPanelOpen,
  onCloseDecisionsPanel,
  onToggleDecisionsPanel,
}: Props) => {
  const { inviteToken } = useAuthStore();
  const [isLastPage, setIsLastPage] = useState(false);

  const feedBoxRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();
  const isDesktop = useIsDesktop();

  const { me, isMeSuccess, isAuthError } = useAuthData();
  const { data: capabilities } = useInstanceCapabilitiesQuery();
  const { server, serverId } = useServerData();
  const videoCallsEnabled = capabilities?.videoCallsEnabled === true;

  const {
    callConfig,
    callPreferences,
    cancelPreJoin,
    confirmJoinCall,
    isJoining,
    isPreJoinOpen,
    joinCall,
    leaveCall,
  } = useChannelCall(serverId, channel?.id);

  const feedQueryKey = ['servers', serverId, 'channels', channel?.id, 'feed'];

  const { data: feedData, fetchNextPage, isFetchingNextPage } =
    useInfiniteQuery({
      queryKey: feedQueryKey,
      queryFn: async ({ pageParam }) => {
        if (!serverId || !channel?.id) {
          throw new Error('Server ID and channel ID are required');
        }
        const result = await api.getChannelFeed(
          serverId,
          channel.id,
          pageParam,
          MESSAGES_PAGE_SIZE,
          inviteToken,
        );
        const isLast = result.feed.length === 0;
        if (isLast) {
          setIsLastPage(true);
        }
        const existingFeed = queryClient
          .getQueryData<FeedQuery>(feedQueryKey)
          ?.pages.flatMap((page) => page.feed);
        return {
          ...result,

          // Keep locally loaded image srcs from being lost on feed refresh.
          feed: preserveFeedImages(existingFeed, result.feed),
        };
      },
      getNextPageParam: (_lastPage, pages) => {
        return pages.flatMap((page) => page.feed).length;
      },
      initialPageParam: 0,
      enabled: !!serverId && !!channel?.id && (isMeSuccess || isAuthError),
    });

  // Listen for new messages
  useSubscription(
    channelPubSubTopic('new-message', serverId, channel?.id, me?.id),
    {
      onMessage: (event) => {
        const { body }: PubSubMessage<NewMessagePayload | ImageMessagePayload> =
          JSON.parse(event.data);
        if (!body) {
          return;
        }

        // Update cache with new message or update existing bot message
        if (body.type === PubSubMessageType.MESSAGE) {
          const messagePayload = body.message;
          const incomingFeedItem = {
            ...messagePayload,
            type: 'message' as const,
          };

          queryClient.setQueryData<FeedQuery>(feedQueryKey, (oldData) => {
            if (!oldData) {
              return {
                pages: [{ feed: [incomingFeedItem] }],
                pageParams: [0],
              };
            }
            const pages = oldData.pages.map((page, index): FeedQueryPage => {
              // Check if message already exists (for bot message updates)
              const existingIndex = page.feed.findIndex(
                (item) =>
                  item.type === 'message' && item.id === messagePayload.id,
              );

              if (existingIndex !== -1) {
                // Update existing message (bot message with command result)
                const updatedFeed = [...page.feed];
                const existingMessage = page.feed[existingIndex];
                updatedFeed[existingIndex] = preserveFeedItemImages(
                  existingMessage.type === 'message'
                    ? existingMessage
                    : undefined,
                  incomingFeedItem,
                );

                // Sort by createdAt descending (newest first)
                updatedFeed.sort(
                  (a, b) =>
                    new Date(b.createdAt).getTime() -
                    new Date(a.createdAt).getTime(),
                );
                return { feed: updatedFeed };
              }

              // Add new message to first page only
              if (index === 0) {
                const updatedFeed = [incomingFeedItem, ...page.feed];
                // Sort by createdAt descending (newest first)
                updatedFeed.sort(
                  (a, b) =>
                    new Date(b.createdAt).getTime() -
                    new Date(a.createdAt).getTime(),
                );
                return { feed: updatedFeed };
              }
              return page;
            });
            return { pages, pageParams: oldData.pageParams };
          });
        }

        // Update cache with image status once uploaded
        if (body.type === PubSubMessageType.IMAGE) {
          queryClient.setQueryData<FeedQuery>(feedQueryKey, (oldData) => {
            if (!oldData) {
              return { pages: [], pageParams: [] };
            }

            const pages = oldData.pages.map((page): FeedQueryPage => {
              const feed = page.feed.map((item) => {
                if (item.type !== 'message') {
                  return item;
                }
                if (item.id !== body.messageId || !item.images) {
                  return item;
                }
                const images = item.images.map((image) =>
                  image.id === body.imageId
                    ? { ...image, isPlaceholder: false }
                    : image,
                );
                return { ...item, images } as FeedItemRes;
              });
              return { feed };
            });

            return { pages, pageParams: oldData.pageParams };
          });
        }

        scrollToBottom();
      },
      enabled: !!me && !!channel && !!serverId,
    },
  );

  // Listen for new polls
  useSubscription(
    channelPubSubTopic('new-poll', serverId, channel?.id, me?.id),
    {
      onMessage: (event) => {
        const { body }: PubSubMessage<NewPollPayload | ProposalMovedPayload> =
          JSON.parse(event.data);
        if (!body) {
          return;
        }
        if (body.type === PubSubMessageType.POLL) {
          const newFeedItem: FeedItemRes = {
            ...(body.poll as FeedItemRes & { type: 'poll' }),
            type: 'poll',
          };
          queryClient.setQueryData<FeedQuery>(feedQueryKey, (oldData) => {
            if (!oldData) {
              return {
                pages: [{ feed: [newFeedItem] }],
                pageParams: [0],
              };
            }
            const pages = oldData.pages.map((page, index): FeedQueryPage => {
              if (index === 0) {
                const existingIndex = page.feed.findIndex(
                  (fi) => fi.type === 'poll' && fi.id === newFeedItem.id,
                );
                if (existingIndex !== -1) {
                  const updatedFeed = [...page.feed];
                  updatedFeed[existingIndex] = newFeedItem;
                  return { feed: updatedFeed };
                }
                const updatedFeed = [newFeedItem, ...page.feed];
                // Sort by createdAt descending (newest first)
                updatedFeed.sort(
                  (a, b) =>
                    new Date(b.createdAt).getTime() -
                    new Date(a.createdAt).getTime(),
                );
                return { feed: updatedFeed };
              }
              return page;
            });
            return { pages, pageParams: oldData.pageParams };
          });
        }
        if (body.type === PubSubMessageType.PROPOSAL_MOVED) {
          queryClient.setQueryData<FeedQuery>(feedQueryKey, (oldData) =>
            replaceProposalWithForumReference(oldData, body.reference),
          );
          return;
        }
        scrollToBottom();
      },
      enabled: !!me && !!channel && !!serverId,
    },
  );

  // Listen for new calls
  useSubscription(
    channelPubSubTopic('new-call', serverId, channel?.id, me?.id),
    {
      onMessage: (event) => {
        const { body }: PubSubMessage<NewCallPayload> = JSON.parse(event.data);
        if (!body) {
          return;
        }
        if (body.type === PubSubMessageType.CALL) {
          const newFeedItem: FeedItemRes = body.call;
          queryClient.setQueryData<FeedQuery>(feedQueryKey, (oldData) => {
            if (!oldData) {
              return {
                pages: [{ feed: [newFeedItem] }],
                pageParams: [0],
              };
            }
            const pages = oldData.pages.map((page, index): FeedQueryPage => {
              const existingIndex = page.feed.findIndex(
                (fi) => fi.type === 'call' && fi.id === newFeedItem.id,
              );
              if (existingIndex !== -1) {
                const updatedFeed = [...page.feed];
                updatedFeed[existingIndex] = newFeedItem;
                return { feed: updatedFeed };
              }

              if (index === 0) {
                const updatedFeed = [newFeedItem, ...page.feed];
                // Sort by createdAt descending (newest first)
                updatedFeed.sort(
                  (a, b) =>
                    new Date(b.createdAt).getTime() -
                    new Date(a.createdAt).getTime(),
                );
                return { feed: updatedFeed };
              }
              return page;
            });
            return { pages, pageParams: oldData.pageParams };
          });
        }
        scrollToBottom();
      },
      enabled: !!me && !!channel && !!serverId,
    },
  );

  // Reset isLastPage when switching channels
  useEffect(() => {
    if (channel?.id) {
      setIsLastPage(false);
    }
  }, [channel?.id]);

  const scrollToBottom = () => {
    if (feedBoxRef.current && feedBoxRef.current.scrollTop >= -200) {
      feedBoxRef.current.scrollTop = 0;
    }
  };

  return (
    <div className="fixed top-0 right-0 bottom-0 left-0 flex">
      {isDesktop && <LeftNavDesktop me={me} />}

      <div className="flex min-w-0 flex-1 flex-col">
        <ChannelTopNav
          channel={channel}
          callConfig={callConfig}
          callPreferences={callPreferences}
          serverName={server?.name}
          isJoiningCall={isJoining}
          isPreJoinOpen={isPreJoinOpen}
          videoCallsEnabled={videoCallsEnabled}
          onCancelPreJoin={cancelPreJoin}
          onConfirmJoinCall={confirmJoinCall}
          onJoinCall={joinCall}
          onLeaveCall={leaveCall}
          isDecisionsPanelOpen={isDecisionsPanelOpen}
          onToggleDecisionsPanel={onToggleDecisionsPanel}
        />

        <Feed
          channel={channel}
          feedBoxRef={feedBoxRef}
          onLoadMore={fetchNextPage}
          feed={feedData?.pages.flatMap((page) => page.feed) || []}
          feedQueryKey={feedQueryKey}
          isLastPage={isLastPage}
          isLoadingMore={isFetchingNextPage}
          isJoiningCall={isJoining}
          onJoinCall={videoCallsEnabled ? joinCall : undefined}
        />

        <MessageForm
          channelId={channel?.id}
          focusOnTyping={!callConfig}
          onSend={scrollToBottom}
        />
      </div>

      {isDesktop && (
        <DecisionsPanel
          isOpen={isDecisionsPanelOpen}
          onClose={onCloseDecisionsPanel}
        />
      )}
    </div>
  );
};
