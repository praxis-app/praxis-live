import {
  expect,
  test,
  type APIRequestContext,
  type Page,
} from '@playwright/test';
import {
  createAuthenticatedUser,
  type AuthenticatedUser,
} from '../lib/auth-api';
import { createTestUser } from '../lib/test-data';
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
  await page.getByRole('button', { name: 'Vote', exact: true }).click();
  await voteResponse;

  await expect(page.getByRole('button', { name: 'Remove vote' })).toBeVisible();
  await expect(page.getByText('1 vote').first()).toBeVisible();
});

async function openCreatePollDialog(page: Page) {
  await page
    .locator('form')
    .filter({ has: page.getByPlaceholder('Send a message...') })
    .getByRole('button')
    .first()
    .click();
  await page.getByRole('menuitem', { name: 'Create poll' }).click();
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
