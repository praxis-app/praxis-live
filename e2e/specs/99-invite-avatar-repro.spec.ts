import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import {
  authorizationHeaders,
  createAuthenticatedUser,
  getOrCreateInstanceAdmin,
} from '../lib/auth';
import { createTestUser } from '../lib/data';
import { createInvite } from '../lib/invites';
import { SERVER_PERMISSIONS } from '../lib/permissions';
import { grantServerPermissions } from '../lib/server-roles';
import { getDefaultServer } from '../lib/servers';

/**
 * React Query refetches every active query when the tab becomes visible again.
 * Headless Chromium does not fire visibilitychange on bringToFront, so the
 * revisit is simulated by dispatching the event React Query listens for.
 */
const revisitTab = async (page: import('@playwright/test').Page) => {
  await page.evaluate(() =>
    window.dispatchEvent(new Event('visibilitychange')),
  );
};

test('invite avatars break for good after one transient image fetch failure', async ({
  page,
  context,
  request,
}) => {
  const label = 'invite-avatar';
  const instanceAdmin = await getOrCreateInstanceAdmin(request);
  const admin = await createAuthenticatedUser(
    request,
    context,
    createTestUser(label),
  );
  const server = await getDefaultServer(request, admin);
  await grantServerPermissions(
    request,
    instanceAdmin,
    admin,
    server.id,
    [SERVER_PERMISSIONS.createInvites],
    label,
  );

  const upload = await request.post('/api/users/profile-picture', {
    headers: authorizationHeaders(admin),
    multipart: {
      file: {
        name: 'valid-image.png',
        mimeType: 'image/png',
        buffer: await readFile('e2e/fixtures/valid-image.png'),
      },
    },
  });
  await expect(upload).toBeOK();

  for (let i = 0; i < 3; i++) {
    await createInvite(request, admin, server.id);
  }

  await page.goto(`/s/${server.slug}/settings/invites`);

  const avatars = page.getByRole('img', { name: admin.user.name });
  const fileMissing = page.getByText('Image file is missing.');

  await expect(avatars).toHaveCount(3);
  for (let i = 0; i < 3; i++) {
    await expect
      .poll(() =>
        avatars.nth(i).evaluate((el) => (el as HTMLImageElement).naturalWidth),
      )
      .toBeGreaterThan(0);
  }

  // One transient failure, as if the network blipped while the tab woke up.
  let failuresServed = 0;
  await page.route('**/api/users/*/images/*', async (route) => {
    if (failuresServed === 0) {
      failuresServed++;
      await route.abort('failed');
      return;
    }
    await route.continue();
  });

  await revisitTab(page);

  // Only the first row reports the failure, and every avatar loses its image.
  await expect(fileMissing).toHaveCount(1);
  await expect(avatars).toHaveCount(0);

  // The network is healthy again, so the other rows recover on the next revisit.
  await revisitTab(page);
  await revisitTab(page);

  // But the first row is stuck: `failed` is never reset once it is set.
  await expect(avatars).toHaveCount(2);
  await expect(fileMissing).toHaveCount(1);
  await page.screenshot({ path: 'test-results/invite-avatar-repro.png' });
});
