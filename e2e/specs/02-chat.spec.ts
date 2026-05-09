import { expect, test, type Locator } from '@playwright/test';
import { createAuthenticatedUser, setupAnonymousInvite } from '../lib/auth';
import { createTestMessage, createTestUser } from '../lib/data';
import { getDefaultServer } from '../lib/servers';
import { ChatPage } from '../pages/chat.page';
import { NavigationPage } from '../pages/navigation.page';

type PollResponse = {
  poll: {
    id: string;
  };
};

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

  await openMessageFormMenu(callChatPanel);
  await page.getByRole('menuitem', { name: 'Create proposal' }).click();

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
      response.url().includes(`/channels/${server.generalChannelId}/calls/`) &&
      response.url().includes('/polls') &&
      response.status() === 200,
  );
  await proposalDialog.getByRole('button', { name: 'Create proposal' }).click();
  const createResponse = await createProposalResponse;
  const { poll } = (await createResponse.json()) as PollResponse;

  await expect(proposalDialog).toBeHidden();

  const proposal = callChatPanel.getByRole('article', {
    name: `Consensus proposal: ${proposalBody}`,
  });
  await expect(proposal).toBeVisible();

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

  await chat.goto();
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

async function openMessageFormMenu(scope: Locator) {
  await expect(scope.getByPlaceholder('Send a message...')).toBeVisible();
  await scope.getByRole('button', { name: 'Open message actions' }).click();
}

async function backgroundColor(locator: Locator) {
  return locator.evaluate(
    (element) => window.getComputedStyle(element).backgroundColor,
  );
}
