import { api } from '@/client/api-client';
import { Feed } from '@/components/feeds/feed';
import { MessageForm } from '@/components/messages/message-form';
import { MESSAGES_PAGE_SIZE } from '@/constants/message.constants';
import { PubSubMessageType } from '@/constants/pub-sub.constants';
import { preserveFeedImages, preserveFeedItemImages } from '@/lib/feed.utils';
import { callPubSubTopic } from '@/lib/pub-sub.utils';
import { useAuthData } from '@/hooks/use-auth-data';
import { useSubscription } from '@/hooks/use-subscription';
import {
  type FeedItemRes,
  type FeedQuery,
  type FeedQueryPage,
} from '@/types/channel.types';
import { type ChannelRes } from '@/types/channel.types';
import { type MessageRes } from '@/types/message.types';
import { type PubSubMessage } from '@/types/shared.types';
import { useInfiniteQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface NewMessagePayload {
  type: PubSubMessageType.MESSAGE;
  message: MessageRes;
}

interface ImageMessagePayload {
  type: PubSubMessageType.IMAGE;
  isPlaceholder: boolean;
  messageId: string;
  imageId: string;
}

interface Props {
  serverId?: string;
  channel: ChannelRes;
  callId: string;
  readOnly?: boolean;
  initialFeedLimit?: number;
}

export const CallChatPanel = ({
  serverId,
  channel,
  callId,
  readOnly = false,
  initialFeedLimit = MESSAGES_PAGE_SIZE,
}: Props) => {
  const [isLastPage, setIsLastPage] = useState(false);
  const feedBoxRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();
  const { me } = useAuthData();
  const { t } = useTranslation();

  const feedQueryKey = [
    'servers',
    serverId,
    'channels',
    channel.id,
    'calls',
    callId,
    'feed',
  ];

  const { data: feedData, fetchNextPage, isFetchingNextPage } =
    useInfiniteQuery({
      queryKey: feedQueryKey,
      queryFn: async ({ pageParam }) => {
        if (!serverId) {
          throw new Error('Server ID is required');
        }
        const result = await api.getCallFeed(
          serverId,
          channel.id,
          callId,
          pageParam,
          Math.max(MESSAGES_PAGE_SIZE, initialFeedLimit),
        );
        if (result.feed.length === 0) {
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
      getNextPageParam: (_lastPage, pages) =>
        pages.flatMap((page) => page.feed).length,
      initialPageParam: 0,
      enabled: !!serverId,
    });

  useEffect(() => {
    setIsLastPage(false);
  }, [callId]);

  const scrollToBottom = () => {
    if (feedBoxRef.current && feedBoxRef.current.scrollTop >= -200) {
      feedBoxRef.current.scrollTop = 0;
    }
  };

  useSubscription(
    callPubSubTopic('new-message', serverId, channel.id, callId, me?.id),
    {
      onMessage: (event) => {
        const { body }: PubSubMessage<NewMessagePayload | ImageMessagePayload> =
          JSON.parse(event.data);
        if (!body) {
          return;
        }

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
              const existingIndex = page.feed.findIndex(
                (item) =>
                  item.type === 'message' && item.id === messagePayload.id,
              );
              if (existingIndex !== -1) {
                const updatedFeed = [...page.feed];
                const existingMessage = page.feed[existingIndex];
                updatedFeed[existingIndex] = preserveFeedItemImages(
                  existingMessage.type === 'message'
                    ? existingMessage
                    : undefined,
                  incomingFeedItem,
                );
                updatedFeed.sort(
                  (a, b) =>
                    new Date(b.createdAt).getTime() -
                    new Date(a.createdAt).getTime(),
                );
                return { feed: updatedFeed };
              }
              if (index === 0) {
                const updatedFeed = [incomingFeedItem, ...page.feed];
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

        if (body.type === PubSubMessageType.IMAGE) {
          queryClient.setQueryData<FeedQuery>(feedQueryKey, (oldData) => {
            if (!oldData) {
              return { pages: [], pageParams: [] };
            }
            const pages = oldData.pages.map(
              (page): FeedQueryPage => ({
                feed: page.feed.map((item) => {
                  if (item.type !== 'message' || item.id !== body.messageId) {
                    return item;
                  }
                  const images = item.images?.map((image) =>
                    image.id === body.imageId
                      ? { ...image, isPlaceholder: false }
                      : image,
                  );
                  return { ...item, images } as FeedItemRes;
                }),
              }),
            );
            return { pages, pageParams: oldData.pageParams };
          });
        }

        scrollToBottom();
      },
      enabled: !!me && !!serverId,
    },
  );

  return (
    <section
      aria-label={t('calls.headers.inCallChat')}
      className="flex h-full min-h-0 flex-col"
    >
      <div className="border-b border-[--color-border] px-3 py-2.5">
        <h2 className="text-sm font-semibold">
          {t('calls.headers.inCallChat')}
        </h2>
        <p className="text-muted-foreground text-xs">
          {t('calls.descriptions.inCallChat')}
        </p>
      </div>
      <Feed
        channel={channel}
        feedBoxRef={feedBoxRef}
        onLoadMore={fetchNextPage}
        feed={feedData?.pages.flatMap((page) => page.feed) || []}
        feedQueryKey={feedQueryKey}
        isLastPage={isLastPage}
        isLoadingMore={isFetchingNextPage}
        scrollMode={readOnly ? 'natural' : 'bottom-anchored'}
      />
      {!readOnly && (
        <MessageForm
          channelId={channel.id}
          callId={callId}
          showActions={false}
          onSend={scrollToBottom}
        />
      )}
    </section>
  );
};
