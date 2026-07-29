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
  setupAnonymousInvite,
} from '../lib/auth';
import { startCallFromTopNav } from '../lib/calls';
import { createTestMessage, createTestUser } from '../lib/data';
import { scrollThroughAllPages } from '../lib/infinite-scroll';
import { createMessages } from '../lib/messages';
import { getDefaultServer } from '../lib/servers';
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
  const otherChannelName = `other-${user.user.suffix}`;
  const createChannelResponse = await request.post(
    `/api/servers/${server.id}/channels`,
    {
      headers: authorizationHeaders(user),
      data: {
        name: otherChannelName,
        description: 'Channel used to verify feed cache behavior.',
        channelType: 'text',
      },
    },
  );
  await expect(createChannelResponse).toBeOK();
  const { channel: otherChannel } =
    (await createChannelResponse.json()) as {
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
  await page
    .getByRole('link', { name: otherChannelName, exact: true })
    .click();
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
    if (
      networkRequest.method() === 'GET' &&
      url.pathname === feedPath
    ) {
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

  const uploadResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes('/images/') &&
      response.url().endsWith('/upload') &&
      response.status() === 201,
  );

  await chat.attachImage();
  await chat.sendMessage(message);
  await uploadResponse;

  await chat.expectMessage(message, authenticatedUser.user.name);
  await chat.expectAttachedImage();
});

test('authenticated user can send an in-call chat message with an image', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);
  const browserErrors: string[] = [];

  page.on('console', (message) => {
    if (message.type() === 'error') {
      browserErrors.push(message.text());
    }
  });
  page.on('pageerror', (error) => {
    browserErrors.push(error.message);
  });

  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser('call-chat-image'),
  );
  const message = createTestMessage(
    'call-chat-image',
    authenticatedUser.user.suffix,
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

    await startCallFromTopNav(page);
    await joinCallResponse;
    await expect(page.getByText('Call in #general')).toBeVisible();

    const callFeedResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'GET' &&
        response.url().includes('/calls/') &&
        response.url().includes('/feed') &&
        response.status() === 200,
    );

    await page.getByRole('button', { name: 'Open call chat' }).click();
    await callFeedResponse;

    const callChatPanel = page.getByRole('region', { name: 'In-call chat' });
    await expect(callChatPanel).toBeVisible();

    const messageResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().includes('/calls/') &&
        response.url().includes('/messages') &&
        response.status() === 200,
    );
    const uploadResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' &&
        response.url().includes('/calls/') &&
        response.url().includes('/images/') &&
        response.url().endsWith('/upload') &&
        response.status() === 201,
    );

    await chat.attachImageIn(callChatPanel);
    await callChatPanel.getByPlaceholder('Send a message...').fill(message);
    await callChatPanel.getByPlaceholder('Send a message...').press('Enter');
    await messageResponse;
    await uploadResponse;

    await expect(callChatPanel.getByText(message)).toBeVisible();
    await expect(
      callChatPanel.getByRole('img', { name: 'Attached image' }).first(),
    ).toBeVisible();
    expect(browserErrors).toEqual([]);
  } finally {
    await leaveCallIfVisible(page);
  }
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
        response.url().endsWith(
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
      if (
        networkRequest.method() === 'GET' &&
        url.pathname === callFeedPath
      ) {
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

    await page
      .getByRole('button', { name: 'Decisions', exact: true })
      .click();
    await decisionResponse;

    const activeDecisionPanel = page.getByRole('region', {
      name: 'Active Decision',
    });
    await expect(activeDecisionPanel).toBeVisible();
    await expect(activeDecisionPanel.getByText('No active decision')).toBeVisible();

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
    await expect(page.getByText('Active Decision')).toBeVisible();
    await expect(activeDecisionPanel.getByText(/0\/\d+ responded/)).toBeVisible();

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

  const uploadResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes('/images/') &&
      response.url().endsWith('/upload') &&
      response.status() === 201,
  );
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
  await uploadResponse;
  await anonSessionResponse;
  await messageResponse;
  await expect(page.getByText(message)).toBeVisible();
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
