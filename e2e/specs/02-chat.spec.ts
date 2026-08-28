import { readFile } from 'node:fs/promises';
import {
  expect,
  test,
  type Locator,
  type Page,
  type Request,
} from '@playwright/test';
import {
  authorizationHeaders,
  createAuthenticatedUser,
  getOrCreateInstanceAdmin,
  seedAuthenticatedSession,
  signUpViaApi,
  setupAnonymousInvite,
} from '../lib/auth';
import { startCallFromTopNav } from '../lib/calls';
import { createTestMessage, createTestUser } from '../lib/data';
import { expectImageToLoad } from '../lib/images';
import { createInvite } from '../lib/invites';
import { scrollThroughAllPages } from '../lib/infinite-scroll';
import { createMessages, pressAndHold } from '../lib/messages';
import { expectRightPanelToResize } from '../lib/right-panel';
import {
  createServer,
  createServerAdmin,
  getDefaultServer,
} from '../lib/servers';
import { ChatPage } from '../pages/chat.page';
import { NavigationPage } from '../pages/navigation.page';

type PollResponse = {
  poll: {
    id: string;
  };
};

type JoinCallResponse = {
  call: {
    id: string;
  };
};

const feedPageSize = 20;
const totalFeedMessages = 41;

test.beforeAll(async ({ request }) => {
  await getOrCreateInstanceAdmin(request);
});

test('authenticated user can send a basic chat message', async ({
  context,
  page,
  request,
}) => {
  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser('chat'),
  );
  const message = createTestMessage('chat', authenticatedUser.user.suffix);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await chat.goto();

  await chat.expectChannel('general');
  await navigation.expectSignedInUser(authenticatedUser.user);
  await chat.sendMessage(message);
  await chat.expectMessage(message, authenticatedUser.user.name);
});

test('members can use a durable reply thread without replies entering the channel feed', async ({
  browser,
  context,
  page,
  request,
}) => {
  const author = await createAuthenticatedUser(
    request,
    context,
    createTestUser('thread-author'),
  );
  const server = await getDefaultServer(request, author);
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  const inviteToken = await createInvite(request, instanceAdmin, server.id);
  const member = await signUpViaApi(
    request,
    createTestUser('thread-member'),
    inviteToken,
  );
  const memberContext = await browser.newContext();

  try {
    await seedAuthenticatedSession(memberContext, member.accessToken);
    const memberPage = await memberContext.newPage();
    const authorChat = new ChatPage(page);
    const rootMessage = `Thread root ${author.user.suffix}`;
    const replyMessage = `Thread reply ${member.user.suffix}`;

    await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
    await authorChat.sendMessage(rootMessage);
    const authorRoot = page
      .getByTestId('feed')
      .locator('[data-message-id]')
      .filter({ hasText: rootMessage });
    await expect(authorRoot).toBeVisible();
    await authorRoot.hover();
    await authorRoot.getByRole('button', { name: 'Open message menu' }).click();
    await page.getByRole('menuitem', { name: 'Reply' }).click();
    const authorThread = page.getByTestId('thread-panel');
    await expect(authorThread.getByText(rootMessage)).toBeVisible();
    await expectRightPanelToResize(page, authorThread, 'thread');

    await memberPage.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
    const memberFeed = memberPage.getByTestId('feed');
    const memberRoot = memberFeed
      .locator('[data-message-id]')
      .filter({ hasText: rootMessage });
    await expect(memberRoot).toBeVisible();
    await memberRoot.hover();
    await memberRoot.getByRole('button', { name: 'Open message menu' }).click();
    await memberPage.getByRole('menuitem', { name: 'Reply' }).click();
    const memberThread = memberPage.getByTestId('thread-panel');
    await expect(memberThread.getByText(rootMessage)).toBeVisible();

    let releaseReplyRequest = () => {};
    const heldReplyRequest = new Promise<void>((resolve) => {
      releaseReplyRequest = resolve;
    });
    let markReplyRequestObserved = () => {};
    const replyRequestObserved = new Promise<void>((resolve) => {
      markReplyRequestObserved = resolve;
    });
    const replyRoute = /\/messages\/[^/]+\/replies$/;
    await memberPage.route(replyRoute, async (route) => {
      if (route.request().method() !== 'POST') {
        await route.continue();
        return;
      }
      markReplyRequestObserved();
      await heldReplyRequest;
      await route.continue();
    });

    const createReplyResponse = memberPage.waitForResponse((response) => {
      const url = new URL(response.url());
      return (
        response.request().method() === 'POST' &&
        /\/messages\/[^/]+\/replies$/.test(url.pathname) &&
        response.status() === 200
      );
    });
    const replyInput = memberThread.getByPlaceholder('Reply to thread...');
    await replyInput.fill(replyMessage);
    await replyInput.press('Enter');
    await replyRequestObserved;
    const memberReply = memberThread.locator('[data-message-id]').filter({
      hasText: replyMessage,
    });
    await expect(memberReply).toHaveCount(0);
    releaseReplyRequest();
    await createReplyResponse;
    await memberPage.unroute(replyRoute);

    await expect(memberReply).toBeVisible();
    await expect(
      authorThread.locator('[data-message-id]').filter({
        hasText: replyMessage,
      }),
    ).toBeVisible();
    await expect(
      authorRoot.getByRole('button', { name: '1 reply' }),
    ).toBeVisible();
    await expect(
      memberRoot.getByRole('button', { name: '1 reply' }),
    ).toBeVisible();
    await expect(
      memberRoot
        .getByRole('button', { name: '1 reply' })
        .getByTitle(member.user.name),
    ).toBeVisible();
    await expect(page.getByTestId('feed').getByText(replyMessage)).toHaveCount(
      0,
    );
    await expect(memberFeed.getByText(replyMessage)).toHaveCount(0);

    await memberPage.reload();
    await expect(memberPage).toHaveURL(/\?thread=[0-9a-f-]+$/);
    const reloadedThread = memberPage.getByTestId('thread-panel');
    await expect(reloadedThread.getByText(rootMessage)).toBeVisible();
    await expect(
      reloadedThread.locator('[data-message-id]').filter({
        hasText: replyMessage,
      }),
    ).toBeVisible();
  } finally {
    await memberContext.close();
  }
});

