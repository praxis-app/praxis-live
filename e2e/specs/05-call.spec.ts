import { expect, test, type Locator } from '@playwright/test';
import { createAuthenticatedUser } from '../lib/auth';
import { createTestUser } from '../lib/data';
import { ChatPage } from '../pages/chat.page';
import { NavigationPage } from '../pages/navigation.page';

const expectTileToRender = async (tile: Locator) => {
  await expect(tile).toBeVisible();
  await expect
    .poll(async () => {
      const box = await tile.boundingBox();

      return Math.min(box?.height ?? 0, box?.width ?? 0);
    })
    .toBeGreaterThan(100);
};

test('authenticated user can start a call and see a video tile', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-video-tile'),
  );
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await chat.goto();

  await chat.expectChannel('general');
  await navigation.expectSignedInUser(authenticatedUser.user);

  const joinCallResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      /\/calls$/.test(response.url()) &&
      response.status() === 200,
  );

  await page.getByRole('button', { name: 'Call' }).click();
  await joinCallResponse;

  await expect(page.getByText('Call in #general')).toBeVisible();

  const tile = page.getByTestId('call-participant-tile').first();
  await expectTileToRender(tile);

  await page.setViewportSize({ height: 760, width: 390 });
  await expectTileToRender(tile);

  await page.setViewportSize({ height: 720, width: 1280 });
  await expectTileToRender(tile);

  const callFeedResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'GET' &&
      response.url().includes('/calls/') &&
      response.url().includes('/feed') &&
      response.status() === 200,
  );

  await page.getByRole('button', { name: 'Open call chat' }).click();
  await callFeedResponse;
  await expectTileToRender(tile);
});
