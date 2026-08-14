import { expect, test } from '@playwright/test';
import {
  authorizationHeaders,
  setupAnonymousSession,
  signUpViaApi,
} from '../lib/auth';
import { createTestUser, INSTANCE_ADMIN_USER } from '../lib/data';
import { createInvite } from '../lib/invites';
import { AuthPage } from '../pages/auth.page';
import { ChatPage } from '../pages/chat.page';
import { NavigationPage } from '../pages/navigation.page';

test('user can sign up from the landing page', async ({ page }) => {
  const user = INSTANCE_ADMIN_USER;
  const auth = new AuthPage(page);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await auth.gotoLanding();
  await auth.followSignupLink();
  await auth.signUp(user);

  await auth.expectSignedUp();
  await chat.expectChannel('general');
  await navigation.expectSignedInUser(user);
  await navigation.expectAccessTokenPersisted();
});

test('anonymous user can register from the landing page', async ({
  context,
  page,
  request,
}) => {
  await setupAnonymousSession(request, context, 'anon-upgrade');
  const user = createTestUser('anon-upgrade');
  const auth = new AuthPage(page);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await auth.gotoLanding();
  await auth.followSignupLink();

  const upgradeResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'PUT' &&
      response.url().endsWith('/api/auth/anon') &&
      response.status() === 204,
  );
  await auth.signUp(user);
  await upgradeResponse;

  await auth.expectSignedUp();
  await chat.expectChannel('general');
  await navigation.expectSignedInUser(user);
  await navigation.expectAccessTokenPersisted();
});

test('invited user can log in and join the invited server', async ({
  page,
  request,
}) => {
  const admin = await signUpViaApi(request, createTestUser('invite-admin'));
  const serverName = `Invite server ${admin.user.suffix}`;
  const serverSlug = `invite-${admin.user.suffix}`;
  const createServerResponse = await request.post('/api/servers', {
    headers: authorizationHeaders(admin),
    data: {
      name: serverName,
      slug: serverSlug,
      description: 'Server for the invite login flow.',
      isDefaultServer: false,
    },
  });
  await expect(createServerResponse).toBeOK();
  const { server } = (await createServerResponse.json()) as {
    server: { id: string; slug: string };
  };
  const inviteToken = await createInvite(request, admin, server.id);
  const invitedUser = createTestUser('invite-member');
  await signUpViaApi(request, invitedUser);

  const auth = new AuthPage(page);
  const chat = new ChatPage(page);

  await page.goto(`/i/${inviteToken}`);
  await expect(page).toHaveURL('/about');
  await expect(
    page.getByRole('link', { name: 'Accept invite', exact: true }).first(),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Log in', exact: true }).click();
  await auth.logIn(invitedUser);

  await expect(page).toHaveURL(`/i/${inviteToken}/join`);
  await expect(
    page.getByRole('heading', { name: "You've been invited" }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: serverName, exact: true }),
  ).toBeVisible();

  const joinResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().endsWith(`/api/servers/${server.id}/join`) &&
      response.status() === 200,
  );
  await page.getByRole('button', { name: 'Accept Invite' }).click();
  await joinResponse;

  await expect(page).toHaveURL(new RegExp(`/s/${server.slug}/c/[^/]+/?$`));
  await chat.expectChannel('general');
  await expect
    .poll(() =>
      page.evaluate(() => window.localStorage.getItem('invite-token')),
    )
    .toBeNull();
});

test('invited user can sign up and join the invited server', async ({
  page,
  request,
}) => {
  const admin = await signUpViaApi(
    request,
    createTestUser('signup-invite-admin'),
  );
  const serverName = `Signup invite server ${admin.user.suffix}`;
  const serverSlug = `signup-invite-${admin.user.suffix}`;
  const createServerResponse = await request.post('/api/servers', {
    headers: authorizationHeaders(admin),
    data: {
      name: serverName,
      slug: serverSlug,
      description: 'Server for the invite signup flow.',
      isDefaultServer: false,
    },
  });
  await expect(createServerResponse).toBeOK();
  const { server } = (await createServerResponse.json()) as {
    server: { id: string; slug: string };
  };
  const inviteToken = await createInvite(request, admin, server.id);
  const invitedUser = createTestUser('signup-invite-member');
  const auth = new AuthPage(page);
  const chat = new ChatPage(page);
  const navigation = new NavigationPage(page);

  await page.goto(`/i/${inviteToken}`);
  await expect(page).toHaveURL('/about');
  await page
    .getByRole('link', { name: 'Accept invite', exact: true })
    .first()
    .click();
  await expect(page).toHaveURL(`/auth/signup/${inviteToken}`);

  const signupResponse = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().endsWith('/api/auth/signup') &&
      response.status() === 201 &&
      response.request().postDataJSON().inviteToken === inviteToken,
  );
  await auth.signUp(invitedUser);
  await signupResponse;

  await expect(page).toHaveURL(new RegExp(`/s/${server.slug}/c/[^/]+/?$`));
  await chat.expectChannel('general');
  await navigation.expectSignedInUser(invitedUser);
  await navigation.expectAccessTokenPersisted();
  await expect
    .poll(() =>
      page.evaluate(() => window.localStorage.getItem('invite-token')),
    )
    .toBeNull();
});
