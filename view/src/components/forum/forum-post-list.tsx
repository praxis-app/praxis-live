import { api } from '@/client/api-client';
import { ForumPostForm } from '@/components/forum/forum-post-form';
import { ForumPostListItem } from '@/components/forum/forum-post-list-item';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useAuthData } from '@/hooks/use-auth-data';
import { useServerData } from '@/hooks/use-server-data';
import { useInView } from '@/hooks/use-in-view';
import { useScrollDirection } from '@/hooks/use-scroll-direction';
import { throttle } from '@/lib/shared.utils';
import { type ChannelRes } from '@/types/channel.types';
import { type ForumPostSort, type ForumPostStatus } from '@/types/forum.types';
import { useInfiniteQuery } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MdAdd } from 'react-icons/md';

const FORUM_POSTS_PAGE_SIZE = 20;
const LOAD_MORE_THROTTLE_MS = 1500;
const IN_VIEW_THRESHOLD = 50;

interface Props {
  channel: ChannelRes;
  selectedPostId?: string;
}

export const ForumPostList = ({ channel, selectedPostId }: Props) => {
  const [sort, setSort] = useState<ForumPostSort>('recent');
  const [status, setStatus] = useState<ForumPostStatus | 'all'>('all');
  const [isCreateOpen, setIsCreateOpen] = useState(false);

  const { serverId, serverPath } = useServerData();
  const { isLoggedIn } = useAuthData();

  const { t } = useTranslation();

  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading } =
    useInfiniteQuery({
      queryKey: [
        'servers',
        serverId,
        'channels',
        channel.id,
        'forum',
        'posts',
        sort,
        status,
      ],
      queryFn: ({ pageParam }) => {
        if (!serverId) throw new Error('Server ID is required');
        return api.getForumPosts(
          serverId,
          channel.id,
          sort,
          status === 'all' ? undefined : status,
          pageParam,
          FORUM_POSTS_PAGE_SIZE,
        );
      },
      initialPageParam: 0,
      getNextPageParam: (lastPage, pages) =>
        lastPage.posts.length < FORUM_POSTS_PAGE_SIZE
          ? undefined
          : pages.flatMap((page) => page.posts).length,
      enabled: !!serverId,
    });

  const posts = Array.from(
    new Map(
      data?.pages.flatMap((page) => page.posts).map((post) => [post.id, post]),
    ).values(),
  );

  const listRef = useRef<HTMLElement>(null);
  const listBottomRef = useRef<HTMLDivElement>(null);
  const scrollDirection = useScrollDirection(listRef);

  const fetchNextPageRef = useRef<() => void>(() => undefined);
  fetchNextPageRef.current = () => {
    if (hasNextPage && !isFetchingNextPage) {
      void fetchNextPage();
    }
  };

  const throttledFetchNextPage = useRef(
    throttle(() => fetchNextPageRef.current(), LOAD_MORE_THROTTLE_MS),
  ).current;

  const { setViewed } = useInView(
    listBottomRef,
    `${IN_VIEW_THRESHOLD}px`,
    () => {
      if (scrollDirection !== 'down') return;
      setViewed(false);
      throttledFetchNextPage();
    },
  );

  return (
    <main
      ref={listRef}
      data-testid="forum-post-list"
      className="min-h-0 flex-1 overflow-y-auto"
    >
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-5 px-4 py-6 md:px-8">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap gap-2">
            <Select
              value={sort}
              onValueChange={(value) => setSort(value as ForumPostSort)}
            >
              <SelectTrigger aria-label={t('forums.labels.sort')}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="recent">
                  {t('forums.sort.recent')}
                </SelectItem>
                <SelectItem value="newest">
                  {t('forums.sort.newest')}
                </SelectItem>
              </SelectContent>
            </Select>
            <Select
              value={status}
              onValueChange={(value) =>
                setStatus(value as ForumPostStatus | 'all')
              }
            >
              <SelectTrigger aria-label={t('forums.labels.filter')}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t('forums.filters.all')}</SelectItem>
                <SelectItem value="open">{t('forums.filters.open')}</SelectItem>
                <SelectItem value="closed">
                  {t('forums.filters.closed')}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <Button
            className="shrink-0 disabled:pointer-events-auto disabled:cursor-not-allowed"
            disabled={!isLoggedIn}
            onClick={() => setIsCreateOpen(true)}
          >
            <MdAdd />
            {t('forums.actions.newPost')}
          </Button>
        </div>

        <div className="grid gap-3">
          {posts.map((post) => (
            <ForumPostListItem
              key={post.id}
              post={post}
              postPath={`${serverPath}/c/${channel.id}/posts/${post.id}`}
              isSelected={post.id === selectedPostId}
            />
          ))}
          {!isLoading && !posts.length && (
            <div className="text-muted-foreground rounded-lg border border-dashed p-10 text-center">
              {t('forums.prompts.noPosts')}
            </div>
          )}
          <div ref={listBottomRef} className="h-px" aria-hidden="true" />
        </div>
      </div>

      <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
        <DialogContent className="max-h-[90vh] overflow-y-auto md:max-w-xl">
          <DialogHeader>
            <DialogTitle>{t('forums.actions.createPost')}</DialogTitle>
            <DialogDescription>
              {t('forums.descriptions.createPost')}
            </DialogDescription>
          </DialogHeader>
          <ForumPostForm
            channel={channel}
            onSuccess={() => setIsCreateOpen(false)}
          />
        </DialogContent>
      </Dialog>
    </main>
  );
};
