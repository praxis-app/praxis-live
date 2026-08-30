import { api } from '@/client/api-client';
import { ThreadPanel } from '@/components/messages/thread/thread-panel';
import { type MessageRes } from '@/types/message.types';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen } from '@testing-library/react';
import { type ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/client/api-client', () => ({
  api: {
    getMessageThreadReplies: vi.fn(),
    getPollThreadReplies: vi.fn(),
  },
}));
vi.mock('@/hooks/use-auth-data', () => ({
  useAuthData: () => ({ inviteToken: null, me: undefined }),
}));
vi.mock('@/hooks/use-server-data', () => ({
  useServerData: () => ({
    server: { id: 'server-1', slug: 'praxis' },
    serverId: 'server-1',
  }),
}));
vi.mock('@/hooks/use-is-desktop', () => ({ useIsDesktop: () => true }));
vi.mock('@/hooks/use-scroll-to-bottom', () => ({
  useScrollToBottom: () => ({
    containerRef: { current: null },
    scrollToBottom: vi.fn(),
    handleContentLoad: vi.fn(),
  }),
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock('react-router-dom', () => ({ useNavigate: () => vi.fn() }));
vi.mock('@/components/messages/message', () => ({
  Message: ({ message }: { message: MessageRes }) => <p>{message.body}</p>,
}));
vi.mock('@/components/messages/message-form', () => ({
  MessageForm: () => <div data-testid="message-form" />,
}));
vi.mock('@/components/polls/inline-poll', () => ({ InlinePoll: () => null }));
vi.mock('@/components/polls/proposals/inline-proposal/inline-proposal', () => ({
  InlineProposal: () => null,
}));

const message = (id: string, body: string, replyCount = 0): MessageRes => ({
  id,
  body,
  user: null,
  userId: null,
  botId: null,
  bot: null,
  replyCount,
  latestReplyAt: null,
  createdAt: '2026-08-30T12:00:00Z',
});

describe('ThreadPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it.each([
    [
      'the tab becomes visible',
      () => document.dispatchEvent(new Event('visibilitychange')),
    ],
    [
      'the browser regains focus',
      () => window.dispatchEvent(new Event('focus')),
    ],
    [
      'the browser comes online',
      () => window.dispatchEvent(new Event('online')),
    ],
  ])('shows a missed reply when %s', async (_description, resume) => {
    const root = message('root-1', 'Root message');
    const missedReply = message('reply-1', 'Missed reply');
    vi.mocked(api.getMessageThreadReplies)
      .mockResolvedValueOnce({
        root,
        replies: [],
        startCursor: null,
        nextCursor: null,
        hasMore: false,
      })
      .mockResolvedValueOnce({
        root: { ...root, replyCount: 1 },
        replies: [missedReply],
        startCursor: null,
        nextCursor: null,
        hasMore: false,
      });
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    );

    render(
      <ThreadPanel
        channel={{ id: 'channel-1', name: 'general', description: null }}
        thread={{ rootKind: 'message', rootId: root.id }}
        feedQueryKey={['feed', 'channel-1']}
        onClose={vi.fn()}
      />,
      { wrapper },
    );

    expect(await screen.findByText('Root message')).toBeInTheDocument();
    expect(screen.queryByText('Missed reply')).not.toBeInTheDocument();

    act(resume);

    expect(await screen.findByText('Missed reply')).toBeInTheDocument();
  });
});
