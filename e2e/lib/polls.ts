import {
  expect,
  type APIRequestContext,
  type Locator,
  type Page,
} from '@playwright/test';
import {
  authorizationHeaders,
  getOrCreateInstanceAdmin,
  type AuthenticatedUser,
} from './auth';
import { assertUuid, runDatabaseCommand } from './db';
import { grantInstanceAdminRole } from './instance-roles';

export async function makeProposalsRatifyWithOneAgreeVote(
  request: APIRequestContext,
  user: AuthenticatedUser,
  serverId: string,
) {
  // Config changes require the instance-level `Server:manage` fallback
  // rather than the server's own admin role, since other specs may have
  // edited that role's `ServerConfig` permission (e.g. via a ratified
  // role-change proposal).
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  await grantInstanceAdminRole(request, instanceAdmin, user);

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

export async function shortenNextPollDuration(
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

export function expirePollDeadline(pollId: string) {
  assertUuid(pollId, 'Poll ID');

  const output = runDatabaseCommand(
    `UPDATE poll_configs SET closing_at = now() - interval '1 second' WHERE poll_id = '${pollId}' AND EXISTS (SELECT 1 FROM polls WHERE id = '${pollId}' AND stage = 'voting');`,
  );

  expect(output).toContain('UPDATE 1');
}

export function getPollVoteSummary(pollId: string) {
  assertUuid(pollId, 'Poll ID');

  return runDatabaseCommand(
    `SELECT COUNT(*) || ':' || COALESCE(MIN(vote_type::text), 'none') FROM votes WHERE poll_id = '${pollId}';`,
    { tuplesOnly: true },
  ).trim();
}

export async function openCreatePollDialog(
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

export async function openCreateProposalDialog(page: Page) {
  await page
    .locator('form')
    .filter({ has: page.getByPlaceholder('Send a message...') })
    .getByRole('button')
    .first()
    .click();
  await page.getByRole('menuitem', { name: 'Create proposal' }).click();
}

export async function selectRadixOption(
  scope: Locator,
  page: Page,
  placeholder: string,
  optionName: string,
) {
  await scope.getByText(placeholder).click();
  await page.getByRole('option', { name: optionName }).click();
}

export function pollCard(page: Page, question: string) {
  return page
    .getByTestId('feed')
    .getByText(question)
    .locator('xpath=ancestor::div[contains(@class, "rounded-md")][1]');
}
