import { expect, type Locator, type Page } from '@playwright/test';

export const notificationBell = (page: Page) =>
  page.getByTestId('notification-bell');

export const notificationCount = (page: Page) =>
  page.getByTestId('notification-count');

export async function openNotifications(page: Page) {
  await notificationBell(page).click();
  return page.getByRole('region', { name: 'Notifications', exact: true });
}

export async function expectUnreadNotifications(page: Page, count: number) {
  if (count === 0) {
    await expect(notificationCount(page)).toHaveCount(0);
    return;
  }
  await expect(notificationCount(page)).toHaveText(String(count));
}

export const notificationItem = (inbox: Locator, text: string | RegExp) =>
  inbox.getByTestId('notification-item').filter({ hasText: text });

export const channelListItem = (page: Page, channelName: string) =>
  page.getByTestId('channel-list-item').filter({ hasText: channelName });

export const channelUnreadIndicator = (page: Page, channelName: string) =>
  channelListItem(page, channelName).getByTestId('channel-unread-indicator');
