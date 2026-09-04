import { expect, test } from '@playwright/test';
import {
  authorizationHeaders,
  createAuthenticatedUser,
  getOrCreateInstanceAdmin,
  signUpViaApi,
} from '../lib/auth';
import { createTestUser } from '../lib/data';
import { notificationItem } from '../lib/notifications';
import { getDefaultServer } from '../lib/servers';

type MessageResponse = { message: { id: string } };

const threadPanelWidth = async (page: import('@playwright/test').Page) => {
  const box = await page.getByTestId('thread-panel').boundingBox();
  return Math.round(box?.width ?? 0);
};

test.beforeAll(async ({ request }) => {
  await getOrCreateInstanceAdmin(request);
});

test('a resized thread panel keeps its width when a notification opens it', async ({
  context,
  page,
  request,
}) => {
  const recipient = await createAuthenticatedUser(
    request,
    context,
    createTestUser('panel-width-recipient'),
  );
  const actor = await signUpViaApi(
    request,
    createTestUser('panel-width-actor'),
  );
  const server = await getDefaultServer(request, recipient);

  const rootResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(recipient),
      data: { body: `Width thread root ${recipient.user.suffix}` },
    },
  );
  await expect(rootResponse).toBeOK();
  const root = ((await rootResponse.json()) as MessageResponse).message;

  await page.setViewportSize({ width: 1440, height: 800 });
  await context.addInitScript(() => {
    localStorage.setItem('decisions-panel-open', 'false');
  });
  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);

  // Open the thread the way a user does, from the message itself.
  const message = page.locator(`[data-message-id="${root.id}"]`);
  await expect(message).toBeVisible();
  await message.hover();
  await message.getByRole('button', { name: 'Open message menu' }).click();
  await page.getByRole('menuitem', { name: 'Reply' }).click();
  await expect(page.getByTestId('thread-panel')).toBeVisible();

  // Widen the thread panel by dragging its handle to the left.
  const handle = page.getByRole('separator', { name: 'Resize right panel' });
  const handleBox = await handle.boundingBox();
  if (!handleBox) {
    throw new Error('Resize handle not found');
  }
  await page.mouse.move(
    handleBox.x + handleBox.width / 2,
    handleBox.y + handleBox.height / 2,
  );
  await page.mouse.down();
  await page.mouse.move(
    handleBox.x + handleBox.width / 2 + 120,
    handleBox.y + handleBox.height / 2,
    { steps: 10 },
  );
  await page.mouse.up();

  const resizedWidth = await threadPanelWidth(page);
  expect(resizedWidth).toBeLessThan(420);

  const storedWidth = await page.evaluate(() => {
    const raw = localStorage.getItem('panel-widths');
    return raw ? (JSON.parse(raw) as { thread?: number }).thread : undefined;
  });
  expect(storedWidth).toBeLessThan(420);

  // Leave the channel the way a user does, without reloading the app.
  await page.getByRole('button', { name: 'User Settings' }).click();
  await expect(
    page.getByRole('heading', { name: 'Profile', exact: true }),
  ).toBeVisible();

  const replyResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages/${root.id}/replies`,
    {
      headers: authorizationHeaders(actor),
      data: { body: `Width reply ${actor.user.suffix}` },
    },
  );
  await expect(replyResponse).toBeOK();

  await page.getByTestId('notification-bell').click();
  const inbox = page.locator('section[aria-label="Notifications"]');
  await notificationItem(inbox, 'replied to a conversation')
    .getByRole('button')
    .first()
    .click();

  await expect(page).toHaveURL(new RegExp(`thread=${root.id}`));
  await expect(page.getByTestId('thread-panel')).toBeVisible();
  await expect(
    page.getByText(`Width reply ${actor.user.suffix}`),
  ).toBeVisible();

  await expect.poll(() => threadPanelWidth(page)).toBeCloseTo(resizedWidth, -1);
});
