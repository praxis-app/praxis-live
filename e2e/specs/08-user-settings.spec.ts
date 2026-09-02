import { expect, test, type Page } from '@playwright/test';
import {
  authorizationHeaders,
  createAuthenticatedUser,
  getOrCreateInstanceAdmin,
  signUpViaApi,
} from '../lib/auth';
import { createTestUser } from '../lib/data';
import {
  expectUnreadNotifications,
  notificationItem,
  openNotifications,
} from '../lib/notifications';
import { getDefaultServer } from '../lib/servers';

const settingsPath = '/users/settings';

const newMessagesSwitch = (page: Page) =>
  page.getByRole('switch', { name: 'New messages' });

const saveNotificationSettings = async (page: Page) => {
  const save = page.getByRole('button', { name: 'Save', exact: true });
  await save.click();
  await expect(save).toBeDisabled();
};

const expectUnifiedSettingsPage = async (page: Page) => {
  await expect(
    page.getByRole('heading', { name: 'Profile', exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel('Bio')).toBeVisible();
  await expect(
    page.getByText('Profile picture', { exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Cover photo', { exact: true })).toBeVisible();

  await expect(
    page.getByRole('heading', { name: 'Notifications', exact: true }),
  ).toBeVisible();
  await expect(page.getByRole('switch')).toHaveCount(4);
};

test.beforeAll(async ({ request }) => {
  await getOrCreateInstanceAdmin(request);
});

test('reaches unified user settings from the desktop nav', async ({
  context,
  page,
  request,
}) => {
  const user = await createAuthenticatedUser(
    request,
    context,
    createTestUser('settings-desktop-nav'),
  );
  const server = await getDefaultServer(request, user);

  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);

  await page.getByRole('button', { name: 'User Settings' }).click();

  await expect(page).toHaveURL(new RegExp(`${settingsPath}$`));
  await expectUnifiedSettingsPage(page);
});

test('reaches unified user settings from the mobile nav', async ({
  context,
  page,
  request,
}) => {
  const user = await createAuthenticatedUser(
    request,
    context,
    createTestUser('settings-mobile-nav'),
  );
  const server = await getDefaultServer(request, user);

  await page.setViewportSize({ width: 390, height: 760 });
  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);

  await page.getByRole('button', { name: 'Open navigation' }).click();

  const navSheet = page.getByRole('dialog');
  await navSheet.getByTitle(user.user.name).first().click();
  await page.getByRole('menuitem', { name: 'Settings' }).click();

  await expect(page).toHaveURL(new RegExp(`${settingsPath}$`));
  await expectUnifiedSettingsPage(page);
});

test('reaches notification settings from the notification inbox', async ({
  context,
  page,
  request,
}) => {
  const user = await createAuthenticatedUser(
    request,
    context,
    createTestUser('settings-inbox-nav'),
  );
  const server = await getDefaultServer(request, user);

  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);

  const inbox = await openNotifications(page);
  await inbox.getByRole('button', { name: 'Notification settings' }).click();

  await expect(page).toHaveURL(new RegExp(`${settingsPath}$`));
  await expectUnifiedSettingsPage(page);
});

test('turning off new message notifications silences them until it is turned back on', async ({
  context,
  page,
  request,
}) => {
  const recipient = await createAuthenticatedUser(
    request,
    context,
    createTestUser('settings-recipient'),
  );
  const actor = await signUpViaApi(request, createTestUser('settings-actor'));
  const server = await getDefaultServer(request, recipient);

  const postMessage = async (body: string) => {
    const response = await request.post(
      `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
      { headers: authorizationHeaders(actor), data: { body } },
    );
    await expect(response).toBeOK();
  };

  await page.goto(settingsPath);
  await expect(newMessagesSwitch(page)).toBeVisible();
  await expect(newMessagesSwitch(page)).toBeChecked();

  await newMessagesSwitch(page).click();
  await expect(newMessagesSwitch(page)).not.toBeChecked();
  await saveNotificationSettings(page);

  // The setting is stored server side, not just in the open page.
  await page.reload();
  await expect(newMessagesSwitch(page)).not.toBeChecked();

  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  await expect(page.getByTestId('notification-bell')).toBeVisible();

  const silencedBody = `Silenced message ${recipient.user.suffix}`;
  await postMessage(silencedBody);
  await expect(page.getByText(silencedBody)).toBeVisible();
  await expectUnreadNotifications(page, 0);

  await page.goto(settingsPath);
  await newMessagesSwitch(page).click();
  await expect(newMessagesSwitch(page)).toBeChecked();
  await saveNotificationSettings(page);

  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  const heardBody = `Delivered message ${recipient.user.suffix}`;
  await postMessage(heardBody);
  await expectUnreadNotifications(page, 1);

  const inbox = await openNotifications(page);
  await expect(notificationItem(inbox, actor.user.name)).toHaveCount(1);
});

test('other notification kinds keep arriving while new messages are off', async ({
  context,
  page,
  request,
}) => {
  const recipient = await createAuthenticatedUser(
    request,
    context,
    createTestUser('settings-reply-recipient'),
  );
  const actor = await signUpViaApi(
    request,
    createTestUser('settings-reply-actor'),
  );
  const server = await getDefaultServer(request, recipient);

  await page.goto(settingsPath);
  await newMessagesSwitch(page).click();
  await expect(newMessagesSwitch(page)).not.toBeChecked();
  await saveNotificationSettings(page);

  const rootResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(recipient),
      data: { body: `Reply root ${recipient.user.suffix}` },
    },
  );
  await expect(rootResponse).toBeOK();
  const rootMessage = (
    (await rootResponse.json()) as {
      message: { id: string };
    }
  ).message;

  const replyResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages/${rootMessage.id}/replies`,
    {
      headers: authorizationHeaders(actor),
      data: { body: `A reply ${recipient.user.suffix}` },
    },
  );
  await expect(replyResponse).toBeOK();

  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  await expectUnreadNotifications(page, 1);

  const inbox = await openNotifications(page);
  await expect(notificationItem(inbox, actor.user.name)).toContainText(
    'replied',
  );
});
