import { expect, test } from '@playwright/test';
import {
  authorizationHeaders,
  createAuthenticatedUser,
  getOrCreateInstanceAdmin,
  signUpViaApi,
} from '../lib/auth';
import { createTestUser } from '../lib/data';
import { createForumChannel } from '../lib/forums';
import {
  expectUnreadNotifications,
  notificationItem,
  openNotifications,
} from '../lib/notifications';
import {
  makeProposalsRatifyWithOneAgreeVote,
  voteViaApi,
} from '../lib/polls';
import { getDefaultServer } from '../lib/servers';

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
  await expect(page).toHaveURL(
    new RegExp(`thread=${proposal.id}.*threadKind=poll`),
  );
  await expect(page.getByTestId('thread-panel').getByText(proposalBody)).toBeVisible();
});
