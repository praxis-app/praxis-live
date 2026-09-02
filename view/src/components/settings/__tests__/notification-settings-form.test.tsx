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

const toggle = (name: string) =>
  screen.getByRole('switch', { name: `settings.names.${name}` });

const saveButton = () => screen.getByRole('button', { name: 'actions.save' });

describe('NotificationSettingsForm', () => {
  it('should show a switch per notification category reflecting stored settings', () => {
    renderForm();

    expect(screen.getAllByRole('switch')).toHaveLength(4);
    expect(toggle('messageNotificationsEnabled')).toBeChecked();
    expect(toggle('replyNotificationsEnabled')).not.toBeChecked();
  });

  it('should keep saving disabled until a setting changes', async () => {
    renderForm();

    expect(saveButton()).toBeDisabled();

    fireEvent.click(toggle('messageNotificationsEnabled'));

    await waitFor(() => {
      expect(saveButton()).toBeEnabled();
    });
    expect(updateUserConfigMock).not.toHaveBeenCalled();
  });

  it('should only send the toggled setting when saving', async () => {
    updateUserConfigMock.mockResolvedValue({
      userConfig: { ...userConfig, messageNotificationsEnabled: false },
    });
    renderForm();

    fireEvent.click(toggle('messageNotificationsEnabled'));
    await waitFor(() => {
      expect(saveButton()).toBeEnabled();
    });
    fireEvent.click(saveButton());

    await waitFor(() => {
      expect(updateUserConfigMock).toHaveBeenCalledWith({
        messageNotificationsEnabled: false,
      });
    });
  });

  it('should cache the saved settings and disable saving again', async () => {
    updateUserConfigMock.mockResolvedValue({
      userConfig: { ...userConfig, replyNotificationsEnabled: true },
    });
    const queryClient = renderForm();

    fireEvent.click(toggle('replyNotificationsEnabled'));
    await waitFor(() => {
      expect(saveButton()).toBeEnabled();
    });
    fireEvent.click(saveButton());

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
    await waitFor(() => {
      expect(saveButton()).toBeDisabled();
    });
  });
});
