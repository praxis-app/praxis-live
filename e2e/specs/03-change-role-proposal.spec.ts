import {
  expect,
  test,
  type APIRequestContext,
  type Locator,
  type Page,
} from '@playwright/test';
import {
  createAuthenticatedUser,
  signUpViaApi,
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

type ServerRoleResponse = {
  serverRole: {
    id: string;
    name: string;
    color: string;
    permissions: { subject: string; action: string[] }[];
    members: { id: string; name: string; displayName?: string | null }[];
  };
};

type ServerRolesResponse = {
  serverRoles: ServerRoleResponse['serverRole'][];
};

const changedRoleColor = '#2196f3';

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

async function openCreateProposalDialog(page: Page) {
  await page
    .locator('form')
    .filter({ has: page.getByPlaceholder('Send a message...') })
    .getByRole('button')
    .first()
    .click();
  await page.getByRole('menuitem', { name: 'Create proposal' }).click();
}

async function selectRadixOption(
  scope: Locator,
  page: Page,
  placeholder: string,
  optionName: string,
) {
  await scope.getByText(placeholder).click();
  await page.getByRole('option', { name: optionName }).click();
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

async function makeProposalsRatifyWithOneAgreeVote(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
) {
  const response = await request.put(`/api/servers/${serverId}/configs`, {
    headers: authorizationHeaders(user),
    data: {
      decisionMakingModel: 'majority-vote',
      agreementThreshold: 51,
      quorumEnabled: false,
      votingTimeLimit: 0,
    },
  });

  await expect(response).toBeOK();
}

async function getAdminRole(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
) {
  const response = await request.get(`/api/servers/${serverId}/roles`, {
    headers: authorizationHeaders(user),
  });

  await expect(response).toBeOK();
  const body = (await response.json()) as ServerRolesResponse;
  const adminRole = body.serverRoles.find((role) => role.name === 'admin');
  expect(adminRole).toBeTruthy();
  return adminRole!;
}

async function getServerRole(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
  serverRoleId: string,
) {
  const response = await request.get(
    `/api/servers/${serverId}/roles/${serverRoleId}`,
    {
      headers: authorizationHeaders(user),
    },
  );

  await expect(response).toBeOK();
  return ((await response.json()) as ServerRoleResponse).serverRole;
}

function authorizationHeaders(user: AuthenticatedUser) {
  return {
    Authorization: `Bearer ${user.accessToken}`,
  };
}