test('opening a message thread on mobile fills the viewport without scrolling', async ({
  context,
  page,
  request,
}) => {
  const user = await createAuthenticatedUser(
    request,
    context,
    createTestUser('mobile-thread'),
  );
  const server = await getDefaultServer(request, user);
  const chat = new ChatPage(page);
  const rootMessage = `Mobile thread root ${user.user.suffix}`;

  await page.setViewportSize({ width: 390, height: 760 });
  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  await chat.sendMessage(rootMessage);

  const root = page
    .getByTestId('feed')
    .locator('[data-message-id]')
    .filter({ hasText: rootMessage });
  await expect(root).toBeVisible();
  await expect(
    root.getByRole('button', { name: 'Open message menu' }),
  ).toHaveCount(0);

  const restingBackground = await backgroundColor(root);
  const release = await pressAndHold(root);

  // The highlight lands almost immediately, well inside the 700ms the menu
  // takes to open, and then persists while the menu is up.
  await expect
    .poll(() => backgroundColor(root), { timeout: 500 })
    .not.toBe(restingBackground);
  await expect(root).toHaveAttribute('data-state', 'open');
  await release();
  await expect.poll(() => backgroundColor(root)).not.toBe(restingBackground);

  // The highlight has to resolve to a real, distinct color in both themes.
  const highlight = await root.evaluate((element) => {
    const html = document.documentElement;
    const previousTheme = html.className;
    const previousTransition = element.style.transition;
    element.style.transition = 'none';
    const read = (theme: string) => {
      html.classList.remove('light', 'dark');
      html.classList.add(theme);
      return window.getComputedStyle(element).backgroundColor;
    };
    const colors = { light: read('light'), dark: read('dark') };
    html.className = previousTheme;
    element.style.transition = previousTransition;
    return colors;
  });
  expect(highlight.light).not.toBe('rgba(0, 0, 0, 0)');
  expect(highlight.dark).not.toBe('rgba(0, 0, 0, 0)');
  expect(highlight.light).not.toBe(highlight.dark);

  await page.getByRole('menuitem', { name: 'Reply' }).click();

  const threadPanel = page.getByTestId('thread-panel');
  await expect(threadPanel.getByText(rootMessage)).toBeVisible();

  // The panel replaces the channel view on mobile, so it must land on screen
  // rather than below the channel view's full-height layout.
  await expect(threadPanel).toBeInViewport({ ratio: 0.9 });
  await expect(
    threadPanel.getByRole('button', { name: 'Back to channel' }),
  ).toBeInViewport();
  await expect(
    threadPanel.getByPlaceholder('Reply to thread...'),
  ).toBeInViewport();

  const layout = await threadPanel.evaluate((element) => ({
    top: element.getBoundingClientRect().top,
    height: element.getBoundingClientRect().height,
    pageScrollY: window.scrollY,
    viewportHeight: window.innerHeight,
  }));
  expect(layout.pageScrollY).toBe(0);
  expect(layout.top).toBeLessThanOrEqual(1);
  expect(layout.height).toBeGreaterThan(layout.viewportHeight * 0.9);
});

