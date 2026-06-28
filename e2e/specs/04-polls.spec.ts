import { expect, test } from '@playwright/test';
import {
  createAuthenticatedUser,
  setupAnonymousSession,
  signUpViaApi,
} from '../lib/auth';
import { createTestUser } from '../lib/data';
import {
  makeProposalsRatifyWithOneAgreeVote,
  openCreatePollDialog,
  openCreateProposalDialog,
  pollCard,
  selectRadixOption,
  shortenNextPollDuration,
} from '../lib/polls';
import { getAdminRole, getDefaultServer, getServerRole } from '../lib/servers';
import { minutesUntil, secondsUntil } from '../lib/time';
import { ChatPage } from '../pages/chat.page';

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

const changedRoleColor = '#2196f3';

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

test('user can create and ratify a proposal to change a role', async ({
  context,
  page,
  request,
}) => {
  const proposer = await createAuthenticatedUser(
    request,
    context,
    createTestUser('role-proposal'),
  );
  const addedMember = await signUpViaApi(
    request,
    createTestUser('role-member'),
  );

  const server = await getDefaultServer(request, proposer);
  await makeProposalsRatifyWithOneAgreeVote(request, proposer, server.id);

  const adminRole = await getAdminRole(request, proposer, server.id);
  const changedRoleName = `admin-${proposer.user.suffix}`;
  const proposalBody = `Change the admin role ${proposer.user.suffix}`;

  const chat = new ChatPage(page);
  await chat.goto();
  await chat.expectChannel('general');

  await openCreateProposalDialog(page);
  const dialog = page.getByRole('dialog', { name: 'Create a New Proposal' });

  await selectRadixOption(dialog, page, 'Select an action type', 'Change role');
  await dialog
    .getByPlaceholder('Enter your proposal details...')
    .fill(proposalBody);
  await dialog.getByRole('button', { name: 'Next' }).click();

  await selectRadixOption(dialog, page, 'Select a role...', 'admin');
  await dialog.getByRole('button', { name: 'Next' }).click();

  await dialog.getByPlaceholder('Name').fill(changedRoleName);
  await dialog.getByRole('button', { name: /Role color/ }).click();
  await dialog
    .getByRole('button', { name: `Pick ${changedRoleColor}` })
    .click();
  await dialog.getByRole('button', { name: 'Next' }).click();

  await dialog.getByRole('switch', { name: 'Manage settings' }).click();
  await dialog.getByRole('button', { name: 'Next' }).click();

  await dialog
    .getByPlaceholder('Search members...')
    .fill(addedMember.user.name);
  await dialog.getByText(addedMember.user.name).click();
  await dialog.getByRole('button', { name: 'Next' }).click();

  await expect(dialog.getByText(changedRoleName)).toBeVisible();
  await expect(dialog.getByText(changedRoleColor)).toBeVisible();
  await expect(dialog.getByText('Manage settings')).toBeVisible();
  await expect(dialog.getByText(addedMember.user.name)).toBeVisible();

  const createProposalResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/channels/${server.generalChannelId}/polls`) &&
      response.status() === 200,
  );
  await dialog.getByRole('button', { name: 'Create proposal' }).click();
  await createProposalResponse;

  await expect(dialog).toBeHidden();
  const proposal = page.getByRole('article', {
    name: `Consensus Proposal: ${proposalBody}`,
  });
  await expect(proposal).toBeVisible();
  await expect(proposal.getByText('Voting', { exact: true })).toBeVisible();

  const voteResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes('/votes') &&
      response.status() === 200,
  );
  await proposal.getByRole('button', { name: 'Agree', exact: true }).click();
  await voteResponse;

  await expect(proposal.getByText('Ratified', { exact: true })).toBeVisible();

  const changedRole = await getServerRole(
    request,
    proposer,
    server.id,
    adminRole.id,
  );

  expect(changedRole.name).toBe(changedRoleName);
  expect(changedRole.color).toBe(changedRoleColor);
  expect(
    changedRole.permissions.some(
      (permission) =>
        permission.subject === 'ServerConfig' &&
        permission.action.includes('manage'),
    ),
  ).toBe(false);
  expect(changedRole.members.map((member) => member.id)).toContain(
    addedMember.userId,
  );
  expect(changedRole.members.map((member) => member.id)).toEqual(
    expect.arrayContaining(adminRole.members.map((member) => member.id)),
  );
});
