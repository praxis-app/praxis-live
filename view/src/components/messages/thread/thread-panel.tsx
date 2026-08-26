import { api } from '@/client/api-client';
import { Message } from '@/components/messages/message';
import { MessageForm } from '@/components/messages/message-form';
import { getThreadQueryKey } from '@/components/messages/thread/thread-query.utils';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { useAuthData } from '@/hooks/use-auth-data';
import { useIsDesktop } from '@/hooks/use-is-desktop';
import { useServerData } from '@/hooks/use-server-data';
import { preserveMessageImages } from '@/lib/feed.utils';
import { cn } from '@/lib/shared.utils';
import { type ChannelRes } from '@/types/channel.types';
import { type MessageRes, type ThreadQuery } from '@/types/message.types';
import { useInfiniteQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { MdArrowBack, MdClose, MdErrorOutline } from 'react-icons/md';

const THREAD_PAGE_SIZE = 50;

interface Props {
  channel: ChannelRes;
  rootMessageId: string;
  onClose: () => void;
}

const mergeMessageImages = (
  existing: MessageRes | undefined,
  incoming: MessageRes,
): MessageRes => ({
  ...incoming,
  images: preserveMessageImages(existing?.images, incoming.images),
});

export const ThreadPanel = ({ channel, rootMessageId, onClose }: Props) => {
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const shouldScrollAfterReplyRef = useRef(false);
  const previousReplyCountRef = useRef<number | undefined>(undefined);

  const { inviteToken, me } = useAuthData();
  const { serverId } = useServerData();
  const isDesktop = useIsDesktop();
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  const queryKey = getThreadQueryKey(
    serverId,
    channel.id,
    rootMessageId,
    inviteToken,
  );
  const threadQuery = useInfiniteQuery({
    queryKey,
    queryFn: async ({ pageParam }) => {
      if (!serverId) {
        throw new Error('Server ID is required');
      }
      const result = await api.getThreadReplies(
        serverId,
        channel.id,
        rootMessageId,
        pageParam || undefined,
        THREAD_PAGE_SIZE,
      );
      const existing = queryClient.getQueryData<ThreadQuery>(queryKey);
      const existingMessages = new Map(
        existing?.pages
          .flatMap((page) => [page.root, ...page.replies])
          .map((message) => [message.id, message]),
      );
      return {
        ...result,
        root: mergeMessageImages(
          existingMessages.get(result.root.id),
          result.root,
        ),
        replies: result.replies.map((reply) =>
          mergeMessageImages(existingMessages.get(reply.id), reply),
        ),
      };
    },
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? lastPage.nextCursor : undefined,
    enabled: !!serverId,
    refetchOnMount: 'always',
  });

  const root = threadQuery.data?.pages[0]?.root;
  const replies = useMemo(() => {
    const chronologicalReplies = [...(threadQuery.data?.pages || [])]
      .reverse()
      .flatMap((page) => page.replies);
    return [
      ...new Map(
        chronologicalReplies.map((reply) => [reply.id, reply]),
      ).values(),
    ];
  }, [threadQuery.data?.pages]);

  useEffect(() => {
    previousReplyCountRef.current = undefined;
    shouldScrollAfterReplyRef.current = false;
  }, [rootMessageId]);

  useEffect(() => {
    const previousReplyCount = previousReplyCountRef.current;
    previousReplyCountRef.current = replies.length;
    const receivedReply =
      previousReplyCount !== undefined && replies.length > previousReplyCount;
    if (!receivedReply && !shouldScrollAfterReplyRef.current) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      shouldScrollAfterReplyRef.current = false;
      const container = scrollContainerRef.current;
      container?.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
    });
    return () => cancelAnimationFrame(frame);
  }, [replies.length]);

  return (
    <aside
      data-testid="thread-panel"
      aria-label={t('messages.threads.title')}
      className={cn(
        'bg-background flex min-h-0 min-w-0 flex-1 flex-col',
        isDesktop && 'max-w-120 border-l',
      )}
    >
      <header className="flex h-13.75 shrink-0 items-center gap-2 border-b px-3">
        {!isDesktop && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={t('messages.threads.back')}
            onClick={onClose}
          >
            <MdArrowBack className="size-5" />
          </Button>
        )}
        <div className="min-w-0">
          <h2 className="font-semibold">{t('messages.threads.title')}</h2>
          <p className="text-muted-foreground truncate text-xs">
            {t('messages.threads.channel', { channel: channel.name })}
          </p>
        </div>
        {isDesktop && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="ml-auto"
            aria-label={t('messages.threads.close')}
            onClick={onClose}
          >
            <MdClose className="size-5" />
          </Button>
        )}
      </header>

      <div ref={scrollContainerRef} className="min-h-0 flex-1 overflow-y-auto">
        {threadQuery.isLoading && (
          <div className="text-muted-foreground flex h-full items-center justify-center text-sm">
            {t('messages.threads.loading')}
          </div>
        )}

        {threadQuery.isError && (
          <div className="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
            <MdErrorOutline className="text-muted-foreground size-8" />
            <div>
              <p className="font-medium">
                {t('messages.threads.notFoundTitle')}
              </p>
              <p className="text-muted-foreground mt-1 text-sm">
                {t('messages.threads.notFoundDescription')}
              </p>
            </div>
          </div>
        )}

        {root && (
          <div className="flex min-h-full flex-col px-4 pt-5 pb-4">
            <Message
              message={root}
              me={me}
              serverId={serverId}
              channelId={channel.id}
            />

            <div
              className="text-muted-foreground my-5 flex items-center gap-3 text-xs font-medium"
              role="separator"
              aria-label={t('messages.labels.replyCount', {
                count: root.replyCount,
              })}
            >
              <span>
                {t('messages.labels.replyCount', {
                  count: root.replyCount,
                })}
              </span>
              <Separator className="flex-1" />
            </div>

            {threadQuery.hasNextPage && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="text-muted-foreground mb-4 self-center"
                disabled={threadQuery.isFetchingNextPage}
                onClick={() => void threadQuery.fetchNextPage()}
              >
                {threadQuery.isFetchingNextPage
                  ? t('messages.threads.loadingOlder')
                  : t('messages.threads.loadOlder')}
              </Button>
            )}

            <div className="flex flex-1 flex-col gap-4" aria-live="polite">
              {replies.map((reply) => (
                <Message
                  key={reply.id}
                  message={reply}
                  me={me}
                  serverId={serverId}
                  channelId={channel.id}
                />
              ))}
              {!replies.length && (
                <p className="text-muted-foreground flex min-h-24 flex-1 items-center justify-center text-center text-sm">
                  {t('messages.threads.empty')}
                </p>
              )}
            </div>
          </div>
        )}
      </div>

      {root && (
        <div className="shrink-0">
          <MessageForm
            key={rootMessageId}
            channelId={channel.id}
            threadRootId={rootMessageId}
            showActions={false}
            focusOnTyping={false}
            onSend={() => {
              shouldScrollAfterReplyRef.current = true;
            }}
          />
        </div>
      )}
    </aside>
  );
};
