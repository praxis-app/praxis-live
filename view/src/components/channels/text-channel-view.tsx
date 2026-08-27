import { api } from '@/client/api-client';
import { Feed } from '@/components/feeds/feed';
import { ChannelTopNav } from '@/components/channels/channel-top-nav';
import { DecisionsPanel } from '@/components/decisions/decisions-panel';
import { MessageForm } from '@/components/messages/message-form';
import { ThreadPanel } from '@/components/messages/thread/thread-panel';
import { getThreadQueryKey } from '@/components/messages/thread/thread-query.utils';
import { LeftNavDesktop } from '@/components/nav/left-nav-desktop';
import { ResizablePanelLayout } from '@/components/shared/resizable-panel-layout';
import { MESSAGES_PAGE_SIZE } from '@/constants/message.constants';
import { useAuthData } from '@/hooks/use-auth-data';
import { useChannelCall } from '@/hooks/use-channel-call';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useInstanceCapabilitiesQuery } from '@/hooks/use-instance-capabilities-query';
import { useFeedQuery } from '@/hooks/use-feed-query';
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
import { type MessageRes, type ThreadQuery } from '@/types/message.types';
import { type PollRes } from '@/types/poll.types';
import { type ProposalForumReferenceRes } from '@/types/forum.types';
import { type PubSubMessage } from '@/types/shared.types';
import { type RightPanel } from '@/types/right-panel.types';
import { PubSubMessageType } from '@/constants/pub-sub.constants';
import {
  preserveFeedImages,
  preserveFeedItemImages,
  replaceProposalWithForumReference,
} from '@/lib/feed.utils';
import { channelPubSubTopic } from '@/lib/pub-sub.utils';
import { cn } from '@/lib/shared.utils';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocation, useNavigate } from 'react-router-dom';

interface NewMessagePayload {
  type: PubSubMessageType.MESSAGE;
  message: MessageRes;
}

