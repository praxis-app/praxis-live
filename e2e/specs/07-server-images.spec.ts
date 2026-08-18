import { expect, test } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createAuthenticatedUser, getOrCreateInstanceAdmin } from '../lib/auth';
import { createTestUser } from '../lib/data';
import { grantInstanceAdminRole } from '../lib/instance-roles';

const fixturePath = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../fixtures/valid-image.png',
);

const serverImageExists = (imageId: string) =>
  spawnSync(
    'docker',
    [
      'compose',
      '-f',
      'e2e/docker-compose.e2e.yml',
      'exec',
      '-T',
      'web',
      'test',
      '-f',
      `/tmp/praxis-live-e2e-uploads/server-images/${imageId}`,
    ],
    { stdio: 'pipe' },
  ).status === 0;

test('instance admin can set a server image when creating and replace it in settings', async ({
  context,
  page,
  request,
}) => {
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  const admin = await createAuthenticatedUser(
    request,
    context,
    createTestUser('server-image'),
  );
  await grantInstanceAdminRole(request, instanceAdmin, admin);
  const serverName = `Image server ${admin.user.suffix}`;

  await page.goto('/settings/servers');
  await expect(
    page.getByRole('heading', { name: 'Create Server' }),
  ).toBeVisible();

  await page.getByLabel('Name').fill(serverName);
  await page.getByLabel('Description').fill('Server image browser coverage.');
  await page.getByTestId('image-input').setInputFiles(fixturePath);

  const createResponsePromise = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().endsWith('/api/servers') &&
      response.status() === 200,
  );
  await page.getByRole('button', { name: 'Create', exact: true }).click();
  const createResponse = await createResponsePromise;
  const { server: createdServer } = (await createResponse.json()) as {
    server: { id: string; slug: string; image: { id: string } };
  };
  expect(createdServer.image.id).toBeTruthy();
  expect(serverImageExists(createdServer.image.id)).toBe(true);

  const createdServerLink = page.getByRole('link').filter({
    has: page.getByText(serverName, { exact: true }),
  });
  await expect(
    createdServerLink.getByRole('img', { name: serverName }),
  ).toBeVisible();

  const createdImageResponse = await request.get(
    `/api/servers/${createdServer.id}/images/${createdServer.image.id}`,
    { headers: { Authorization: `Bearer ${admin.accessToken}` } },
  );
  await expect(createdImageResponse).toBeOK();
  expect(createdImageResponse.headers()['content-type']).toBe('image/png');

  const serverBySlugResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/servers/slug/${createdServer.slug}`),
  );
  await page.goto(`/s/${createdServer.slug}/events`);
  expect((await serverBySlugResponse).status()).toBe(200);

  await page.goto(`/settings/servers/${createdServer.id}/edit`);
  await expect(page.getByText('Properties', { exact: true })).toBeVisible();
  const updatedDescription = 'Updated server identity cache coverage.';
  await page.getByLabel('Description').fill(updatedDescription);
  await page.getByTestId('image-input').setInputFiles(fixturePath);

  const updateResponsePromise = page.waitForResponse(
    (response) =>
      response.request().method() === 'PUT' &&
      response.url().endsWith(`/api/servers/${createdServer.id}`) &&
      response.status() === 200,
  );
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  const updateResponse = await updateResponsePromise;
  const { server: updatedServer } = (await updateResponse.json()) as {
    server: { image: { id: string } };
  };
  expect(updatedServer.image.id).not.toBe(createdServer.image.id);

  await expect(page.getByRole('img', { name: serverName })).toBeVisible();
  const updatedImageResponse = await request.get(
    `/api/servers/${createdServer.id}/images/${updatedServer.image.id}`,
    { headers: { Authorization: `Bearer ${admin.accessToken}` } },
  );
  await expect(updatedImageResponse).toBeOK();
  expect(updatedImageResponse.headers()['content-type']).toBe('image/png');
  const replacedImageResponse = await request.get(
    `/api/servers/${createdServer.id}/images/${createdServer.image.id}`,
    { headers: { Authorization: `Bearer ${admin.accessToken}` } },
  );
  expect(replacedImageResponse.status()).toBe(404);

  await page.goto(`/s/${createdServer.slug}/events`);
  await page
    .getByRole('button', { name: /praxis/i })
    .first()
    .click();
  await page.getByText(serverName, { exact: true }).click();
  await expect(
    page.getByText(updatedDescription, { exact: true }),
  ).toBeVisible();
  expect(serverImageExists(createdServer.image.id)).toBe(false);
  expect(serverImageExists(updatedServer.image.id)).toBe(true);

  const deleteResponse = await request.delete(
    `/api/servers/${createdServer.id}`,
    { headers: { Authorization: `Bearer ${admin.accessToken}` } },
  );
  await expect(deleteResponse).toBeOK();
  expect(serverImageExists(updatedServer.image.id)).toBe(false);
});
