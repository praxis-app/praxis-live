import { api } from '@/client/api-client';
import { ForumPostMenu } from '@/components/forum/forum-post-menu';
import { ForumProposalPresentation } from '@/components/forum/forum-proposal-presentation';
import { Message } from '@/components/messages/message';
import { MessageForm } from '@/components/messages/message-form';
import { ProposalSettingsDialog } from '@/components/polls/proposals/proposal-settings-dialog';
import { FormattedText } from '@/components/shared/formatted-text';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { UserAvatar } from '@/components/users/user-avatar';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { cn } from '@/lib/shared.utils';
import { timeAgo } from '@/lib/time.utils';
import { type ChannelRes } from '@/types/channel.types';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MdClose, MdLockOutline } from 'react-icons/md';
import { Link } from 'react-router-dom';

interface Props {
  channel: ChannelRes;
  postId: string;
  isPane?: boolean;
}

export const ForumPostDetail = ({ channel, postId, isPane = false }: Props) => {
  const [isProposalSettingsOpen, setIsProposalSettingsOpen] = useState(false);
  const { t } = useTranslation();
  const { me } = useAuthData();
  const { serverId, serverPath } = useServerData();

  const postQueryKey = [
    'servers',
    serverId,
    'channels',
    channel.id,
    'forum',
    'posts',
    postId,
  ];

  const { data } = useQuery({
    queryKey: postQueryKey,
    queryFn: () => {
      if (!serverId) throw new Error('Server ID is required');
      return api.getForumPost(serverId, channel.id, postId);
    },
    enabled: !!serverId,
  });
  const post = data?.post;

  const scrollContainerRef = useRef<HTMLElement | null>(null);
  const shouldScrollAfterReplyRef = useRef(false);
  const wasNearBottomRef = useRef(true);
  const previousReplyCountRef = useRef<number | undefined>(undefined);
  const replyCount = post?.replies.length;

  useEffect(() => {
    setIsProposalSettingsOpen(false);
    shouldScrollAfterReplyRef.current = false;
    wasNearBottomRef.current = true;
    previousReplyCountRef.current = undefined;
  }, [postId]);

  useEffect(() => {
    const previousReplyCount = previousReplyCountRef.current;
    previousReplyCountRef.current = replyCount;
    if (replyCount === undefined) return;

    const receivedNewReply =
      previousReplyCount !== undefined && replyCount > previousReplyCount;
    if (
      !shouldScrollAfterReplyRef.current &&
      !(receivedNewReply && wasNearBottomRef.current)
    ) {
      return;
    }

    const frame = requestAnimationFrame(() => {
      shouldScrollAfterReplyRef.current = false;
      const container = scrollContainerRef.current;
      container?.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
      wasNearBottomRef.current = true;
    });

    return () => cancelAnimationFrame(frame);
  }, [replyCount]);

  const setScrollContainer = useCallback((element: HTMLElement | null) => {
    scrollContainerRef.current = element;
    if (element) {
      wasNearBottomRef.current =
        element.scrollHeight - element.scrollTop - element.clientHeight <= 200;
    }
  }, []);

  const handleScroll = useCallback(() => {
    const container = scrollContainerRef.current;
    if (container) {
      wasNearBottomRef.current =
        container.scrollHeight - container.scrollTop - container.clientHeight <=
        200;
    }
  }, []);

  if (!post) {
    return isPane ? (
      <aside className="bg-background min-w-0 flex-1 border-l md:max-w-180" />
    ) : (
      <main className="min-h-0 flex-1" />
    );
  }

  const author = post.user.displayName || post.user.name;
  const isAuthor = me?.id === post.user.id;

  const showForumPostMenu =
    post.proposal || (isAuthor && (post.status === 'open' || !post.proposal));

  const replyForm = (
    <MessageForm
      channelId={channel.id}
      forumPostId={post.id}
      showActions={false}
      disabled={post.status === 'closed'}
      onSend={() => {
        shouldScrollAfterReplyRef.current = true;
      }}
    />
  );

  const detailContent = (
    <div
      className={cn(
        'mx-auto flex w-full max-w-4xl flex-col gap-5 px-3 pt-6 md:px-5 md:py-6',
        isPane && 'max-w-none px-4',
      )}
    >
      <article>
        <div className="flex items-start gap-3">
          <UserAvatar
            className="mt-1 shrink-0"
            name={author}
            userId={post.user.id}
            imageId={post.user.profilePicture?.id}
          />
          <div className="min-w-0 flex-1">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0 flex-1">
                <h1 className="text-xl font-semibold">{post.title}</h1>
                <p className="text-muted-foreground text-sm">
                  {author} · {timeAgo(post.createdAt)}
                </p>
              </div>
              <div className="flex shrink-0 items-center gap-1">
                {post.status === 'closed' && (
                  <span className="text-muted-foreground flex items-center gap-1 text-sm">
                    <MdLockOutline />
                    {t('forums.labels.closed')}
                  </span>
                )}
                {showForumPostMenu && (
                  <ForumPostMenu
                    channel={channel}
                    post={post}
                    isAuthor={isAuthor}
                    onViewProposalSettings={() =>
                      setIsProposalSettingsOpen(true)
                    }
                  />
                )}
              </div>
            </div>
            <FormattedText text={post.body} className="mt-4" />
            {post.proposal && (
              <ForumProposalPresentation
                channel={channel}
                proposal={post.proposal}
                postQueryKey={postQueryKey}
                me={me}
              />
            )}
          </div>
        </div>
      </article>

      {post.proposal && (
        <ProposalSettingsDialog
          actionType={post.proposal.action?.actionType}
          config={post.proposal.config}
          open={isProposalSettingsOpen}
          onOpenChange={setIsProposalSettingsOpen}
        />
      )}

      <section className="space-y-4">
        <div
          className="text-muted-foreground flex items-center gap-3 text-xs font-medium"
          role="separator"
          aria-label={t('forums.labels.discussion')}
        >
          <Separator className="flex-1" />
          <span>{t('forums.labels.discussion')}</span>
          <Separator className="flex-1" />
        </div>
        {post.replies.map((reply) => (
          <Message
            key={reply.id}
            message={reply}
            me={me}
            serverId={serverId}
            channelId={channel.id}
          />
        ))}
        {!post.replies.length && (
          <p className="text-muted-foreground text-sm">
            {t('forums.prompts.noReplies')}
          </p>
        )}
      </section>

      {!isPane && <div className="-mx-3">{replyForm}</div>}
    </div>
  );

  if (isPane) {
    return (
      <aside className="bg-background flex min-w-0 flex-1 flex-col border-l md:max-w-180">
        <header className="flex h-13.75 shrink-0 items-center justify-between gap-3 border-b px-4">
          <h2 className="truncate font-medium">{post.title}</h2>
          <Button variant="ghost" size="icon" asChild>
            <Link
              to={`${serverPath}/c/${channel.id}`}
              aria-label={t('forums.actions.closePostPane')}
            >
              <MdClose className="size-6" />
            </Link>
          </Button>
        </header>
        <div
          ref={setScrollContainer}
          onScroll={handleScroll}
          className="min-h-0 flex-1 overflow-y-auto"
        >
          {detailContent}
        </div>
        <div className="shrink-0">{replyForm}</div>
      </aside>
    );
  }

  return (
    <main
      ref={setScrollContainer}
      onScroll={handleScroll}
      className="min-h-0 flex-1 overflow-y-auto"
    >
      {detailContent}
    </main>
  );
};
