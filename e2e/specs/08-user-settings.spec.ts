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

test('edit profile opens the profile section of user settings', async ({
  context,
  page,
  request,
}) => {
  const user = await createAuthenticatedUser(
    request,
    context,
    createTestUser('settings-edit-profile'),
  );
  const server = await getDefaultServer(request, user);

  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);

  await page.getByRole('button', { name: / Online$/ }).click();
  await page.getByRole('menuitem', { name: 'Edit profile' }).click();

  await expect(page).toHaveURL(/\/users\/settings\?section=profile$/);
  await expectUnifiedSettingsPage(page);
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
