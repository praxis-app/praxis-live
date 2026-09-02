import { expect, test, type Locator } from '@playwright/test';
import {
  authorizationHeaders,
  createAuthenticatedUser,
  getOrCreateInstanceAdmin,
  signUpViaApi,
} from '../lib/auth';
import { createTestUser } from '../lib/data';
import { createForumChannel } from '../lib/forums';
import { createMessages } from '../lib/messages';
import {
  channelListItem,
  channelUnreadIndicator,
  expectUnreadNotifications,
  notificationItem,
  openNotifications,
} from '../lib/notifications';
import {
  makeProposalsRatifyWithOneAgreeVote,
  voteViaApi,
} from '../lib/polls';
import { getDefaultServer } from '../lib/servers';

/** Matches MESSAGES_PAGE_SIZE, so the feed has to page back to old targets. */
const channelFeedPageSize = 20;

async function expectScrolledIntoFeed(feed: Locator, target: Locator) {
  await expect
    .poll(async () => {
      const [feedBox, targetBox] = await Promise.all([
        feed.boundingBox(),
        target.boundingBox(),
      ]);
      if (!feedBox || !targetBox) {
        return false;
      }
      return (
        targetBox.y >= feedBox.y &&
        targetBox.y + targetBox.height <= feedBox.y + feedBox.height
      );
    })
    .toBe(true);
}

type MessageResponse = { message: { id: string } };
type ForumPostResponse = { post: { id: string; rootMessageId: string } };
type PollResponse = { poll: { id: string } };

test.beforeAll(async ({ request }) => {
  await getOrCreateInstanceAdmin(request);
});

test('recovers realtime message notifications, opens the target, and persists read state', async ({
  context,
  page,
  request,
}) => {
  const recipient = await createAuthenticatedUser(
    request,
    context,
    createTestUser('notify-recipient'),
  );
  const actor = await signUpViaApi(request, createTestUser('notify-actor'));
  const server = await getDefaultServer(request, recipient);
  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  await expect(page.getByTestId('notification-bell')).toBeVisible();

  const firstBody = `Realtime notification ${recipient.user.suffix}`;
  const firstResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(actor),
      data: { body: firstBody },
    },
  );
  await expect(firstResponse).toBeOK();
  const firstMessage = ((await firstResponse.json()) as MessageResponse).message;
  await expectUnreadNotifications(page, 1);

  let inbox = await openNotifications(page);
  const firstItem = notificationItem(inbox, actor.user.name);
  await expect(firstItem).toContainText('#general');
  await firstItem.getByRole('button').first().click();
  await expect(page.locator(`[data-message-id="${firstMessage.id}"]`)).toBeVisible();
  await expectUnreadNotifications(page, 0);

  await page.reload();
  await expectUnreadNotifications(page, 0);

  await context.setOffline(true);
  const recoveredBody = `Recovered notification ${recipient.user.suffix}`;
  const recoveredResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(actor),
      data: { body: recoveredBody },
    },
  );
  await expect(recoveredResponse).toBeOK();
  await context.setOffline(false);
  await expectUnreadNotifications(page, 1);
  inbox = await openNotifications(page);
  await expect(
    inbox
      .locator('[data-testid="notification-item"][data-unread="true"]')
      .filter({ hasText: actor.user.name }),
  ).toBeVisible();
  const inboxBox = await inbox.boundingBox();
  const lastItemBox = await inbox.getByTestId('notification-item').last().boundingBox();
  expect(inboxBox).not.toBeNull();
  expect(lastItemBox).not.toBeNull();
  expect(
    inboxBox!.y + inboxBox!.height - (lastItemBox!.y + lastItemBox!.height),
  ).toBeLessThan(4);
});