test('sending a message snaps a scrolled channel feed to the bottom', async ({
  context,
  page,
  request,
}) => {
  const user = await createAuthenticatedUser(
    request,
    context,
    createTestUser('send-scroll'),
  );
  const server = await getDefaultServer(request, user);
  const existingMessages = Array.from(
    { length: feedPageSize },
    (_, index) =>
      `Existing message ${String(index + 1).padStart(2, '0')} ${
        user.user.suffix
      }`,
  );
  await createMessages({
    request,
    user,
    serverId: server.id,
    channelId: server.generalChannelId,
    bodies: existingMessages,
  });

  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  const feed = page.getByTestId('feed');
  await expect(feed.getByText(existingMessages.at(-1)!)).toBeVisible();
  await feed.evaluate((element) => {
    element.scrollTop = -element.scrollHeight;
  });
  await expect
    .poll(() => feed.evaluate((element) => element.scrollTop))
    .toBeLessThan(-200);

  const message = createTestMessage('send-scroll', user.user.suffix);
  const chat = new ChatPage(page);
  await chat.sendMessage(message);

  await expect
    .poll(() => feed.evaluate((element) => element.scrollTop))
    .toBe(0);
  await expect(feed.getByText(message)).toBeVisible();
});

test('text channel feed preserves its pages and syncs only newer messages when revisited', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const user = await createAuthenticatedUser(
    request,
    context,
    createTestUser('text-scroll'),
  );
  const server = await getDefaultServer(request, user);
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  const otherChannelName = `other-${user.user.suffix}`;
  const createChannelResponse = await request.post(
    `/api/servers/${server.id}/channels`,
    {
      headers: authorizationHeaders(instanceAdmin),
      data: {
        name: otherChannelName,
        description: 'Channel used to verify feed cache behavior.',
        channelType: 'text',
      },
    },
  );
  await expect(createChannelResponse).toBeOK();
  const { channel: otherChannel } = (await createChannelResponse.json()) as {
    channel: { id: string };
  };

  const messageBodies = Array.from(
    { length: totalFeedMessages },
    (_, index) =>
      `Infinite text message ${String(index + 1).padStart(2, '0')} ${
        user.user.suffix
      }`,
  );
  await createMessages({
    request,
    user,
    serverId: server.id,
    channelId: server.generalChannelId,
    bodies: messageBodies,
  });

  const feedPath = `/api/servers/${server.id}/channels/${server.generalChannelId}/feed`;
  const firstPageResponse = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      response.request().method() === 'GET' &&
      url.pathname === feedPath &&
      !url.searchParams.has('before') &&
      !url.searchParams.has('after') &&
      url.searchParams.get('limit') === String(feedPageSize) &&
      response.status() === 200
    );
  });
  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  await firstPageResponse;

  const feed = page.getByTestId('feed');
  const oldestMessage = messageBodies[0];
  await expect(feed.getByText(messageBodies.at(-1)!)).toBeVisible();
  await expect(feed.getByText(oldestMessage)).toHaveCount(0);

  await scrollThroughAllPages({
    page,
    scrollContainer: feed,
    pageSize: feedPageSize,
    totalItems: totalFeedMessages,
    direction: 'up',
    matchesPageResponse: (response) => {
      const url = new URL(response.url());
      return (
        response.request().method() === 'GET' &&
        url.pathname === feedPath &&
        url.searchParams.has('before') &&
        url.searchParams.get('limit') === String(feedPageSize) &&
        response.status() === 200
      );
    },
  });
  await expect(feed.getByText(oldestMessage)).toBeVisible();

  const otherFeedResponse = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      response.request().method() === 'GET' &&
      url.pathname ===
        `/api/servers/${server.id}/channels/${otherChannel.id}/feed` &&
      !url.searchParams.has('before') &&
      !url.searchParams.has('after') &&
      response.status() === 200
    );
  });
  await page.getByRole('link', { name: otherChannelName, exact: true }).click();
  await otherFeedResponse;

  const newerMessage = `Message received while away ${user.user.suffix}`;
  await createMessages({
    request,
    user,
    serverId: server.id,
    channelId: server.generalChannelId,
    bodies: [newerMessage],
  });

  const revisitRequests: string[] = [];
  const recordRevisitedFeedRequest = (networkRequest: Request) => {
    const url = new URL(networkRequest.url());
    if (networkRequest.method() === 'GET' && url.pathname === feedPath) {
      revisitRequests.push(
        url.searchParams.has('after')
          ? 'after'
          : url.searchParams.has('before')
            ? 'before'
            : 'initial',
      );
    }
  };
  page.on('request', recordRevisitedFeedRequest);

  const newerMessagesResponse = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      response.request().method() === 'GET' &&
      url.pathname === feedPath &&
      url.searchParams.has('after') &&
      response.status() === 200
    );
  });
  await page.getByRole('link', { name: 'general', exact: true }).click();
  await newerMessagesResponse;
  await page.waitForTimeout(500);
  page.off('request', recordRevisitedFeedRequest);

  expect(revisitRequests).toEqual(['after']);
  await expect(feed.getByText(newerMessage)).toBeVisible();
  await expect(feed.getByText(oldestMessage)).toBeVisible();
});

