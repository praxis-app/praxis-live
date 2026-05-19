import { expect, test, type Locator, type Page } from '@playwright/test';
import {
  createAuthenticatedUser,
  seedAuthenticatedSession,
  signUpViaApi,
} from '../lib/auth';
import { createTestMessage, createTestUser } from '../lib/data';
import { createInvite } from '../lib/invites';
import { getDefaultServer } from '../lib/servers';
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

const expectRenderedParticipantTiles = async (
  page: { getByTestId: (testId: string) => Locator },
  count: number,
) => {
  const tiles = page.getByTestId('call-participant-tile');
  await expect(tiles).toHaveCount(count);

  for (let index = 0; index < count; index += 1) {
    await expectTileToRender(tiles.nth(index));
  }
};

const expectParticipantTilesToBeLaidOut = async (
  page: { getByTestId: (testId: string) => Locator },
  count: number,
) => {
  const tiles = page.getByTestId('call-participant-tile');

  await expectRenderedParticipantTiles(page, count);
  await expect
    .poll(async () => {
      const boxes = await Promise.all(
        Array.from({ length: count }, (_, index) =>
          tiles.nth(index).boundingBox(),
        ),
      );

      if (boxes.some((box) => !box)) {
        return false;
      }

      return boxes.every((box, index) =>
        boxes.every((otherBox, otherIndex) => {
          if (!box || !otherBox || index === otherIndex) {
            return true;
          }

          return (
            box.x + box.width <= otherBox.x ||
            otherBox.x + otherBox.width <= box.x ||
            box.y + box.height <= otherBox.y ||
            otherBox.y + otherBox.height <= box.y
          );
        }),
      );
    })
    .toBe(true);
};

const leaveCallIfVisible = async (page: Page) => {
  const leaveButton = page.getByRole('button', { name: 'Leave call' });
  if (!(await leaveButton.isVisible())) {
    return;
  }

  const leaveCallResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().endsWith('/leave') &&
      response.status() === 200,
  );

  await leaveButton.click();
  await leaveCallResponse;
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

  try {
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
  } finally {
    await leaveCallIfVisible(page);
  }
});

test('starting a call immediately appears in other users channel feeds', async ({
  browser,
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const starter = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-feed-starter'),
  );
  const server = await getDefaultServer(request, starter);
  const inviteToken = await createInvite(request, starter, server.id);
  const observer = await signUpViaApi(
    request,
    createTestUser('call-feed-observer'),
    inviteToken,
  );
  const observerContext = await browser.newContext();

  try {
    await seedAuthenticatedSession(observerContext, observer.accessToken);
    const observerPage = await observerContext.newPage();
    const starterChat = new ChatPage(page);
    const observerChat = new ChatPage(observerPage);

    await starterChat.goto();
    await starterChat.expectChannel('general');
    await observerPage.goto(page.url());
    await observerChat.expectChannel('general');

    const observerCallArtifact = observerPage
      .locator('article')
      .filter({ hasText: `Started by ${starter.user.name}` });

    await expect(observerCallArtifact).toHaveCount(0);

    const joinCallResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        /\/calls$/.test(response.url()) &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Call' }).click();
    await joinCallResponse;

    await expect(observerCallArtifact).toContainText('Call is active');
    await expect(
      observerCallArtifact.getByRole('button', { name: 'Join active video' }),
    ).toBeVisible();

    const leaveCallResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().endsWith('/leave') &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Leave call' }).click();
    await leaveCallResponse;

    await expect(observerCallArtifact).toContainText('Call ended');
    await expect(
      observerCallArtifact.getByRole('button', { name: 'Join active video' }),
    ).toHaveCount(0);
  } finally {
    await observerContext.close();
  }
});

test('second user can join an active call from the call artifact', async ({
  browser,
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const starter = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-artifact-starter'),
  );
  const server = await getDefaultServer(request, starter);
  const inviteToken = await createInvite(request, starter, server.id);
  const joiner = await signUpViaApi(
    request,
    createTestUser('call-artifact-joiner'),
    inviteToken,
  );
  const joinerContext = await browser.newContext();
  let joinerPage: Page | undefined;

  try {
    await seedAuthenticatedSession(joinerContext, joiner.accessToken);
    joinerPage = await joinerContext.newPage();
    const starterChat = new ChatPage(page);
    const joinerChat = new ChatPage(joinerPage);

    await starterChat.goto();
    await starterChat.expectChannel('general');
    await joinerPage.goto(page.url());
    await joinerChat.expectChannel('general');

    const joinerCallArtifact = joinerPage
      .locator('article')
      .filter({ hasText: `Started by ${starter.user.name}` });

    const starterJoinCallResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        /\/calls$/.test(response.url()) &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Call' }).click();
    await starterJoinCallResponse;
    await expect(page.getByText('Call in #general')).toBeVisible();
    await expectRenderedParticipantTiles(page, 1);
    await joinerPage.reload();
    await joinerChat.expectChannel('general');

    await expect(joinerCallArtifact).toContainText('Call is active');

    const joinerJoinCallResponse = joinerPage.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().includes('/calls/') &&
        response.url().endsWith('/join') &&
        response.status() === 200,
    );

    await joinerCallArtifact
      .getByRole('button', { name: 'Join active video' })
      .click();
    await joinerJoinCallResponse;
    await expect(joinerPage.getByText('Call in #general')).toBeVisible();

    await expectRenderedParticipantTiles(page, 2);
    await expectRenderedParticipantTiles(joinerPage, 2);

    await leaveCallIfVisible(joinerPage);
    await leaveCallIfVisible(page);
  } finally {
    if (joinerPage) {
      await leaveCallIfVisible(joinerPage);
    }
    await leaveCallIfVisible(page);
    await joinerContext.close();
  }
});