test('thread and forum reply notifications open their exact conversations', async ({
  context,
  page,
  request,
}) => {
  const recipient = await createAuthenticatedUser(
    request,
    context,
    createTestUser('reply-recipient'),
  );
  const actor = await signUpViaApi(request, createTestUser('reply-actor'));
  const server = await getDefaultServer(request, recipient);
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  const forum = await createForumChannel(
    request,
    instanceAdmin,
    server.id,
    `notify-${recipient.user.suffix}`,
  );

  const rootResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(recipient),
      data: { body: `Thread root ${recipient.user.suffix}` },
    },
  );
  await expect(rootResponse).toBeOK();
  const root = ((await rootResponse.json()) as MessageResponse).message;

  const postResponse = await request.post(
    `/api/servers/${server.id}/channels/${forum.id}/forum/posts`,
    {
      headers: authorizationHeaders(recipient),
      data: {
        title: `Notification forum ${recipient.user.suffix}`,
        body: 'Forum notification root',
      },
    },
  );
  await expect(postResponse).toBeOK();
  const post = ((await postResponse.json()) as ForumPostResponse).post;

  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  const threadReplyResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages/${root.id}/replies`,
    {
      headers: authorizationHeaders(actor),
      data: { body: `Thread reply ${actor.user.suffix}` },
    },
  );
  await expect(threadReplyResponse).toBeOK();
  await expectUnreadNotifications(page, 1);

  let inbox = await openNotifications(page);
  await notificationItem(inbox, 'replied to a conversation')
    .getByRole('button')
    .first()
    .click();
  await expect(page).toHaveURL(new RegExp(`thread=${root.id}`));
  await expect(page.getByTestId('thread-panel')).toBeVisible();

  const forumReplyResponse = await request.post(
    `/api/servers/${server.id}/channels/${forum.id}/forum/posts/${post.id}/replies`,
    {
      headers: authorizationHeaders(actor),
      data: {
        body: `Forum reply ${actor.user.suffix}`,
        parentMessageId: post.rootMessageId,
      },
    },
  );
  await expect(forumReplyResponse).toBeOK();
  await expectUnreadNotifications(page, 1);
  inbox = await openNotifications(page);
  await notificationItem(inbox, 'replied to your forum post')
    .getByRole('button')
    .first()
    .click();
  await expect(page).toHaveURL(new RegExp(`/posts/${post.id}`));
  await expect(page.getByText(`Forum reply ${actor.user.suffix}`)).toBeVisible();
});

test('marks channels with unread activity in the channel list', async ({
  context,
  page,
  request,
}) => {
  const member = await createAuthenticatedUser(
    request,
    context,
    createTestUser('unread-member'),
  );
  const actor = await signUpViaApi(request, createTestUser('unread-actor'));
  const server = await getDefaultServer(request, member);
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  const forumName = `unread-${member.user.suffix}`;
  const forum = await createForumChannel(
    request,
    instanceAdmin,
    server.id,
    forumName,
  );

  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  await expect(channelListItem(page, 'general')).toBeVisible();
  await page.goto(`/s/${server.slug}/c/${forum.id}`);
  await expect(channelUnreadIndicator(page, 'general')).toHaveCount(0);

  const messageResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(actor),
      data: { body: `Unread channel message ${member.user.suffix}` },
    },
  );
  await expect(messageResponse).toBeOK();
  await expect(channelUnreadIndicator(page, 'general')).toBeVisible();

  await channelListItem(page, 'general').getByRole('link').first().click();
  await expect(page).toHaveURL(new RegExp(`/c/${server.generalChannelId}`));
  await expect(channelUnreadIndicator(page, 'general')).toHaveCount(0);
  await page.reload();
  await expect(channelUnreadIndicator(page, 'general')).toHaveCount(0);

  const postResponse = await request.post(
    `/api/servers/${server.id}/channels/${forum.id}/forum/posts`,
    {
      headers: authorizationHeaders(actor),
      data: {
        title: `Unread forum post ${member.user.suffix}`,
        body: 'Forum posts mark their channel unread too',
      },
    },
  );
  await expect(postResponse).toBeOK();
  const post = ((await postResponse.json()) as ForumPostResponse).post;
  await expect(channelUnreadIndicator(page, forumName)).toBeVisible();

  const inbox = await openNotifications(page);
  await notificationItem(inbox, 'created a new post')
    .getByRole('button')
    .first()
    .click();
  await expect(page).toHaveURL(new RegExp(`/posts/${post.id}`));
  await expect(channelUnreadIndicator(page, forumName)).toHaveCount(0);
});

test('proposal vote and ratification notifications open the proposal', async ({
  context,
  page,
  request,
}) => {
  const proposer = await createAuthenticatedUser(
    request,
    context,
    createTestUser('proposal-recipient'),
  );
  const voter = await signUpViaApi(request, createTestUser('proposal-voter'));
  const server = await getDefaultServer(request, proposer);
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  await makeProposalsRatifyWithOneAgreeVote(request, instanceAdmin, server.id);

  const proposalBody = `Notification proposal ${proposer.user.suffix}`;
  const proposalResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/polls`,
    {
      headers: authorizationHeaders(proposer),
      data: {
        body: proposalBody,
        pollType: 'proposal',
        action: { actionType: 'test' },
      },
    },
  );
  await expect(proposalResponse).toBeOK();
  const proposal = ((await proposalResponse.json()) as PollResponse).poll;

  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  await voteViaApi(
    request,
    voter,
    server.id,
    server.generalChannelId,
    proposal.id,
    'agree',
  );
  await expectUnreadNotifications(page, 2);

  const inbox = await openNotifications(page);
  await expect(notificationItem(inbox, 'voted agree')).toBeVisible();
  const outcome = notificationItem(inbox, 'was ratified');
  await expect(outcome).toBeVisible();
  await outcome.getByRole('button').first().click();
  await expect(
    page.getByTestId('feed').locator(`[data-decision-id="${proposal.id}"]`),
  ).toContainText(proposalBody);
});

