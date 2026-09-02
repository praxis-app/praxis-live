import { NotificationSettingsForm } from '@/components/settings/notification-settings-form';
import { type UserConfigRes } from '@/types/user-config.types';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

const updateUserConfigMock = vi.fn();

vi.mock('@/client/api-client', () => ({
  api: {
    updateUserConfig: (...args: unknown[]) => updateUserConfigMock(...args),
  },
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const userConfig: UserConfigRes = {
  messageNotificationsEnabled: true,
  replyNotificationsEnabled: false,
  proposalNotificationsEnabled: true,
  roleNotificationsEnabled: true,
  updatedAt: '2026-09-02T00:00:00Z',
};

const renderForm = (config: UserConfigRes = userConfig) => {
  const queryClient = new QueryClient({
    defaultOptions: { mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <NotificationSettingsForm userConfig={config} />
    </QueryClientProvider>,
  );
  return queryClient;
};

describe('NotificationSettingsForm', () => {
  it('should show a switch per notification category reflecting stored settings', () => {
    renderForm();

    const switches = screen.getAllByRole('switch');
    expect(switches).toHaveLength(4);
    expect(
      screen.getByRole('switch', {
        name: 'settings.names.messageNotificationsEnabled',
      }),
    ).toBeChecked();
    expect(
      screen.getByRole('switch', {
        name: 'settings.names.replyNotificationsEnabled',
      }),
    ).not.toBeChecked();
  });

  it('should only send the toggled setting when turning one off', async () => {
    updateUserConfigMock.mockResolvedValue({
      userConfig: { ...userConfig, messageNotificationsEnabled: false },
    });
    renderForm();

    fireEvent.click(
      screen.getByRole('switch', {
        name: 'settings.names.messageNotificationsEnabled',
      }),
    );

    await waitFor(() => {
      expect(updateUserConfigMock).toHaveBeenCalledWith({
        messageNotificationsEnabled: false,
      });
    });
  });

  it('should turn a disabled setting back on', async () => {
    updateUserConfigMock.mockResolvedValue({
      userConfig: { ...userConfig, replyNotificationsEnabled: true },
    });
    const queryClient = renderForm();

    fireEvent.click(
      screen.getByRole('switch', {
        name: 'settings.names.replyNotificationsEnabled',
      }),
    );

    await waitFor(() => {
      expect(updateUserConfigMock).toHaveBeenCalledWith({
        replyNotificationsEnabled: true,
      });
    });
    await waitFor(() => {
      expect(queryClient.getQueryData(['users', 'me', 'configs'])).toEqual({
        userConfig: { ...userConfig, replyNotificationsEnabled: true },
      });
    });
  });
});
