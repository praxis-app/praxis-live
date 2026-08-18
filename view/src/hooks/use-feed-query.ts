import {
  hashKey,
  type QueryKey,
  useInfiniteQuery,
  useQueryClient,
} from '@tanstack/react-query';
import { useEffect, useEffectEvent, useMemo, useRef } from 'react';
import {
  type FeedItemRes,
  type FeedPageRes,
  type FeedQuery,
} from '@/types/channel.types';

interface FeedCursorParams {
  before?: string;
  after?: string;
}

interface Options {
  enabled: boolean;
  pageSize: number;
  queryKey: QueryKey;
  fetchPage: (cursor: FeedCursorParams, limit: number) => Promise<FeedPageRes>;
}

const feedItemKey = (item: FeedItemRes) => `${item.type}:${item.id}`;

const sortNewestFirst = (left: FeedItemRes, right: FeedItemRes) =>
  new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime() ||
  right.id.localeCompare(left.id);

export const useFeedQuery = ({
  enabled,
  pageSize,
  queryKey,
  fetchPage,
}: Options) => {
  const queryClient = useQueryClient();
  const queryHash = hashKey(queryKey);
  const hadCachedData = useMemo(
    () => !!queryClient.getQueryData<FeedQuery>(queryKey),
    [queryClient, queryKey],
  );
  const syncedQueryHashes = useRef(new Set<string>());

  const query = useInfiniteQuery({
    queryKey,
    queryFn: ({ pageParam }) =>
      fetchPage(pageParam ? { before: pageParam } : {}, pageSize),
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? lastPage.nextCursor : undefined,
    initialPageParam: null as string | null,
    enabled,
    staleTime: Infinity,
  });

  const syncNewerItems = useEffectEvent(async () => {
    const cachedFeed = queryClient.getQueryData<FeedQuery>(queryKey);
    const newestCursor = cachedFeed?.pages[0]?.startCursor;
    if (!cachedFeed || !newestCursor) {
      await query.refetch();
      return;
    }

    const newItems: FeedItemRes[] = [];
    let after = newestCursor;
    let latestCursor = newestCursor;
    let hasMore = true;

    while (hasMore) {
      const page = await fetchPage({ after }, pageSize);
      newItems.push(...page.feed);
      if (!page.nextCursor) break;
      latestCursor = page.nextCursor;
      after = page.nextCursor;
      hasMore = page.hasMore;
    }

    if (newItems.length === 0) return;

    queryClient.setQueryData<FeedQuery>(queryKey, (currentFeed) => {
      if (!currentFeed?.pages[0]) return currentFeed;

      const firstPage = currentFeed.pages[0];
      const mergedItems = [...newItems, ...firstPage.feed];
      const uniqueItems = [
        ...new Map(
          mergedItems.map((item) => [feedItemKey(item), item]),
        ).values(),
      ].sort(sortNewestFirst);

      return {
        ...currentFeed,
        pages: [
          { ...firstPage, feed: uniqueItems, startCursor: latestCursor },
          ...currentFeed.pages.slice(1),
        ],
      };
    });
  });

  useEffect(() => {
    if (
      !enabled ||
      !hadCachedData ||
      syncedQueryHashes.current.has(queryHash)
    ) {
      return;
    }
    syncedQueryHashes.current.add(queryHash);
    void syncNewerItems();
  }, [enabled, hadCachedData, queryHash]);

  return query;
};
