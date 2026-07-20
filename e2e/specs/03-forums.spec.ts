import { expect, test } from '@playwright/test';
import { createAuthenticatedUser } from '../lib/auth';
import { createTestUser } from '../lib/data';
import { createForumChannel } from '../lib/forums';
import {
  makeProposalsRatifyWithOneAgreeVote,
  openCreateProposalDialog,
  selectRadixOption,
} from '../lib/polls';
import { getDefaultServer } from '../lib/servers';
import { ChatPage } from '../pages/chat.page';

type ForumPostResponse = {
  post: {
    id: string;
  };
};

test('user can move a text proposal to a forum, reply, vote, and see it ratified', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const proposer = await createAuthenticatedUser(
    request,
    context,
    createTestUser('forum-move'),
  );
  const server = await getDefaultServer(request, proposer);
  const forumChannelName = `forum-${proposer.user.suffix}`;
  const forumChannel = await createForumChannel(
    request,
    proposer,
    server.id,
    forumChannelName,
  );
  await makeProposalsRatifyWithOneAgreeVote(request, proposer, server.id);

  const proposalBody = `Move proposal ${proposer.user.suffix}`;
  const reply = `Moved proposal reply ${proposer.user.suffix}`;
  const chat = new ChatPage(page);

  await chat.goto();
  await chat.expectChannel('general');
  await openCreateProposalDialog(page);

  const createProposalDialog = page.getByRole('dialog', {
    name: 'Create a New Proposal',
  });
  await selectRadixOption(
    createProposalDialog,
    page,
    'Select an action type',
    'Test',
  );
  await createProposalDialog
    .getByPlaceholder('Enter your proposal details...')
    .fill(proposalBody);
  await createProposalDialog.getByRole('button', { name: 'Next' }).click();

  const createProposalResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200,
  );
  await createProposalDialog
    .getByRole('button', { name: 'Create proposal' })
    .click();
  await createProposalResponse;
  await expect(createProposalDialog).toBeHidden();

  const textProposal = page.getByRole('article', {
    name: `Majority Vote Proposal: ${proposalBody}`,
  });
  await expect(textProposal).toBeVisible();
  await textProposal
    .getByRole('button', { name: 'Open proposal menu' })
    .click();
  await page.getByRole('menuitem', { name: 'Move to forum' }).click();

  const moveDialog = page.getByRole('dialog', { name: 'Move to forum' });
  await moveDialog.getByRole('combobox').click();
  await page.getByRole('option', { name: forumChannelName }).click();

  const moveProposalResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes('/move-to-forum') &&
      response.status() === 200,
  );
  await moveDialog.getByRole('button', { name: 'Move proposal' }).click();
  const movedProposalResponse = await moveProposalResponse;
  const { post } = (await movedProposalResponse.json()) as ForumPostResponse;

  await expect(page).toHaveURL(
    new RegExp(`/c/${forumChannel.id}/posts/${post.id}$`),
  );
  await expect(
    page.getByRole('article').getByRole('heading', { name: proposalBody }),
  ).toBeVisible();

  const forumProposal = page.getByRole('region', { name: 'Proposal' });
  await expect(forumProposal.getByText(proposalBody)).toBeVisible();

  const replyResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/forum/posts/${post.id}/replies`) &&
      response.status() === 200,
  );
  await page.getByPlaceholder('Send a message...').fill(reply);
  await page.getByPlaceholder('Send a message...').press('Enter');
  await replyResponse;
  await expect(page.getByText(reply)).toBeVisible();

  const voteResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes('/votes') &&
      response.status() === 200,
  );
  await forumProposal
    .getByRole('button', { name: 'Agree', exact: true })
    .click();
  await voteResponse;
  await expect(
    forumProposal.getByText('Ratified', { exact: true }),
  ).toBeVisible();
});

test('user can turn a forum discussion into a ratified proposal', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const proposer = await createAuthenticatedUser(
    request,
    context,
    createTestUser('forum-discussion'),
  );
  const server = await getDefaultServer(request, proposer);
  const forumChannelName = `forum-${proposer.user.suffix}`;
  const forumChannel = await createForumChannel(
    request,
    proposer,
    server.id,
    forumChannelName,
  );
  await makeProposalsRatifyWithOneAgreeVote(request, proposer, server.id);

  const postTitle = `Forum discussion ${proposer.user.suffix}`;
  const postBody = `Opening message ${proposer.user.suffix}`;
  const reply = `Discussion reply ${proposer.user.suffix}`;
  const proposalBody = `Discussion proposal ${proposer.user.suffix}`;

  await page.goto(`/s/${server.slug}/c/${forumChannel.id}`);
  const newPostButton = page.getByRole('button', { name: 'New post' });
  await expect(newPostButton).toBeVisible();
  await newPostButton.click();

  const createPostDialog = page.getByRole('dialog', { name: 'Create post' });
  await createPostDialog.getByLabel('Title').fill(postTitle);
  await createPostDialog.getByLabel('Message').fill(postBody);

  const createPostResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/channels/${forumChannel.id}/forum/posts`) &&
      response.status() === 200,
  );
  await createPostDialog
    .getByRole('button', { name: 'Create discussion' })
    .click();
  const createdPostResponse = await createPostResponse;
  const { post } = (await createdPostResponse.json()) as ForumPostResponse;

  await expect(page).toHaveURL(
    new RegExp(`/c/${forumChannel.id}/posts/${post.id}$`),
  );
  await expect(
    page.getByRole('article').getByRole('heading', { name: postTitle }),
  ).toBeVisible();
  await expect(page.getByText(postBody)).toBeVisible();

  const replyResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/forum/posts/${post.id}/replies`) &&
      response.status() === 200,
  );
  await page.getByPlaceholder('Send a message...').fill(reply);
  await page.getByPlaceholder('Send a message...').press('Enter');
  await replyResponse;
  await expect(page.getByText(reply)).toBeVisible();

  await page.getByRole('button', { name: 'Open post menu' }).click();
  await page
    .getByRole('menuitem', { name: 'Create proposal from discussion' })
    .click();

  const createProposalDialog = page.getByRole('dialog', {
    name: 'Create proposal from discussion',
  });
  await selectRadixOption(
    createProposalDialog,
    page,
    'Select an action type',
    'Test',
  );
  await createProposalDialog
    .getByPlaceholder('Enter your proposal details...')
    .fill(proposalBody);
  await createProposalDialog.getByRole('button', { name: 'Next' }).click();

  const createProposalResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/forum/posts/${post.id}/proposal`) &&
      response.status() === 200,
  );
  await createProposalDialog
    .getByRole('button', { name: 'Create proposal' })
    .click();
  await createProposalResponse;
  await expect(createProposalDialog).toBeHidden();

  const forumProposal = page.getByRole('region', { name: 'Proposal' });
  await expect(forumProposal.getByText(proposalBody)).toBeVisible();

  const voteResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes('/votes') &&
      response.status() === 200,
  );
  await forumProposal
    .getByRole('button', { name: 'Agree', exact: true })
    .click();
  await voteResponse;
  await expect(
    forumProposal.getByText('Ratified', { exact: true }),
  ).toBeVisible();
});
