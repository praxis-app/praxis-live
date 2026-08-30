import { useFeedQuery } from '@/hooks/use-feed-query';
import { type FeedItemRes, type FeedPageRes } from '@/types/channel.types';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { type ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

const message = (id: string, createdAt: string): FeedItemRes => ({
  type: 'message',
  id,
  body: `Message ${id}`,
  user: null,
  userId: null,
  botId: null,
  bot: null,
  replyCount: 0,
  latestReplyAt: null,
  createdAt,
});

describe('useFeedQuery', () => {
  it('catches up on items missed while the tab was not visible', async () => {
    const initialMessage = message('initial', '2026-08-30T12:00:00Z');
    const missedMessage = message('missed', '2026-08-30T12:05:00Z');
    const initialPage: FeedPageRes = {
      feed: [initialMessage],
      startCursor: 'initial-cursor',
      nextCursor: null,
      hasMore: false,
    };
    const catchUpPage: FeedPageRes = {
      feed: [missedMessage],
      startCursor: 'missed-cursor',
      nextCursor: null,
      hasMore: false,
    };
    const fetchPage = vi.fn(
      async (cursor: { before?: string; after?: string }) =>
        cursor.after ? catchUpPage : initialPage,
    );
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const queryKey = ['feed', 'channel-1'] as const;
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    );

    const { result } = renderHook(
      () => {
        const query = useFeedQuery({
          enabled: true,
          pageSize: 20,
          queryKey,
          fetchPage,
        });
        return {
          ...query,
          visibleFeed: query.data?.pages[0].feed,
        };
      },
      { wrapper },
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    await waitFor(() => {
      expect(fetchPage).toHaveBeenLastCalledWith(
        { after: 'initial-cursor' },
        20,
      );
    });
    await waitFor(() => {
      expect(result.current.visibleFeed).toEqual([
        missedMessage,
        initialMessage,
      ]);
    });
  });
});