test('call renders four participant tiles without overlap', async ({
  browser,
  context,
  page,
  request,
}) => {
  test.setTimeout(90_000);

  await page.setViewportSize({ height: 720, width: 1280 });

  const starter = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-four-tiles-starter'),
  );
  const server = await getDefaultServer(request, starter);
  const inviteToken = await createInvite(request, starter, server.id);
  const joiners = await Promise.all(
    [1, 2, 3].map((index) =>
      signUpViaApi(
        request,
        createTestUser(`call-four-tiles-joiner-${index}`),
        inviteToken,
      ),
    ),
  );
  const joinerContexts = await Promise.all(
    joiners.map(async (joiner) => {
      const joinerContext = await browser.newContext({
        viewport: { height: 720, width: 1280 },
      });
      await seedAuthenticatedSession(joinerContext, joiner.accessToken);

      return joinerContext;
    }),
  );
  const joinerPages: Page[] = [];

  try {
    const starterChat = new ChatPage(page);
    await starterChat.goto();
    await starterChat.expectChannel('general');

    const starterJoinCallResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        /\/calls$/.test(response.url()) &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Call' }).click();
    await starterJoinCallResponse;
    await expect(page.getByText('Call in #general')).toBeVisible();
    await expectParticipantTilesToBeLaidOut(page, 1);

    for (const joinerContext of joinerContexts) {
      const joinerPage = await joinerContext.newPage();
      joinerPages.push(joinerPage);

      const joinerChat = new ChatPage(joinerPage);
      await joinerPage.goto(page.url());
      await joinerChat.expectChannel('general');

      const joinerCallArtifact = joinerPage
        .locator('article')
        .filter({ hasText: `Started by ${starter.user.name}` });
      await expect(joinerCallArtifact).toContainText('Call is active');

      const joinerJoinCallResponse = joinerPage.waitForResponse(
        (response) =>
          response.request().method() === 'POST' &&
          response.url().includes('/calls/') &&
          response.url().endsWith('/join') &&
          response.status() === 200,
      );

      await joinerCallArtifact
        .getByRole('button', { name: 'Join active video' })
        .click();
      await joinerJoinCallResponse;
      await expect(joinerPage.getByText('Call in #general')).toBeVisible();
    }

    await expectParticipantTilesToBeLaidOut(page, 4);
  } finally {
    for (const joinerPage of joinerPages) {
      await leaveCallIfVisible(joinerPage);
    }
    await leaveCallIfVisible(page);
    await Promise.all(joinerContexts.map((joinerContext) => joinerContext.close()));
  }
});

test('multi-user call stays active until the last participant leaves', async ({
  browser,
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const starter = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-last-leaver-starter'),
  );
  const server = await getDefaultServer(request, starter);
  const inviteToken = await createInvite(request, starter, server.id);
  const joiner = await signUpViaApi(
    request,
    createTestUser('call-last-leaver-joiner'),
    inviteToken,
  );
  const joinerContext = await browser.newContext();
  let joinerPage: Page | undefined;

  try {
    await seedAuthenticatedSession(joinerContext, joiner.accessToken);
    joinerPage = await joinerContext.newPage();
    const starterChat = new ChatPage(page);
    const joinerChat = new ChatPage(joinerPage);

    await starterChat.goto();
    await starterChat.expectChannel('general');
    await joinerPage.goto(page.url());
    await joinerChat.expectChannel('general');

    const starterCallArtifact = page
      .locator('article')
      .filter({ hasText: `Started by ${starter.user.name}` });
    const joinerCallArtifact = joinerPage
      .locator('article')
      .filter({ hasText: `Started by ${starter.user.name}` });

    const starterJoinCallResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        /\/calls$/.test(response.url()) &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Call' }).click();
    await starterJoinCallResponse;
    await expect(page.getByText('Call in #general')).toBeVisible();
    await expectRenderedParticipantTiles(page, 1);

    await joinerPage.reload();
    await joinerChat.expectChannel('general');
    await expect(joinerCallArtifact).toContainText('Call is active');

    const joinerJoinCallResponse = joinerPage.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().includes('/calls/') &&
        response.url().endsWith('/join') &&
        response.status() === 200,
    );

    await joinerCallArtifact
      .getByRole('button', { name: 'Join active video' })
      .click();
    await joinerJoinCallResponse;
    await expect(joinerPage.getByText('Call in #general')).toBeVisible();
    await expectRenderedParticipantTiles(page, 2);
    await expectRenderedParticipantTiles(joinerPage, 2);

    const starterLeaveCallResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().endsWith('/leave') &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Leave call' }).click();
    await starterLeaveCallResponse;

    await expectRenderedParticipantTiles(joinerPage, 1);
    await expect(starterCallArtifact).toContainText('Call is active');
    await expect(starterCallArtifact).not.toContainText('Call ended');

    const joinerLeaveCallResponse = joinerPage.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().endsWith('/leave') &&
        response.status() === 200,
    );

    await joinerPage.getByRole('button', { name: 'Leave call' }).click();
    await joinerLeaveCallResponse;

    await expect(starterCallArtifact).toContainText('Call ended');
  } finally {
    if (joinerPage) {
      await leaveCallIfVisible(joinerPage);
    }
    await leaveCallIfVisible(page);
    await joinerContext.close();
  }
});

