import {
  expect,
  test,
  type APIRequestContext,
  type Page,
} from '@playwright/test';
import {
  createAuthenticatedUser,
  setupAnonymousSession,
  type AuthenticatedUser,
} from '../lib/auth';
import { createTestUser } from '../lib/data';
import { ChatPage } from '../pages/chat.page';

type ServerResponse = {
  server: {
    id: string;
    slug: string;
    generalChannelId: string;
  };
};

type PollResponse = {
  poll: {
    id: string;
    body?: string | null;
    pollType: string;
    config: {
      closingAt?: string | null;
      multipleChoice?: boolean | null;
    };
    options: {
      id: string;
      text: string;
      voteCount: number;
    }[];
  };
};

test('authenticated user can create and vote in a poll', async ({
  context,
  page,
  request,
}) => {
  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser('poll'),
  );
  const server = await getDefaultServer(request, authenticatedUser);
  const question = `Where should we meet ${authenticatedUser.user.suffix}?`;
  const options = [
    `Library ${authenticatedUser.user.suffix}`,
    `Cafe ${authenticatedUser.user.suffix}`,
    `Park ${authenticatedUser.user.suffix}`,
  ];

  const chat = new ChatPage(page);
  await chat.goto();
  await chat.expectChannel('general');

  await openCreatePollDialog(page);
  const dialog = page.getByRole('dialog', { name: 'Create a Poll' });

  await dialog
    .getByPlaceholder('What question do you want to ask?')
    .fill(question);
  await dialog.getByPlaceholder('Answer 1').fill(options[0]);
  await dialog.getByPlaceholder('Answer 2').fill(options[1]);
  await dialog.getByRole('button', { name: 'Add another answer' }).click();
  await dialog.getByPlaceholder('Answer 3').fill(options[2]);
  await dialog.getByRole('combobox', { name: 'Duration' }).click();
  await page.getByRole('option', { name: '30 minutes' }).click();

  const createPollResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200,
  );
  await dialog.getByRole('button', { name: 'Create poll' }).click();
  const response = await createPollResponse;
  const { poll } = (await response.json()) as PollResponse;

  expect(poll.body).toBe(question);
  expect(poll.pollType).toBe('poll');
  expect(poll.options.map((option) => option.text)).toEqual(options);
  expect(poll.config.multipleChoice).toBe(false);
  expect(poll.config.closingAt).toBeTruthy();
  expect(minutesUntil(poll.config.closingAt!)).toBeGreaterThanOrEqual(29);
  expect(minutesUntil(poll.config.closingAt!)).toBeLessThanOrEqual(30);

  await expect(dialog).toBeHidden();
  await expect(page.getByText(question)).toBeVisible();
  for (const option of options) {
    await expect(page.getByText(option)).toBeVisible();
  }
  await expect(page.getByText('Ends in 30 minutes')).toBeVisible();

  const voteResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/polls/${poll.id}/votes`) &&
      response.status() === 200,
  );
  await page.getByRole('button', { name: options[1] }).click();
  await page.getByRole('button', { name: 'Vote', exact: true }).first().click();
  await voteResponse;

  await expect(page.getByRole('button', { name: 'Remove vote' })).toBeVisible();
  await expect(page.getByText('1 vote').first()).toBeVisible();
});

test('authenticated user sees a poll close after its closing time passes', async ({
  context,
  page,
  request,
}) => {
  const authenticatedUser = await createAuthenticatedUser(
    request,
    context,
    createTestUser('poll-close'),
  );
  const server = await getDefaultServer(request, authenticatedUser);
  const question = `Which snack wins ${authenticatedUser.user.suffix}?`;
  const options = [
    `Pretzels ${authenticatedUser.user.suffix}`,
    `Popcorn ${authenticatedUser.user.suffix}`,
  ];

  const chat = new ChatPage(page);
  await chat.goto();
  await chat.expectChannel('general');

  await openCreatePollDialog(page);
  const dialog = page.getByRole('dialog', { name: 'Create a Poll' });

  await dialog
    .getByPlaceholder('What question do you want to ask?')
    .fill(question);
  await dialog.getByPlaceholder('Answer 1').fill(options[0]);
  await dialog.getByPlaceholder('Answer 2').fill(options[1]);
  await dialog.getByRole('combobox', { name: 'Duration' }).click();
  await page.getByRole('option', { name: '30 minutes' }).click();
  await shortenNextPollDuration(page, server.generalChannelId, 5);

  const createPollResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200,
  );
  await dialog.getByRole('button', { name: 'Create poll' }).click();
  const response = await createPollResponse;
  const { poll } = (await response.json()) as PollResponse;

  expect(poll.config.closingAt).toBeTruthy();
  expect(secondsUntil(poll.config.closingAt!)).toBeLessThanOrEqual(6);

  await expect(dialog).toBeHidden();
  await expect(page.getByText(question).first()).toBeVisible();

  const voteResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/polls/${poll.id}/votes`) &&
      response.status() === 200,
  );
  await page.getByRole('button', { name: options[1] }).click();
  await page.getByRole('button', { name: 'Vote', exact: true }).first().click();
  await voteResponse;

  await expect(page.getByRole('button', { name: 'Remove vote' })).toBeVisible();
  await expect(page.getByText('1 vote').first()).toBeVisible();

  await expect
    .poll(
      async () => {
        await page.reload();
        await expect(page.getByText(question).first()).toBeVisible();
        return pollCard(page, question).getByText('Poll closed').isVisible();
      },
      { timeout: 20_000 },
    )
    .toBe(true);

  const closedPoll = pollCard(page, question);
  await expect(closedPoll.getByText('Poll closed')).toBeVisible();
  await expect(
    closedPoll.getByRole('button', { name: 'Vote', exact: true }),
  ).toHaveCount(0);
  await expect(
    closedPoll.getByRole('button', { name: 'Remove vote' }),
  ).toHaveCount(0);
  await expect(closedPoll.getByText('1 vote').first()).toBeVisible();
});