test('notification targets flash where they appear in the feed', async ({
  context,
  page,
  request,
}) => {
  const recipient = await createAuthenticatedUser(
    request,
    context,
    createTestUser('flash-recipient'),
  );
  const actor = await signUpViaApi(request, createTestUser('flash-actor'));
  const server = await getDefaultServer(request, recipient);

  const messageResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(actor),
      data: { body: `Flashed message ${recipient.user.suffix}` },
    },
  );
  await expect(messageResponse).toBeOK();
  const message = ((await messageResponse.json()) as MessageResponse).message;

  const proposalBody = `Flashed proposal ${recipient.user.suffix}`;
  const proposalResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/polls`,
    {
      headers: authorizationHeaders(recipient),
      data: {
        body: proposalBody,
        pollType: 'proposal',
        action: { actionType: 'test' },
      },
    },
  );
  await expect(proposalResponse).toBeOK();
  const proposal = ((await proposalResponse.json()) as PollResponse).poll;
  await voteViaApi(
    request,
    actor,
    server.id,
    server.generalChannelId,
    proposal.id,
    'agree',
  );

  // Bury both targets so opening their notifications has to page back to them.
  await createMessages({
    request,
    user: recipient,
    serverId: server.id,
    channelId: server.generalChannelId,
    bodies: Array.from(
      { length: channelFeedPageSize * 2 },
      (_, index) => `Newer flash filler ${index + 1} ${recipient.user.suffix}`,
    ),
  });

  await page.setViewportSize({ width: 1440, height: 720 });
  await page.goto(`/s/${server.slug}/c/${server.generalChannelId}`);
  const feed = page.getByTestId('feed');
  await expect(feed).toBeVisible();

  let inbox = await openNotifications(page);
  await notificationItem(inbox, 'sent a new message')
    .getByRole('button')
    .first()
    .click();

  const focusedMessage = feed.locator(`[data-message-id="${message.id}"]`);
  await expect(focusedMessage).toHaveAttribute('data-focus-highlight', 'true');
  await expect(focusedMessage).toBeFocused();
  await expectScrolledIntoFeed(feed, focusedMessage);
  await expect(focusedMessage).not.toHaveAttribute(
    'data-focus-highlight',
    'true',
  );

  inbox = await openNotifications(page);
  await notificationItem(inbox, 'voted agree')
    .getByRole('button')
    .first()
    .click();

  const focusedProposal = feed.locator(`[data-decision-id="${proposal.id}"]`);
  await expect(focusedProposal).toHaveAttribute('data-focus-highlight', 'true');
  await expect(focusedProposal).toBeFocused();
  await expectScrolledIntoFeed(feed, focusedProposal);
  await expect(focusedProposal).not.toHaveAttribute(
    'data-focus-highlight',
    'true',
  );

  // A reply opens its thread panel and still places the proposal in the feed.
  const replyResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/polls/${proposal.id}/replies`,
    {
      headers: authorizationHeaders(actor),
      data: { body: `Flashed proposal reply ${actor.user.suffix}` },
    },
  );
  await expect(replyResponse).toBeOK();
  const reply = ((await replyResponse.json()) as MessageResponse).message;

  inbox = await openNotifications(page);
  await notificationItem(inbox, 'replied to a conversation')
    .getByRole('button')
    .first()
    .click();

  const threadPanel = page.getByTestId('thread-panel');
  await expect(threadPanel).toContainText(proposalBody);
  await expect(focusedProposal).toHaveAttribute('data-focus-highlight', 'true');
  await expectScrolledIntoFeed(feed, focusedProposal);

  // The reply itself is flashed in the panel, which older replies need since
  // the panel opens scrolled to its newest reply.
  await expect(
    threadPanel.locator(`[data-message-id="${reply.id}"]`),
  ).toHaveAttribute('data-focus-highlight', 'true');
});

