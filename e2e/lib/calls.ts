import { expect, type Locator, type Page } from '@playwright/test';
import { execFileSync } from 'node:child_process';

const confirmPreJoin = async (page: Page) => {
  await expect(
    page.getByRole('heading', { name: 'Check your setup' }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Join now' }).click();
};

export const startCallFromTopNav = async (page: Page) => {
  await page.getByRole('button', { name: 'Call' }).click();
  await confirmPreJoin(page);
};

export const joinCallFromArtifact = async (
  page: Page,
  callArtifact: Locator,
) => {
  await callArtifact.getByRole('button', { name: 'Join active video' }).click();
  await confirmPreJoin(page);
};

export const ageActiveCallForStaleCleanup = (callId: string) => {
  expect(callId).toMatch(
    /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
  );

  const output = execFileSync(
    'docker',
    [
      'compose',
      '-f',
      'docker-compose.e2e.yml',
      'exec',
      '-T',
      'database',
      'psql',
      '-U',
      'postgres',
      '-d',
      'postgres',
      '-v',
      'ON_ERROR_STOP=1',
      '-c',
      `UPDATE calls SET updated_at = now() - interval '25 hours' WHERE id = '${callId}' AND status = 'active';`,
    ],
    {
      cwd: process.cwd(),
      encoding: 'utf8',
    },
  );

  expect(output).toContain('UPDATE 1');
};