test('anonymous user can create only allowed chat polls', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(60_000);

  const { admin, server } = await setupAnonymousSession(
    request,
    context,
    'anon-poll',
  );
  const pollQuestion = `Anonymous poll ${admin.user.suffix}?`;
  const proposalBody = `Anonymous test proposal ${admin.user.suffix}`;
  const chat = new ChatPage(page);

  await chat.goto();
  await chat.expectChannel('general');

  await openCreatePollDialog(page, 'Create poll');
  const pollDialog = page.getByRole('dialog', { name: 'Create a Poll' });
  await pollDialog
    .getByPlaceholder('What question do you want to ask?')
    .fill(pollQuestion);
  await pollDialog.getByPlaceholder('Answer 1').fill('Yes');
  await pollDialog.getByPlaceholder('Answer 2').fill('No');

  const pollResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200,
  );
  await pollDialog.getByRole('button', { name: 'Create poll' }).click();
  await pollResponse;
  await expect(pollDialog).toBeHidden();
  await expect(page.getByText(pollQuestion)).toBeVisible();

  await openCreatePollDialog(page, 'Create proposal');
  const proposalDialog = page.getByRole('dialog', {
    name: 'Create a New Proposal',
  });
  await proposalDialog.getByRole('combobox').click();
  await page.getByRole('option', { name: 'General decision' }).click();
  await expect(
    proposalDialog.getByText(
      'Anonymous users can only create test proposals. Please register to create other proposal types.',
    ),
  ).toBeVisible();

  await proposalDialog.getByRole('combobox').click();
  await page.getByRole('option', { name: 'Test' }).click();
  await proposalDialog
    .getByPlaceholder('Enter your proposal details...')
    .fill(proposalBody);
  await proposalDialog.getByRole('button', { name: 'Next' }).click();

  const proposalResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200,
  );
  await proposalDialog.getByRole('button', { name: 'Create proposal' }).click();
  await proposalResponse;
  await expect(proposalDialog).toBeHidden();
  await expect(page.getByText(proposalBody)).toBeVisible();
});

async function shortenNextPollDuration(
  page: Page,
  channelId: string,
  seconds: number,
) {
  await page.route(
    `**/channels/${channelId}/polls`,
    async (route) => {
      const request = route.request();
      if (request.method() !== 'POST') {
        await route.continue();
        return;
      }

      const payload = JSON.parse(request.postData() ?? '{}') as {
        closingAt?: string;
      };
      payload.closingAt = new Date(Date.now() + seconds * 1000).toISOString();

      await route.continue({ postData: JSON.stringify(payload) });
    },
    { times: 1 },
  );
}

async function openCreatePollDialog(
  page: Page,
  menuItemName: 'Create poll' | 'Create proposal' = 'Create poll',
) {
  await page
    .locator('form')
    .filter({ has: page.getByPlaceholder('Send a message...') })
    .getByRole('button')
    .first()
    .click();
  await page.getByRole('menuitem', { name: menuItemName }).click();
}

function pollCard(page: Page, question: string) {
  return page
    .getByText(question)
    .locator('xpath=ancestor::div[contains(@class, "rounded-md")][1]');
}

async function getDefaultServer(
  request: APIRequestContext,
  user: AuthenticatedUser,
) {
  const response = await request.get('/api/servers/default', {
    headers: authorizationHeaders(user),
  });

  await expect(response).toBeOK();
  return ((await response.json()) as ServerResponse).server;
}

function authorizationHeaders(user: AuthenticatedUser) {
  return {
    Authorization: `Bearer ${user.accessToken}`,
  };
}

function minutesUntil(isoTimestamp: string) {
  return Math.round((new Date(isoTimestamp).getTime() - Date.now()) / 60_000);
}

function secondsUntil(isoTimestamp: string) {
  return Math.round((new Date(isoTimestamp).getTime() - Date.now()) / 1000);
}