test('authenticated user can send a chat message with an image', async ({
  context,
  page,
  request,
}) => {
  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser('chat-image'),
  );
  const message = createTestMessage(
    'chat-image',
    authenticatedUser.user.suffix,
  );
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await chat.goto();

  await chat.expectChannel('general');
  await navigation.expectSignedInUser(authenticatedUser.user);

  const messageResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().endsWith('/messages') &&
      response.status() === 200,
  );

  await chat.attachImage();
  await chat.sendMessage(message);
  await messageResponse;

  await chat.expectMessage(message, authenticatedUser.user.name);
  await chat.expectAttachedImage();
});

test('invite holder can read a non-default server feed with images', async ({
  context,
  page,
  request,
}) => {
  const admin = await createServerAdmin(request, 'invite-feed-admin');
  const serverSlug = `invite-feed-${admin.user.suffix}`;
  await createServer(request, admin, {
    name: `Invite feed ${admin.user.suffix}`,
    slug: serverSlug,
    description: 'Non-default server for invite feed access.',
  });

  const getServerResponse = await request.get(
    `/api/servers/slug/${serverSlug}`,
    { headers: authorizationHeaders(admin) },
  );
  await expect(getServerResponse).toBeOK();
  const { server } = (await getServerResponse.json()) as {
    server: {
      id: string;
      slug: string;
      generalChannelId: string;
    };
  };

  const messageBody = createTestMessage('invite-feed', admin.user.suffix);
  const createMessageResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(admin),
      multipart: {
        payload: JSON.stringify({ body: messageBody }),
        files: {
          name: 'valid-image.png',
          mimeType: 'image/png',
          buffer: await readFile('e2e/fixtures/valid-image.png'),
        },
      },
    },
  );
  await expect(createMessageResponse).toBeOK();
  const { message } = (await createMessageResponse.json()) as {
    message: { id: string };
  };

  const feedPath = `/api/servers/${server.id}/channels/${server.generalChannelId}/feed`;
  const feedWithoutInviteResponse = await request.get(feedPath);
  expect(feedWithoutInviteResponse.status()).toBe(403);

  const inviteToken = await createInvite(request, admin, server.id);
  await context.addInitScript((token) => {
    window.localStorage.removeItem('access_token');
    window.localStorage.setItem('invite-token', token);
  }, inviteToken);

  const feedResponsePromise = page.waitForResponse((response) =>
    response.url().includes(feedPath),
  );
  const imageResponsePromise = page.waitForResponse((response) =>
    response.url().includes(`/messages/${message.id}/images/`),
  );
  await page.goto(`/s/${server.slug}`);

  expect((await feedResponsePromise).status()).toBe(200);
  await expect(page.getByText(messageBody)).toBeVisible();
  expect((await imageResponsePromise).status()).toBe(200);
  await expectImageToLoad(
    page.getByRole('img', { name: 'Attached image' }).first(),
  );
});