interface ThreadReplyPayload {
  type: PubSubMessageType.THREAD_REPLY;
  rootMessageId: string;
  reply: MessageRes;
  replyCount: number;
  latestReplyAt: string;
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

interface Props {
  channel?: ChannelRes;
  rightPanel: RightPanel;
  onCloseDecisionsPanel: () => void;
  onToggleDecisionsPanel: () => void;
  onOpenThread: (rootMessageId: string) => void;
  onCloseThread: () => void;
}

const addReplyUser = (
  replyUsers: MessageRes['replyUsers'],
  replyUser: MessageRes['user'],
) => {
  if (!replyUser) {
    return replyUsers;
  }
  return [
    replyUser,
    ...(replyUsers || []).filter((user) => user.id !== replyUser.id),
  ].slice(0, 3);
};

export const TextChannelView = ({
  channel,
  rightPanel,
  onCloseDecisionsPanel,
  onToggleDecisionsPanel,
  onOpenThread,
  onCloseThread,
}: Props) => {
  const { inviteToken } = useAuthStore();

  const feedBoxRef = useRef<HTMLDivElement>(null);
  const shouldScrollAfterSendRef = useRef(false);

  const queryClient = useQueryClient();
  const isDesktop = useIsDesktop();
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const { me, isMeSuccess, isAuthError } = useAuthData();
  const { data: capabilities } = useInstanceCapabilitiesQuery();
  const { server, serverId } = useServerData();
  const isDecisionsPanelOpen = rightPanel?.type === 'activeDecisions';
  const threadRootId =
    rightPanel?.type === 'thread' ? rightPanel.rootMessageId : undefined;

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

  const feedQueryKey = useMemo(
    () => [
      'servers',
      serverId,
      'channels',
      channel?.id,
      'feed',
      ...(inviteToken ? ['invite', inviteToken] : []),
    ],
    [channel?.id, inviteToken, serverId],
  );

  const {
    data: feedData,
    fetchNextPage,
    hasNextPage,
    isFetchNextPageError,
    isFetchingNextPage,
  } = useFeedQuery({
    queryKey: feedQueryKey,
    fetchPage: async (cursor, limit) => {
      if (!serverId || !channel?.id) {
        throw new Error('Server ID and channel ID are required');
      }
      const result = await api.getChannelFeed(
        serverId,
        channel.id,
        cursor,
        limit,
      );
      const existingFeed = queryClient
        .getQueryData<FeedQuery>(feedQueryKey)
        ?.pages.flatMap((page) => page.feed);
      return {
        ...result,

        // Keep locally loaded image srcs from being lost on feed refresh.
        feed: preserveFeedImages(existingFeed, result.feed),
      };
    },
    pageSize: MESSAGES_PAGE_SIZE,
    enabled: !!serverId && !!channel?.id && (isMeSuccess || isAuthError),
  });

  const feed = useMemo(
    () => feedData?.pages.flatMap((page) => page.feed) || [],
    [feedData?.pages],
  );

  // Keep a newly sent message in view.
  useEffect(() => {
    if (!shouldScrollAfterSendRef.current) {
      return;
    }

    const frame = requestAnimationFrame(() => {
      shouldScrollAfterSendRef.current = false;
      if (feedBoxRef.current) {
        feedBoxRef.current.scrollTop = 0;
      }
    });

    return () => cancelAnimationFrame(frame);
  }, [feed]);

  const navigationState = location.state as { decisionId?: unknown } | null;
  const navigationDecisionId =
    typeof navigationState?.decisionId === 'string'
      ? navigationState.decisionId
      : undefined;

  const clearFocusedDecisionRequest = useCallback(() => {
    void navigate(location, { replace: true, state: null });
  }, [location, navigate]);

  const videoCallsEnabled = capabilities?.videoCallsEnabled === true;
  const focusedDecisionId = navigationDecisionId;

  // Load more of the feed until the selected decision is found.
  useEffect(() => {
    const isDecisionLoaded = feed.some(
      (item) => item.type === 'poll' && item.id === focusedDecisionId,
    );
    if (
      !focusedDecisionId ||
      isDecisionLoaded ||
      !hasNextPage ||
      isFetchNextPageError ||
      isFetchingNextPage
    ) {
      return;
    }
    void fetchNextPage({ cancelRefetch: false });
  }, [
    feed,
    fetchNextPage,
    focusedDecisionId,
    hasNextPage,
    isFetchNextPageError,
    isFetchingNextPage,
  ]);

  // Listen for new messages
  useSubscription(
    channelPubSubTopic('new-message', serverId, channel?.id, me?.id),
    {
      onMessage: (event) => {
        const { body }: PubSubMessage<NewMessagePayload | ThreadReplyPayload> =
          JSON.parse(event.data);
        if (!body) {
          return;
        }

        if (body.type === PubSubMessageType.THREAD_REPLY) {
          queryClient.setQueryData<FeedQuery>(feedQueryKey, (oldData) => {
            if (!oldData) {
              return oldData;
            }
            return {
              ...oldData,
              pages: oldData.pages.map((page) => ({
                ...page,
                feed: page.feed.map((item) =>
                  item.type === 'message' && item.id === body.rootMessageId
                    ? {
                        ...item,
                        replyCount: body.replyCount,
                        replyUsers: addReplyUser(
                          item.replyUsers,
                          body.reply.user,
                        ),
                        latestReplyAt: body.latestReplyAt,
                      }
                    : item,
                ),
              })),
            };
          });

          const threadQueryKey = getThreadQueryKey(
            serverId,
            channel?.id,
            body.rootMessageId,
            inviteToken,
          );
          queryClient.setQueryData<ThreadQuery>(threadQueryKey, (oldData) => {
            if (!oldData?.pages[0]) {
              return oldData;
            }
            const alreadyExists = oldData.pages.some((page) =>
              page.replies.some((reply) => reply.id === body.reply.id),
            );
            return {
              ...oldData,
              pages: oldData.pages.map((page, index) => ({
                ...page,
                root: {
                  ...page.root,
                  replyCount: body.replyCount,
                  replyUsers: addReplyUser(
                    page.root.replyUsers,
                    body.reply.user,
                  ),
                  latestReplyAt: body.latestReplyAt,
                },
                replies:
                  index === 0 && !alreadyExists
                    ? [...page.replies, body.reply]
                    : page.replies.map((reply) =>
                        reply.id === body.reply.id ? body.reply : reply,
                      ),
              })),
            };
          });
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
                pageParams: [null],
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
                return { ...page, feed: updatedFeed };
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
                return { ...page, feed: updatedFeed };
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
                pageParams: [null],
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
                  return { ...page, feed: updatedFeed };
                }
                const updatedFeed = [newFeedItem, ...page.feed];
                // Sort by createdAt descending (newest first)
                updatedFeed.sort(
                  (a, b) =>
                    new Date(b.createdAt).getTime() -
                    new Date(a.createdAt).getTime(),
                );
                return { ...page, feed: updatedFeed };
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
                pageParams: [null],
              };
            }
            const pages = oldData.pages.map((page, index): FeedQueryPage => {
              const existingIndex = page.feed.findIndex(
                (fi) => fi.type === 'call' && fi.id === newFeedItem.id,
              );
              if (existingIndex !== -1) {
                const updatedFeed = [...page.feed];
                updatedFeed[existingIndex] = newFeedItem;
                return { ...page, feed: updatedFeed };
              }

              if (index === 0) {
                const updatedFeed = [newFeedItem, ...page.feed];
                // Sort by createdAt descending (newest first)
                updatedFeed.sort(
                  (a, b) =>
                    new Date(b.createdAt).getTime() -
                    new Date(a.createdAt).getTime(),
                );
                return { ...page, feed: updatedFeed };
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

  const scrollToBottom = () => {
    if (feedBoxRef.current && feedBoxRef.current.scrollTop >= -200) {
      feedBoxRef.current.scrollTop = 0;
    }
  };

  const desktopRightPanel =
    isDesktop && channel && threadRootId ? (
      <ThreadPanel
        channel={channel}
        rootMessageId={threadRootId}
        onClose={onCloseThread}
      />
    ) : isDesktop && isDecisionsPanelOpen ? (
      <DecisionsPanel isOpen onClose={onCloseDecisionsPanel} />
    ) : null;

  return (
    <div className="fixed top-0 right-0 bottom-0 left-0 flex">
      <ResizablePanelLayout
        panel={isDesktop ? <LeftNavDesktop me={me} /> : null}
        panelType="channelsList"
        resizeHandleLabel={t('actions.resizeChannelsPanel')}
      >
        <ResizablePanelLayout
          panel={desktopRightPanel}
          panelType={threadRootId ? 'thread' : 'activeDecisions'}
          resizeHandleLabel={t('actions.resizeRightPanel')}
        >
          <div
            className={cn(
              'flex h-full min-w-0 flex-1 flex-col',
              !isDesktop && threadRootId && 'hidden',
            )}
          >
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
              feed={feed}
              channel={channel}
              feedBoxRef={feedBoxRef}
              isLastPage={!hasNextPage}
              isJoiningCall={isJoining}
              feedQueryKey={feedQueryKey}
              isLoadingMore={isFetchingNextPage}
              focusedDecisionId={focusedDecisionId}
              focusedDecisionRequestKey={location.key}
              onFocusedDecisionHandled={clearFocusedDecisionRequest}
              onJoinCall={videoCallsEnabled ? joinCall : undefined}
              onLoadMore={() => void fetchNextPage({ cancelRefetch: false })}
              onOpenThread={onOpenThread}
            />

            <MessageForm
              channelId={channel?.id}
              focusOnTyping={!callConfig}
              onSend={() => {
                shouldScrollAfterSendRef.current = true;
              }}
            />
          </div>
        </ResizablePanelLayout>

        {!isDesktop && channel && threadRootId && (
          <ThreadPanel
            channel={channel}
            rootMessageId={threadRootId}
            onClose={onCloseThread}
          />
        )}
      </ResizablePanelLayout>
    </div>
  );
};
