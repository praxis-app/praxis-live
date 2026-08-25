import { api } from '@/client/api-client';
import { AuthWrapper } from '@/components/auth/auth-wrapper';
import { useServerData } from '@/hooks/use-server-data';
import { useAuthStore } from '@/store/auth.store';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, waitFor } from '@testing-library/react';
import { type ReactNode } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/client/api-client', () => ({
  api: {
    getServerBySlug: vi.fn(),
    setCurrentServer: vi.fn(),
    getCurrentUser: vi.fn(),
    getUserImage: vi.fn(),
    getDefaultServer: vi.fn(),
    getServerByInviteToken: vi.fn(),
    isFirstUser: vi.fn(),
  },
}));

const SERVER_SLUG = 'praxis';
const mockServer = {
  id: 'server-1',
  name: 'praxis',
  slug: SERVER_SLUG,
  generalChannelId: 'channel-1',
};

// Every component that reads server data mounts its own copy of the hook.
const Consumer = () => {
  useServerData();
  return null;
};

const renderConsumers = (count: number) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[`/s/${SERVER_SLUG}`]}>
        <Routes>
          <Route
            path="/s/:serverSlug"
            element={<AuthWrapper>{children}</AuthWrapper>}
          />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>
  );

  return render(
    <>
      {Array.from({ length: count }, (_, index) => (
        <Consumer key={index} />
      ))}
    </>,
    { wrapper },
  );
};

describe('useServerData', () => {
  beforeEach(() => {
    useAuthStore.setState({ isLoggedIn: true, accessToken: 'token' });
    vi.mocked(api.getServerBySlug).mockResolvedValue({
      server: mockServer,
    } as never);
    vi.mocked(api.getCurrentUser).mockResolvedValue({
      user: { id: 'user-1', anonymous: false, permissions: {} },
    } as never);
    vi.mocked(api.setCurrentServer).mockResolvedValue(undefined as never);
  });

  it('records the visit once no matter how many consumers are mounted', async () => {
    renderConsumers(25);

    await waitFor(() => {
      expect(api.getServerBySlug).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(api.setCurrentServer).toHaveBeenCalled();
    });

    expect(api.setCurrentServer).toHaveBeenCalledTimes(1);
    expect(api.setCurrentServer).toHaveBeenCalledWith(mockServer.id);
  });

  it('does not re-record the visit on re-render', async () => {
    const { rerender } = renderConsumers(3);

    await waitFor(() => {
      expect(api.setCurrentServer).toHaveBeenCalled();
    });
    const callsAfterMount = vi.mocked(api.setCurrentServer).mock.calls.length;

    rerender(
      <>
        <Consumer />
        <Consumer />
        <Consumer />
      </>,
    );

    expect(api.setCurrentServer).toHaveBeenCalledTimes(callsAfterMount);
  });
});