test('in-call chat feed preserves its pages and syncs only newer messages when reopened', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(90_000);

  const user = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-scroll'),
  );
  const server = await getDefaultServer(request, user);

  try {
    await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);

    const joinCallResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response
          .url()
          .endsWith(
            `/api/servers/${server.id}/channels/${server.generalChannelId}/calls`,
          ) &&
        response.status() === 200,
    );
    await startCallFromTopNav(page);
    const callResponse = await joinCallResponse;
    const { call } = (await callResponse.json()) as JoinCallResponse;

    const messageBodies = Array.from(
      { length: totalFeedMessages },
      (_, index) =>
        `Infinite call message ${String(index + 1).padStart(2, '0')} ${
          user.user.suffix
        }`,
    );
    await createMessages({
      request,
      user,
      serverId: server.id,
      channelId: server.generalChannelId,
      callId: call.id,
      bodies: messageBodies,
    });

    const callFeedPath = `/api/servers/${server.id}/channels/${server.generalChannelId}/calls/${call.id}/feed`;
    const firstPageResponse = page.waitForResponse((response) => {
      const url = new URL(response.url());
      return (
        response.request().method() === 'GET' &&
        url.pathname === callFeedPath &&
        !url.searchParams.has('before') &&
        !url.searchParams.has('after') &&
        url.searchParams.get('limit') === String(feedPageSize) &&
        response.status() === 200
      );
    });
    await page.getByRole('button', { name: 'Open call chat' }).click();
    await firstPageResponse;

    const callChatPanel = page.getByRole('region', {
      name: 'In-call chat',
    });
    const callFeed = callChatPanel.getByTestId('feed');
    const oldestMessage = messageBodies[0];
    await expect(callFeed.getByText(messageBodies.at(-1)!)).toBeVisible();
    await expect(callFeed.getByText(oldestMessage)).toHaveCount(0);

    await scrollThroughAllPages({
      page,
      scrollContainer: callFeed,
      pageSize: feedPageSize,
      totalItems: totalFeedMessages,
      direction: 'up',
      matchesPageResponse: (response) => {
        const url = new URL(response.url());
        return (
          response.request().method() === 'GET' &&
          url.pathname === callFeedPath &&
          url.searchParams.has('before') &&
          url.searchParams.get('limit') === String(feedPageSize) &&
          response.status() === 200
        );
      },
    });

    await expect(callFeed.getByText(oldestMessage)).toBeVisible();

    await page.getByRole('button', { name: 'Open call chat' }).click();
    await expect(callChatPanel).toHaveCount(0);

    const newerMessage = `Call message received while closed ${user.user.suffix}`;
    await createMessages({
      request,
      user,
      serverId: server.id,
      channelId: server.generalChannelId,
      callId: call.id,
      bodies: [newerMessage],
    });

    const revisitRequests: string[] = [];
    const recordRevisitedFeedRequest = (networkRequest: Request) => {
      const url = new URL(networkRequest.url());
      if (networkRequest.method() === 'GET' && url.pathname === callFeedPath) {
        revisitRequests.push(
          url.searchParams.has('after')
            ? 'after'
            : url.searchParams.has('before')
              ? 'before'
              : 'initial',
        );
      }
    };
    page.on('request', recordRevisitedFeedRequest);

    const newerMessagesResponse = page.waitForResponse((response) => {
      const url = new URL(response.url());
      return (
        response.request().method() === 'GET' &&
        url.pathname === callFeedPath &&
        url.searchParams.has('after') &&
        response.status() === 200
      );
    });
    await page.getByRole('button', { name: 'Open call chat' }).click();
    await newerMessagesResponse;
    page.off('request', recordRevisitedFeedRequest);

    const reopenedCallFeed = page
      .getByRole('region', { name: 'In-call chat' })
      .getByTestId('feed');
    expect(revisitRequests).toEqual(['after']);
    await expect(reopenedCallFeed.getByText(newerMessage)).toBeVisible();
    await expect(reopenedCallFeed.getByText(oldestMessage)).toBeVisible();
  } finally {
    await leaveCallIfVisible(page);
  }
});