test('selecting a notification from another page closes the inbox before routing', async ({
  context,
  page,
  request,
}) => {
  const recipient = await createAuthenticatedUser(
    request,
    context,
    createTestUser('inbox-close-recipient'),
  );
  const actor = await signUpViaApi(
    request,
    createTestUser('inbox-close-actor'),
  );
  const server = await getDefaultServer(request, recipient);

  const messageResponse = await request.post(
    `/api/servers/${server.id}/channels/${server.generalChannelId}/messages`,
    {
      headers: authorizationHeaders(actor),
      data: { body: `Inbox close message ${recipient.user.suffix}` },
    },
  );
  await expect(messageResponse).toBeOK();

  await page.goto('/users/settings');
  await page.getByTestId('notification-bell').click();
  const inbox = page.locator('section[aria-label="Notifications"]');
  await expect(inbox).toBeVisible();

  await page.evaluate(() => {
    const target = window as unknown as { __frames: string[] };
    target.__frames = [];
    const sample = () => {
      const open = Array.from(
        document.querySelectorAll('section[aria-label="Notifications"]'),
      ).filter(
        (element) => getComputedStyle(element).visibility !== 'hidden',
      ).length;
      target.__frames.push(`${location.pathname} inbox:${open}`);
      if (target.__frames.length < 120) {
        requestAnimationFrame(sample);
      }
    };
    requestAnimationFrame(sample);
  });

  await notificationItem(inbox, 'sent a new message')
    .getByRole('button')
    .first()
    .click();

  await page.waitForURL(/\/c\//);
  await expect(page.getByTestId('feed')).toBeVisible();
  await expect(inbox).toBeHidden();

  // The inbox must never be painted over the page it routed to.
  const frames = await page.evaluate(
    () => (window as unknown as { __frames: string[] }).__frames,
  );
  const overlapping = frames.filter(
    (frame) => !frame.startsWith('/users/settings') && frame.endsWith('inbox:1'),
  );
  expect(overlapping).toEqual([]);
});
