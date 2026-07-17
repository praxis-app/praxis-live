import { api } from '@/client/api-client';
import { ForumPostProposal } from '@/components/forum/forum-post-proposal';
import { Message } from '@/components/messages/message';
import { MessageForm } from '@/components/messages/message-form';
import { FormattedText } from '@/components/shared/formatted-text';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { UserAvatar } from '@/components/users/user-avatar';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { handleError } from '@/lib/error.utils';
import { cn } from '@/lib/shared.utils';
import { timeAgo } from '@/lib/time.utils';
import { type ChannelRes } from '@/types/channel.types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { LuArrowLeft } from 'react-icons/lu';
import { MdClose, MdLockOutline } from 'react-icons/md';
import { Link } from 'react-router-dom';

interface Props {
  channel: ChannelRes;
  postId: string;
  isPane?: boolean;
}

export const ForumPostDetail = ({ channel, postId, isPane = false }: Props) => {
  const { t } = useTranslation();
  const { me } = useAuthData();
  const { serverId, serverPath } = useServerData();
  const queryClient = useQueryClient();
  const feedQueryKey = ['servers', serverId, 'channels', channel.id, 'feed'];
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

  const { mutate: closePost, isPending: isClosing } = useMutation({
    mutationFn: () => {
      if (!serverId) throw new Error('Server ID is required');
      return api.closeForumPost(serverId, channel.id, postId);
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: ['servers', serverId, 'channels', channel.id, 'forum'],
      });
    },
    onError: handleError,
  });

  if (!post) {
    return isPane ? (
      <aside className="bg-background min-w-0 flex-1 border-l md:max-w-[720px]" />
    ) : (
      <main className="min-h-0 flex-1" />
    );
  }

  const author = post.user.displayName || post.user.name;
  const isAuthor = me?.id === post.user.id;

  const replyForm = (
    <MessageForm
      channelId={channel.id}
      forumPostId={post.id}
      showActions={false}
      disabled={post.status === 'closed'}
    />
  );

  const detailContent = (
    <div
      className={cn(
        'mx-auto flex w-full max-w-4xl flex-col gap-5 px-4 py-6 md:px-8',
        isPane && 'max-w-none px-5',
      )}
    >
      {(!isPane || (isAuthor && post.status === 'open')) && (
        <div
          className={cn(
            'flex items-center justify-between gap-3',
            isPane && 'justify-end',
          )}
        >
          {!isPane && (
            <Button variant="ghost" asChild>
              <Link to={`${serverPath}/c/${channel.id}`}>
                <LuArrowLeft />
                {t('forums.actions.allPosts')}
              </Link>
            </Button>
          )}
          {isAuthor && post.status === 'open' && (
            <Button
              variant="outline"
              disabled={isClosing}
              onClick={() => closePost()}
            >
              <MdLockOutline />
              {t('forums.actions.closePost')}
            </Button>
          )}
        </div>
      )}

      <article>
        <div className="flex items-start gap-3">
          <UserAvatar
            className="mt-1 shrink-0"
            name={author}
            userId={post.user.id}
            imageId={post.user.profilePicture?.id}
          />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-start justify-between gap-2">
              <div>
                <h1 className="text-xl font-semibold">{post.title}</h1>
                <p className="text-muted-foreground text-sm">
                  {author} · {timeAgo(post.createdAt)}
                </p>
              </div>
              {post.status === 'closed' && (
                <span className="text-muted-foreground flex items-center gap-1 text-sm">
                  <MdLockOutline />
                  {t('forums.labels.closed')}
                </span>
              )}
            </div>
            <FormattedText text={post.body} className="mt-4" />
            <ForumPostProposal
              channel={channel}
              post={post}
              feedQueryKey={feedQueryKey}
            />
          </div>
        </div>
      </article>

      <section className="space-y-4">
        <h2 className="font-medium">
          {t('forums.labels.replies', { count: post.replyCount })}
        </h2>
        <Separator />
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

      {!isPane && replyForm}
    </div>
  );

  if (isPane) {
    return (
      <aside className="bg-background flex min-w-0 flex-1 flex-col border-l md:max-w-[720px]">
        <header className="flex h-[55px] shrink-0 items-center justify-between gap-3 border-b px-4">
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
        <div className="min-h-0 flex-1 overflow-y-auto">{detailContent}</div>
        <div className="shrink-0">{replyForm}</div>
      </aside>
    );
  }

  return (
    <main className="min-h-0 flex-1 overflow-y-auto">{detailContent}</main>
  );
};