test('authenticated user can create and vote on an in-call proposal', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-proposal-vote'),
  );
  const server = await getDefaultServer(request, authenticatedUser);
  const proposalBody = `In-call proposal ${authenticatedUser.user.suffix}`;
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

    await startCallFromTopNav(page);
    await joinCallResponse;
    await expect(page.getByText('Call in #general')).toBeVisible();

    const decisionResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'GET' &&
        response.url().includes('/calls/') &&
        response.url().includes('/decisions') &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Decisions', exact: true }).click();
    await decisionResponse;

    const activeDecisionPanel = page.getByRole('region', {
      name: 'Active Decision',
    });
    await expect(activeDecisionPanel).toBeVisible();
    await expect(
      activeDecisionPanel.getByText('No active decision'),
    ).toBeVisible();
    await expectRightPanelToResize(page, activeDecisionPanel, 'callDecisions');

    await activeDecisionPanel
      .getByRole('button', { name: 'Create proposal' })
      .click();

    const proposalDialog = page.getByRole('dialog', {
      name: 'Create a New Proposal',
    });
    await proposalDialog.getByRole('combobox').click();
    await page.getByRole('option', { name: 'Test' }).click();
    await proposalDialog
      .getByPlaceholder('Enter your proposal details...')
      .fill(proposalBody);
    await proposalDialog.getByRole('button', { name: 'Next' }).click();

    const createProposalResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response
          .url()
          .includes(`/channels/${server.generalChannelId}/calls/`) &&
        response.url().includes('/polls') &&
        response.status() === 200,
    );
    await proposalDialog
      .getByRole('button', { name: 'Create proposal' })
      .click();
    const createResponse = await createProposalResponse;
    const { poll } = (await createResponse.json()) as PollResponse;

    await expect(proposalDialog).toBeHidden();

    const proposal = activeDecisionPanel.getByRole('article', {
      name: `Consensus proposal: ${proposalBody}`,
    });
    await expect(proposal).toBeVisible();
    await expect(
      activeDecisionPanel.getByRole('heading', {
        name: 'Active Decision',
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      activeDecisionPanel.getByText(/0\/\d+ responded/),
    ).toBeVisible();

    const disagreeButton = proposal.getByRole('button', { name: 'Disagree' });
    const initialBackground = await backgroundColor(disagreeButton);

    const createVoteResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().includes(`/polls/${poll.id}/votes`) &&
        response.status() === 200,
    );
    await disagreeButton.click();
    await createVoteResponse;

    await expect
      .poll(() => backgroundColor(disagreeButton))
      .not.toBe(initialBackground);

    const updateVoteResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'PUT' &&
        response.url().includes(`/polls/${poll.id}/votes/`) &&
        response.status() === 200,
    );
    await proposal.getByRole('button', { name: 'Abstain' }).click();
    await updateVoteResponse;

    await leaveCallIfVisible(page);
    const channelProposal = page.getByRole('article', {
      name: `Consensus proposal: ${proposalBody}`,
    });
    await expect(channelProposal).toBeVisible();
    await expect(channelProposal.getByText('Created in-call')).toBeVisible();
    await expect(
      channelProposal.getByRole('link', { name: 'View call' }),
    ).toBeVisible();
  } finally {
    await leaveCallIfVisible(page);
  }
});

test('anonymous user can send messages with an image attached', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const { admin, server } = await setupAnonymousInvite(
    request,
    context,
    'anon-chat-image',
  );
  const message = createTestMessage('anon-chat-image', admin.user.suffix);
  const chat = new ChatPage(page);

  await chat.gotoExplore();
  await chat.expectChannel('general');

  const anonSessionResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().endsWith('/api/auth/anon') &&
      response.status() === 200,
  );
  const messageResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response
        .url()
        .includes(`/channels/${server.generalChannelId}/messages`) &&
      response.status() === 200,
  );

  await chat.attachImage();
  await chat.sendMessage(message);
  await page.getByRole('button', { name: 'Send anonymously' }).click();
  await anonSessionResponse;
  await messageResponse;
  await expect(chat.messageFeed().getByText(message)).toBeVisible();
  await chat.expectAttachedImage();
});

async function leaveCallIfVisible(page: Page) {
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
}

async function backgroundColor(locator: Locator) {
  return locator.evaluate(
    (element) => window.getComputedStyle(element).backgroundColor,
  );
}