test('in-call chat messages are delivered realtime between participants', async ({
  browser,
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const starter = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-chat-realtime-starter'),
  );
  const message = createTestMessage(
    'call-chat-realtime',
    starter.user.suffix,
  );
  const server = await getDefaultServer(request, starter);
  const inviteToken = await createInvite(request, starter, server.id);
  const joiner = await signUpViaApi(
    request,
    createTestUser('call-chat-realtime-joiner'),
    inviteToken,
  );
  const joinerContext = await browser.newContext();
  let joinerPage: Page | undefined;

  try {
    await seedAuthenticatedSession(joinerContext, joiner.accessToken);
    joinerPage = await joinerContext.newPage();
    const starterChat = new ChatPage(page);
    const joinerChat = new ChatPage(joinerPage);

    await starterChat.goto();
    await starterChat.expectChannel('general');
    await joinerPage.goto(page.url());
    await joinerChat.expectChannel('general');

    const joinerCallArtifact = joinerPage
      .locator('article')
      .filter({ hasText: `Started by ${starter.user.name}` });

    const starterJoinCallResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        /\/calls$/.test(response.url()) &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Call' }).click();
    await starterJoinCallResponse;
    await expect(page.getByText('Call in #general')).toBeVisible();
    await expectRenderedParticipantTiles(page, 1);

    await joinerPage.reload();
    await joinerChat.expectChannel('general');
    await expect(joinerCallArtifact).toContainText('Call is active');

    const joinerJoinCallResponse = joinerPage.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().includes('/calls/') &&
        response.url().endsWith('/join') &&
        response.status() === 200,
    );

    await joinerCallArtifact
      .getByRole('button', { name: 'Join active video' })
      .click();
    await joinerJoinCallResponse;
    await expect(joinerPage.getByText('Call in #general')).toBeVisible();
    await expectRenderedParticipantTiles(page, 2);
    await expectRenderedParticipantTiles(joinerPage, 2);

    const starterCallFeedResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'GET' &&
        response.url().includes('/calls/') &&
        response.url().includes('/feed') &&
        response.status() === 200,
    );
    await page.getByRole('button', { name: 'Open call chat' }).click();
    await starterCallFeedResponse;

    const joinerCallFeedResponse = joinerPage.waitForResponse(
      (response) =>
        response.request().method() === 'GET' &&
        response.url().includes('/calls/') &&
        response.url().includes('/feed') &&
        response.status() === 200,
    );
    await joinerPage.getByRole('button', { name: 'Open call chat' }).click();
    await joinerCallFeedResponse;

    const starterCallChatPanel = page.getByRole('region', {
      name: 'In-call chat',
    });
    const joinerCallChatPanel = joinerPage.getByRole('region', {
      name: 'In-call chat',
    });
    await expect(starterCallChatPanel).toBeVisible();
    await expect(joinerCallChatPanel).toBeVisible();

    const messageResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().includes('/calls/') &&
        response.url().includes('/messages') &&
        response.status() === 200,
    );

    await starterCallChatPanel
      .getByPlaceholder('Send a message...')
      .fill(message);
    await starterCallChatPanel
      .getByPlaceholder('Send a message...')
      .press('Enter');
    await messageResponse;

    await expect(joinerCallChatPanel.getByText(message)).toBeVisible();
    await expect(
      joinerCallChatPanel.getByText(starter.user.name).first(),
    ).toBeVisible();
  } finally {
    if (joinerPage) {
      await leaveCallIfVisible(joinerPage);
    }
    await leaveCallIfVisible(page);
    await joinerContext.close();
  }
});
